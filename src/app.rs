use std::path::PathBuf;
use std::time::Instant;

use egui::{Color32, Context, Pos2, Ui};

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
    pub drag_start: Option<Pos2>,
    pub session_path: Option<PathBuf>,
    pub status_msg: Option<StatusMessage>,
    pub status_time: Option<Instant>,
    pub status_duration: std::time::Duration,
    pub selected_graph: Option<usize>,
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
            drag_start: None,
            session_path: None,
            status_msg: None,
            status_time: None,
            status_duration: std::time::Duration::from_secs(3),
            selected_graph: Some(0),
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
                    g.data = Some(PlotData::compute(f, self.x_min, self.x_max, self.n_points));
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
}

impl eframe::App for PlotApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // --- Горячие клавиши ---
        let input = ctx.input(|i| i.clone());
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

        // --- Левая панель управления ---
        egui::SidePanel::left("controls").show(ctx, |ui| {
            crate::ui::controls::show_controls_panel(self, ctx, ui);
        });

        // --- Центральная область: холст для графиков ---
        egui::CentralPanel::default().show(ctx, |ui: &mut Ui| {
            crate::ui::canvas::show_canvas_panel(self, ctx, ui);
        });
    }
}
