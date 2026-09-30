use std::path::PathBuf;
use std::time::Instant;

use egui::{Color32, Pos2, Rect};

use crate::config;
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
    pub adaptive: bool,          // Использовать адаптивный алгоритм
    pub adaptive_tolerance: f64, // Порог для адаптивного алгоритма
    pub drag_start: Option<Pos2>,
    pub session_path: Option<PathBuf>,
    pub status_msg: Option<StatusMessage>,
    pub status_time: Option<Instant>,
    pub status_duration: std::time::Duration,
    pub selected_graph: Option<usize>,
    /// Путь к последней сохранённой/загруженной сессии.
    pub last_session_path: Option<PathBuf>,
    /// Флаг: нужно предложить сохранить сессию при выходе.
    pub pending_save_on_exit: bool,
    /// Флаг: показать окно помощи.
    pub show_help_window: bool,
    /// Флаг: показать окно "О программе".
    pub show_about_window: bool,
    /// Флаг: показать диалог сохранения при выходе.
    pub show_save_dialog: bool,
    /// Флаг: готово к закрытию (пользователь нажал "Да").
    pub is_ready_to_close: bool,
    /// Флаг: есть несохранённые изменения.
    pub has_unsaved_changes: bool,
    /// Флаг: отменить закрытие (ожидание действия пользователя).
    pub cancel_close: bool,
    /// Последний размер canvas для отслеживания ресайза
    pub _last_canvas_rect: Option<Rect>,
    /// Размер окна для сохранения
    pub window_size: [f32; 2],
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
            last_session_path: None,
            pending_save_on_exit: false,
            show_help_window: false,
            show_about_window: false,
            show_save_dialog: false,
            is_ready_to_close: false,
            has_unsaved_changes: false,
            cancel_close: false,
            _last_canvas_rect: None,
            window_size: [1000.0, 700.0],
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
                                    let is_dup =
                                        intersections.iter().any(|p: &IntersectionPoint| {
                                            (p.x - ix).abs()
                                                < 0.01 * (self.viewport.x_max - self.viewport.x_min)
                                        });
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

    /// Сохранить размер окна в config.
    pub fn save_window_size(&mut self) {
        let config = config::AppConfig::load();
        let new_config = config::AppConfig {
            window_width: self.window_size[0] as f64,
            window_height: self.window_size[1] as f64,
            ..config
        };
        let _ = new_config.save();
    }

    /// Восстановить размер окна из config.
    pub fn restore_window_size(&mut self) {
        let config = config::AppConfig::load();
        self.window_size = [config.window_width as f32, config.window_height as f32];
    }

    /// Предложить сохранить сессию при выходе.
    /// Возвращает путь, если пользователь выбрал файл для сохранения.
    pub fn prompt_save_on_exit(&mut self) -> Option<PathBuf> {
        // Предлагаем сохранить только если сессия никогда не была явно сохранена
        if self.session_path.is_some() {
            return None;
        }

        let path = rfd::FileDialog::new()
            .set_file_name("plot_session.json")
            .add_filter("JSON", &["json"])
            .save_file();

        if let Some(ref p) = path {
            let session = crate::session::SessionData::from_app(self);
            match session.save_to_file(p) {
                Ok(()) => {
                    self.session_path = Some(p.clone());
                    self.last_session_path = Some(p.clone());
                    self.pending_save_on_exit = false;
                    return Some(p.clone());
                }
                Err(e) => {
                    self.status_msg = Some(StatusMessage::Error(format!("Ошибка сохранения: {e}")));
                    self.status_time = Some(Instant::now());
                }
            }
        }

        self.pending_save_on_exit = false;
        None
    }

    /// Загрузить последнюю сессию, если она существует.
    pub fn try_load_last_session(&mut self) {
        let path = self.last_session_path.clone();
        if let Some(ref path) = path {
            if path.exists() {
                match crate::session::SessionData::load_from_file(path) {
                    Ok(session_data) => {
                        session_data.apply_to_app(self);
                        self.session_path = Some(path.clone());
                        self.status_msg =
                            Some(StatusMessage::Info(format!("Автозагрузка: {}", path.display())));
                        self.status_time = Some(Instant::now());
                    }
                    Err(e) => {
                        self.status_msg = Some(StatusMessage::Error(format!("Ошибка автозагрузки: {e}")));
                        self.status_time = Some(Instant::now());
                    }
                }
            }
        }
    }
}

impl eframe::App for PlotApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        
        // --- Запрос на сохранение сессии при выходе ---
        // 1. Проверяем, нажал ли пользователь на крестик окна
        if ctx.input(|i| i.viewport().close_requested()) {
            // Показываем диалог только если есть несохранённые изменения
            if !self.is_ready_to_close && self.has_unsaved_changes {
                // Отменяем автоматическое закрытие приложения
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                // Показываем свое диалоговое окно подтверждения
                self.show_save_dialog = true;
            }
        }

        // 2. Отрисовываем диалог подтверждения
        if self.show_save_dialog {
            let mut save_file = false;
            egui::Window::new("Сохранить сессию?")
                .resizable(false)
                .collapsible(false)
                .show(&ctx, |ui| {
                    ui.label("Вы хотите сохранить сессию перед выходом?");
                    ui.horizontal(|ui| {
                        if ui.button("Сохранить").clicked() {
                            save_file = true;
                        }
                        if ui.button("Не сохранять").clicked() {
                            self.pending_save_on_exit = false;
                            self.show_save_dialog = false;
                        }
                        if ui.button("Отмена").clicked() {
                            self.pending_save_on_exit = false;
                            self.show_save_dialog = false;
                        }
                    });
                });

            if save_file {
                // 3. Если пользователь нажал "Сохранить" — показываем диалог сохранения
                let path = rfd::FileDialog::new()
                    .set_file_name("plot_session.json")
                    .add_filter("JSON", &["json"])
                    .save_file();

                if let Some(p) = path {
                    let session = crate::session::SessionData::from_app(self);
                    if session.save_to_file(&p).is_ok() {
                        self.session_path = Some(p.clone());
                        self.last_session_path = Some(p.clone());
                        self.has_unsaved_changes = false;
                        let mut config = config::AppConfig::load();
                        config.last_session_path = Some(p.to_string_lossy().to_string());
                        let _ = config.save();
                        self.status_msg = Some(StatusMessage::Info("Сессия сохранена".to_string()));
                        self.status_time = Some(Instant::now());
                    }
                }
                self.show_save_dialog = false;
                self.pending_save_on_exit = false;
                // Готовимся к закрытию
                self.is_ready_to_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }

            // Не показываем основной интерфейс
            return;
        }

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
            self.has_unsaved_changes = true;
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
                        self.last_session_path = Some(p.clone());
                        self.pending_save_on_exit = false;
                        self.has_unsaved_changes = false;
                        // Сохраняем путь в config
                        let mut config = config::AppConfig::load();
                        config.last_session_path = Some(p.to_string_lossy().to_string());
                        let _ = config.save();
                        self.status_msg =
                            Some(StatusMessage::Info(format!("Загружено: {}", p.display())));
                        self.status_time = Some(Instant::now());
                    }
                    Err(e) => {
                        self.status_msg = Some(StatusMessage::Error(format!("Ошибка: {e}")));
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
                    self.has_unsaved_changes = true;
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
        if input.key_pressed(egui::Key::F5) || (input.key_pressed(egui::Key::Enter) && ctrl) {
            self.recompute_all();
        }

        // Автоматический пересчёт dirty графиков
        self.recompute_all();

        // --- Левая панель управления ---
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

        // --- Окна помощи и "О программе" ---
        egui::Window::new("Помощь")
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .resizable(true)
            .open(&mut self.show_help_window)
            .show(&ctx, |ui| {
                crate::ui::controls::render_help_content(ui);
            });

        egui::Window::new("О программе")
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .resizable(false)
            .open(&mut self.show_about_window)
            .show(&ctx, |ui| {
                crate::ui::controls::render_about_content(ui);
            });
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;
    use crate::config::AppConfig;

    fn create_temp_config(width: f64, height: f64, suffix: &str) -> PathBuf {
        let temp_dir = std::env::temp_dir().join("function-plotter-test");
        fs::create_dir_all(&temp_dir).unwrap();
        let config_path = temp_dir.join(format!("config{}.json", suffix));
        
        let config = AppConfig {
            version: 2,
            window_width: width,
            window_height: height,
            auto_load_last_session: false,
            last_session_path: None,
        };
        config.save_to(&config_path).unwrap();
        config_path
    }

    #[test]
    fn test_window_size_default() {
        let app = PlotApp::default();
        
        // По умолчанию размер должен быть 1000x700
        assert_eq!(app.window_size[0], 1000.0);
        assert_eq!(app.window_size[1], 700.0);
    }

    #[test]
    fn test_window_size_from_config() {
        let config_path = create_temp_config(1200.0, 800.0, "_from_config");
        
        // Загружаем config и проверяем, что значения правильные
        let config = AppConfig::load_from(&config_path);
        assert_eq!(config.window_width, 1200.0);
        assert_eq!(config.window_height, 800.0);
        
        // Чистим
        let temp_dir = config_path.parent().unwrap().to_path_buf();
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_save_and_restore_window_size() {
        let config_path = create_temp_config(1280.0, 720.0, "_save_restore");
        
        // Создаём config с нужным размером
        let config = AppConfig::load_from(&config_path);
        let new_config = AppConfig {
            window_width: 1280.0,
            window_height: 720.0,
            ..config
        };
        new_config.save_to(&config_path).unwrap();
        
        // Восстанавливаем размер из config
        let restored_config = AppConfig::load_from(&config_path);
        let window_size = [restored_config.window_width as f32, restored_config.window_height as f32];
        
        assert_eq!(window_size[0], 1280.0);
        assert_eq!(window_size[1], 720.0);
        
        // Чистим
        let temp_dir = config_path.parent().unwrap().to_path_buf();
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
