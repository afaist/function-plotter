use std::path::PathBuf;
use std::time::Instant;

use egui::{Color32, Pos2, Rect};

use crate::evaluator::PlotData;
use crate::parser::{self, ParsedFormula};
use crate::renderer::{PlotStyle, Viewport};
use crate::session;

/// Сообщение в статус-баре с автоматическим затуханием.
#[derive(Clone, Debug)]
pub enum StatusMessage {
    Info(String),
    Error(String),
}

impl StatusMessage {
    pub fn text(&self) -> &str {
        match self {
            Self::Info(s) => s,
            Self::Error(s) => s,
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            Self::Info(_) => Color32::from_rgb(100, 255, 100),
            Self::Error(_) => Color32::from_rgb(255, 100, 100),
        }
    }
}

/// Точка пересечения двух графиков.
#[derive(Clone, Debug)]
pub struct IntersectionPoint {
    pub x: f64,
    pub y: f64,
}

/// Один график: формула, стиль, вычисленные точки.
pub struct GraphEntry {
    pub formula_text: String,
    pub parsed: Option<ParsedFormula>,
    pub parse_error: Option<String>,
    pub style: PlotStyle,
    pub data: Option<PlotData>,
    /// Нужно ли пересчитать точки (изменилась формула/диапазон).
    pub dirty: bool,
}

impl GraphEntry {
    pub fn new(label: &str, color: Color32, formula: &str) -> Self {
        let mut entry = Self {
            formula_text: formula.to_string(),
            parsed: None,
            parse_error: None,
            style: PlotStyle {
                color,
                label: label.to_string(),
                visible: true,
            },
            data: None,
            dirty: true,
        };
        entry.reparse();
        entry
    }

    pub fn reparse(&mut self) {
        match parser::parse(&self.formula_text) {
            Ok(p) => {
                self.parsed = Some(p);
                self.parse_error = None;
                self.dirty = true; // формула изменилась — нужно пересчитать
            }
            Err(e) => {
                self.parsed = None;
                self.parse_error = Some(e);
                self.dirty = true;
            }
        }
    }

    /// Пометить график как требующий пересчёта.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

/// Основное состояние приложения.
pub struct PlotApp {
    pub graphs: Vec<GraphEntry>,
    pub x_min: f64,
    pub x_max: f64,
    pub n_points: usize,
    pub viewport: Viewport,
    pub auto_y: bool,
    pub adaptive: bool, // Использовать адаптивный алгоритм
    pub adaptive_tolerance: f64, // Порог для адаптивного алгоритма
    pub drag_start: Option<Pos2>,
    pub session_path: Option<PathBuf>,
    pub status_msg: Option<StatusMessage>,
    pub status_time: Option<Instant>,
    pub status_duration: std::time::Duration,
    pub selected_graph: Option<usize>,
    /// Последний размер canvas для отслеживания ресайза
    #[allow(dead_code)]
    pub _last_canvas_rect: Option<Rect>,
}

impl Default for PlotApp {
    fn default() -> Self {
        let mut app = Self {
            graphs: Vec::new(),
            x_min: -10.0,
            x_max: 10.0,
            n_points: 500,
            viewport: Viewport::new(),
            auto_y: true,
            adaptive: false,
            adaptive_tolerance: 0.01,
            drag_start: None,
            session_path: None,
            status_msg: None,
            status_time: None,
            status_duration: std::time::Duration::from_secs(3),
            selected_graph: Some(0),
            _last_canvas_rect: None,
        };
        app.graphs.push(GraphEntry::new(
            "f1",
            Color32::from_rgb(100, 200, 255),
            "sin(x)",
        ));
        app.graphs.push(GraphEntry::new(
            "f2",
            Color32::from_rgb(255, 150, 100),
            "x^2 / 10",
        ));
        app.recompute_all();
        app
    }
}

impl PlotApp {
    /// Пересчитать ТОЛЬКО изменённые (dirty) графики.
    pub fn recompute_all(&mut self) {
        let mut any_dirty = false;
        for g in &mut self.graphs {
            if g.dirty {
                if let Some(ref f) = g.parsed {
                    g.data = if self.adaptive {
                        Some(PlotData::compute_adaptive(
                            f,
                            self.x_min,
                            self.x_max,
                            10, // max refinements
                            self.adaptive_tolerance,
                        ))
                    } else {
                        Some(PlotData::compute(f, self.x_min, self.x_max, self.n_points))
                    };
                } else {
                    g.data = None;
                }
                g.dirty = false;
                any_dirty = true;
            }
        }
        // Если что-то пересчитали — нужно обновить Y-масштаб
        if any_dirty && self.auto_y {
            self.update_viewport_y();
        }
    }

    /// Пометить ВСЕ графики как требующие пересчёта (при изменении viewport/параметров).
    pub fn mark_all_dirty(&mut self) {
        for g in &mut self.graphs {
            g.dirty = true;
        }
    }

    fn update_viewport_y(&mut self) {
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for g in &self.graphs {
            if let Some(ref d) = g.data {
                if g.style.visible {
                    y_min = y_min.min(d.y_min);
                    y_max = y_max.max(d.y_max);
                }
            }
        }
        if y_min.is_finite() && y_max.is_finite() {
            let pad = (y_max - y_min).max(1.0) * 0.1;
            self.viewport.y_min = y_min - pad;
            self.viewport.y_max = y_max + pad;
        }
    }

    /// Найти точки пересечения видимых графиков.
    pub fn find_intersections(&self) -> Vec<IntersectionPoint> {
        let mut intersections = Vec::new();
        let visible: Vec<&GraphEntry> = self
            .graphs
            .iter()
            .filter(|g| g.style.visible && g.data.is_some())
            .collect();

        for i in 0..visible.len() {
            for j in (i + 1)..visible.len() {
                let data_a = visible[i].data.as_ref().unwrap();
                let data_b = visible[j].data.as_ref().unwrap();

                // Ищем пересечения по общей сетке X
                let min_len = data_a.points.len().min(data_b.points.len());
                for k in 1..min_len {
                    let (xa_a, ya_a) = data_a.points[k - 1];
                    let (_xa_b, yb_a) = data_b.points[k - 1];
                    let (xa_c, ya_c) = data_a.points[k];
                    let (_xa_d, yb_c) = data_b.points[k];

                    // Проверяем sign change: (ya - yb) меняет знак
                    let diff_prev = ya_a - yb_a;
                    let diff_curr = ya_c - yb_c;

                    if diff_prev * diff_curr < 0.0 {
                        // Линейная интерполяция для точности
                        let dx = xa_c - xa_a;
                        if dx.abs() > 1e-15 {
                            let t = diff_prev / (diff_prev - diff_curr);
                            let ix = xa_a + t * dx;
                            // Интерполируем Y для обоих графиков
                            let slope_a = (ya_c - ya_a) / dx;
                            let slope_b = (yb_c - yb_a) / dx;
                            let iy_a = ya_a + slope_a * (ix - xa_a);
                            let iy_b = yb_a + slope_b * (ix - xa_a);
                            let iy = (iy_a + iy_b) / 2.0;

                            if ix.is_finite() && iy.is_finite() {
                                // Проверяем что точка в viewport
                                if ix >= self.viewport.x_min
                                    && ix <= self.viewport.x_max
                                    && iy >= self.viewport.y_min
                                    && iy <= self.viewport.y_max
                                {
                                    // Проверяем дубликаты
                                    let is_dup = intersections
                                        .iter()
                                        .any(|p: &IntersectionPoint| (p.x - ix).abs() < 0.01 * (self.viewport.x_max - self.viewport.x_min));
                                    if !is_dup {
                                        intersections.push(IntersectionPoint { x: ix, y: iy });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        intersections
    }
}

impl eframe::App for PlotApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // --- Горячие клавиши ---
        let input = ui.input(|i| i.clone());
        let ctrl = input.modifiers.ctrl || input.modifiers.command;

        // Ctrl+N — новый график
        if input.key_pressed(egui::Key::N) && ctrl {
            let palette = [
                Color32::from_rgb(150, 255, 150),
                Color32::from_rgb(255, 255, 100),
                Color32::from_rgb(200, 100, 255),
                Color32::from_rgb(100, 255, 200),
            ];
            let idx = self.graphs.len();
            let color = palette[idx % palette.len()];
            self.graphs
                .push(GraphEntry::new(&format!("f{}", idx + 1), color, "x"));
            self.selected_graph = Some(self.graphs.len() - 1);
            self.recompute_all();
        }

        // Ctrl+O — загрузить сессию
        if input.key_pressed(egui::Key::O) && ctrl {
            let path = rfd::FileDialog::new()
                .add_filter("JSON", &["json"])
                .pick_file();
            if let Some(ref p) = path {
                match session::SessionData::load_from_file(p) {
                    Ok(session_data) => {
                        session_data.apply_to_app(self);
                        self.session_path = Some(p.clone());
                        self.status_msg = Some(StatusMessage::Info(format!(
                            "Загружено: {}",
                            p.display()
                        )));
                        self.status_time = Some(Instant::now());
                    }
                    Err(e) => {
                        self.status_msg = Some(StatusMessage::Error(format!(
                            "Ошибка: {e}"
                        )));
                        self.status_time = Some(Instant::now());
                    }
                }
            }
        }

        // Delete — удалить выбранный график
        if input.key_pressed(egui::Key::Delete) && self.graphs.len() > 1 {
            if let Some(idx) = self.selected_graph {
                if idx < self.graphs.len() {
                    self.graphs.remove(idx);
                    // Корректировка selected_graph
                    if idx >= self.graphs.len() {
                        self.selected_graph = Some(self.graphs.len() - 1);
                    } else {
                        self.selected_graph = Some(idx);
                    }
                    self.recompute_all();
                }
            }
        }

        // R — сбросить масштаб
        if input.key_pressed(egui::Key::R) && !ctrl {
            self.viewport = Viewport::new();
            self.x_min = self.viewport.x_min;
            self.x_max = self.viewport.x_max;
            self.auto_y = true;
            self.recompute_all();
        }

        // F5 / Ctrl+Enter — принудительная перерасчёт
        if input.key_pressed(egui::Key::F5)
            || (input.key_pressed(egui::Key::Enter) && ctrl)
        {
            self.recompute_all();
        }

        // Автоматический пересчёт dirty графиков
        self.recompute_all();

        // --- Левая панель управления ---
        let ctx = ui.ctx().clone();
        egui::Panel::left("controls")
            .resizable(true)
            .default_size(260.0)
            .show(ui, |ui| {
                crate::ui::controls::show_controls_panel(self, &ctx, ui);
            });

        // --- Центральная область: холст для графиков ---
        egui::CentralPanel::default().show(ui, |ui| {
            crate::ui::canvas::show_canvas_panel(self, &ctx, ui);
        });
    }
}
