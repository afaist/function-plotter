use std::path::PathBuf;
use std::time::Instant;

use egui::{Color32, Pos2, Rect};

use crate::config;
use crate::evaluator::PlotData;
use crate::export::save_rect_to_png;
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
#[derive(Clone)]
pub struct GraphEntry {
    pub formula_text: String,
    pub parsed: Option<ParsedFormula>,
    pub parse_error: Option<String>,
    pub style: PlotStyle,
    pub data: Option<PlotData>,
    /// Нужно ли пересчитать точки (изменилась формула/диапазон).
    pub dirty: bool,
    /// Сырые данные из CSV (x, y) — если график загружен из файла.
    pub raw_data: Option<Vec<(f64, f64)>>,
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
            raw_data: None,
        };
        entry.reparse();
        entry
    }

    /// Создать график из сырых данных (импорт из CSV).
    pub fn from_raw_data(label: &str, color: Color32, points: Vec<(f64, f64)>) -> Self {
        let mut entry = Self {
            formula_text: format!("data ({})", points.len()),
            parsed: None,
            parse_error: None,
            style: PlotStyle {
                color,
                label: label.to_string(),
                visible: true,
            },
            data: None,
            dirty: false,
            raw_data: Some(points),
        };
        entry.recompute_from_raw();
        entry
    }

    /// Пересчитать PlotData из raw_data.
    fn recompute_from_raw(&mut self) {
        if let Some(ref points) = self.raw_data {
            if points.is_empty() {
                self.data = None;
                return;
            }
            let mut y_min = f64::INFINITY;
            let mut y_max = f64::NEG_INFINITY;
            for &(_, y) in points {
                if y.is_finite() {
                    y_min = y_min.min(y);
                    y_max = y_max.max(y);
                }
            }
            if !y_min.is_finite() || !y_max.is_finite() {
                y_min = -1.0;
                y_max = 1.0;
            }
            self.data = Some(PlotData {
                points: points.clone(),
                y_min,
                y_max,
                fill_points: Vec::new(),
            });
        }
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

    /// Получить информацию о типе формулы для отображения в UI.
    pub fn formula_type_info(&self) -> Option<(egui::Color32, &str, &str)> {
        let parsed = self.parsed.as_ref()?;
        let (color, icon, label) = match parsed.formula_type() {
            parser::FormulaType::Regular => (Color32::from_rgb(150, 150, 150), "●", "Обычная"),
            parser::FormulaType::Derivative => (Color32::from_rgb(255, 170, 50), "∂", "Производная"),
            parser::FormulaType::Integral => (Color32::from_rgb(100, 200, 255), "∫", "Интеграл"),
            parser::FormulaType::Polar => (Color32::from_rgb(100, 255, 100), "◎", "Полярная"),
            parser::FormulaType::Parametric => (Color32::from_rgb(255, 130, 200), "⟳", "Параметрическая"),
        };
        Some((color, icon, label))
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
    pub polar_mode: bool,        // Режим полярных координат
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
    /// Флаг: показать окно шаблонов.
    pub show_templates_window: bool,
    /// Флаг: показать таблицу значений.
    pub show_values_table: bool,
    /// Индекс графика для отображения таблицы.
    pub values_table_graph_index: Option<usize>,
    /// Количество шагов для таблицы значений.
    pub values_table_steps: usize,
    /// Флаг: показать диалог сохранения при выходе.
    pub show_save_dialog: bool,
    /// Флаг: готово к закрытию (пользователь нажал "Да").
    pub is_ready_to_close: bool,
    /// Флаг: есть несохранённые изменения.
    pub has_unsaved_changes: bool,
    /// Флаг: отменить закрытие (ожидание действия пользователя).
    pub cancel_close: bool,
    /// Стек для undo: снимки состояния graphs
    pub undo_stack: Vec<Vec<GraphEntry>>,
    /// Стек для redo: снимки состояния graphs
    pub redo_stack: Vec<Vec<GraphEntry>>,
    /// Максимальный размер истории
    pub max_history: usize,
    /// Последний размер canvas для отслеживания ресайза
    pub _last_canvas_rect: Option<Rect>,
    /// Размер окна для сохранения
    pub window_size: [f32; 2],
    /// Флаг: сделать скриншот
    pub should_capture: bool,
    /// Координаты холста для обрезки скриншота
    pub graph_rect: Option<Rect>,
    /// Путь для сохранения скриншота
    pub save_path: Option<PathBuf>,
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
            polar_mode: false,
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
            show_templates_window: false,
            show_values_table: false,
            values_table_graph_index: None,
            values_table_steps: 10,
            show_save_dialog: false,
            is_ready_to_close: false,
            has_unsaved_changes: false,
            cancel_close: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: 50,
            _last_canvas_rect: None,
            window_size: [1000.0, 700.0],
            should_capture: false,
            graph_rect: None,
            save_path: None,
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

    /// Сохранить текущее состояние graphs в undo_stack.
    pub fn save_snapshot(&mut self) {
        self.undo_stack.push(self.graphs.clone());
        self.redo_stack.clear(); // новое действие — очищаем redo
        // Ограничить размер истории
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
    }

    /// Отменить последнее действие. Возвращает true если было что отменять.
    pub fn undo(&mut self) -> bool {
        if self.undo_stack.is_empty() {
            return false;
        }
        // Сохраняем текущее состояние в redo
        self.redo_stack.push(self.graphs.clone());
        // Восстанавливаем предыдущее
        self.graphs = self.undo_stack.pop().unwrap();
        self.mark_all_dirty();
        true
    }

    /// Повторить отменённое действие. Возвращает true если было что повторять.
    pub fn redo(&mut self) -> bool {
        if self.redo_stack.is_empty() {
            return false;
        }
        // Сохраняем текущее состояние в undo
        self.undo_stack.push(self.graphs.clone());
        // Восстанавливаем следующее
        self.graphs = self.redo_stack.pop().unwrap();
        self.mark_all_dirty();
        true
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

    /// Сгенерировать таблицу значений для указанного графика.
    pub fn generate_values_table(&self, graph_index: usize, steps: usize) -> Option<Vec<(f64, f64)>> {
        let graph = self.graphs.get(graph_index)?;
        let data = graph.data.as_ref()?;
        
        let n = steps.max(2).min(1000);
        let x_range = self.viewport.x_max - self.viewport.x_min;
        let x_step = x_range / (n - 1) as f64;
        let x_start = self.viewport.x_min;
        
        let mut table = Vec::with_capacity(n);
        for i in 0..n {
            let x = x_start + i as f64 * x_step;
            let y = data.points.get(i).map(|p| p.1).unwrap_or(f64::NAN);
            table.push((x, y));
        }
        
        Some(table)
    }

    /// Отрисовка окна таблицы значений.
    #[allow(dead_code)]
    fn render_values_table_ui(&mut self, ui: &mut egui::Ui) {
        // Эта функция оставлена для обратной совместимости
        let _ = ui;
    }
}

/// Данные для таблицы значений.
#[derive(Clone, Debug)]
struct ValuesTableData {
    graph_label: String,
    formula: String,
    points: Vec<(f64, f64)>,
}

/// Отрисовка окна таблицы значений (standalone функция).
fn render_values_table_ui(
    ui: &mut egui::Ui,
    table_steps: &mut usize,
    x_min: f64,
    x_max: f64,
    table_data: Option<&ValuesTableData>,
    graph_label: &Option<String>,
) {
    ui.heading("Таблица значений");
    ui.add_space(8.0);

    if let Some(data) = table_data {
        // Количество шагов
        let mut steps = *table_steps;
        ui.horizontal(|ui| {
            ui.label("Шагов:");
            if ui.add(egui::Slider::new(&mut steps, 5..=100).text("количество")).changed() {
                *table_steps = steps;
            }
        });

        // Таблица
        let n = steps.max(2).min(1000);
        let x_range = x_max - x_min;
        let x_step = x_range / (n - 1) as f64;

        ui.add_space(8.0);
        ui.label(format!("График: {}  |  Формула: {}", data.graph_label, data.formula));
        ui.separator();

        // Используем Grid для таблицы
        egui::Grid::new("values_grid")
            .striped(true)
            .spacing([40.0, 8.0])
            .show(ui, |ui| {
                ui.strong("X");
                ui.strong("Y");
                ui.end_row();

                for i in 0..n {
                    let x = x_min + i as f64 * x_step;
                    let y = data.points.get(i).map(|p| p.1).unwrap_or(f64::NAN);
                    
                    ui.label(format!("{:.4}", x));
                    if y.is_nan() {
                        ui.label("—");
                    } else {
                        ui.label(format!("{:.4}", y));
                    }
                    ui.end_row();
                }
            });
    } else {
        ui.label("Выберите график для отображения таблицы");
        if let Some(ref label) = graph_label {
            ui.label(format!("Выбран: {}", label));
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

        // 2. Отрисовываем диалог подтверждения поверх основного интерфейса
        if self.show_save_dialog {
            let mut save_file = false;
            let mut close_without_save = false;
            let mut cancel = false;
            
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
                            close_without_save = true;
                        }
                        if ui.button("Отмена").clicked() {
                            cancel = true;
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
            } else if close_without_save {
                self.show_save_dialog = false;
                self.pending_save_on_exit = false;
                // Готовимся к закрытию — на следующем кадре close_requested() не покажет диалог
                self.is_ready_to_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else if cancel {
                // Отмена — просто закрываем диалог, приложение остаётся открытым
                self.show_save_dialog = false;
            }

            // Не показываем основной интерфейс
            return;
        }

        // --- Горячие клавиши ---
        let input = ui.input(|i| i.clone());
        let ctrl = input.modifiers.ctrl || input.modifiers.command;

        // Ctrl+Z — отмена
        if input.key_pressed(egui::Key::Z) && ctrl && !input.modifiers.shift {
            if self.undo() {
                self.status_msg = Some(StatusMessage::Info("Отменено".to_string()));
                self.status_time = Some(Instant::now());
            }
        }

        // Ctrl+Shift+Z или Ctrl+Y — повтор
        if (input.key_pressed(egui::Key::Z) && input.modifiers.shift && ctrl)
            || (input.key_pressed(egui::Key::Y) && ctrl)
        {
            if self.redo() {
                self.status_msg = Some(StatusMessage::Info("Повторено".to_string()));
                self.status_time = Some(Instant::now());
            }
        }

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
            self.save_snapshot();
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
                    self.save_snapshot();
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

        // Окно таблицы значений
        egui::Window::new("Таблица значений")
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .resizable(true)
            .open(&mut self.show_values_table)
            .show(&ctx, |ui| {
                // Извлекаем данные для таблицы, чтобы избежать borrow conflicts
                let table_data = self.values_table_graph_index.and_then(|idx| {
                    if idx < self.graphs.len() {
                        let graph = &self.graphs[idx];
                        graph.data.as_ref().map(|data| ValuesTableData {
                            graph_label: graph.style.label.clone(),
                            formula: graph.formula_text.clone(),
                            points: data.points.clone(),
                        })
                    } else {
                        None
                    }
                });

                let x_min = self.viewport.x_min;
                let x_max = self.viewport.x_max;
                let graph_label = self.values_table_graph_index.and_then(|idx| {
                    if idx < self.graphs.len() {
                        Some(self.graphs[idx].style.label.clone())
                    } else {
                        None
                    }
                });

                render_values_table_ui(ui, &mut self.values_table_steps, x_min, x_max, table_data.as_ref(), &graph_label);
            });

        // --- Обработка скриншота ---
        if self.should_capture {
            let screenshot = ctx.input(|i| {
                i.raw.events.iter().find_map(|event| {
                    if let egui::Event::Screenshot { image, .. } = event {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            });
            if let Some(screenshot) = screenshot {
                self.should_capture = false;
                
                if let (Some(rect), Some(ref save_path)) = (self.graph_rect, &self.save_path) {
                    if let Err(e) = save_rect_to_png(&screenshot, rect, save_path) {
                        self.status_msg = Some(StatusMessage::Error(format!("Ошибка PNG: {e}")));
                        self.status_time = Some(Instant::now());
                    }
                }
                self.save_path = None;
            }
        }
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
        if let Err(e) = fs::create_dir_all(&temp_dir) {
            eprintln!("Warning: Could not create temp dir: {e}");
        }
        let config_path = temp_dir.join(format!("config{}.json", suffix));
        
        let config = AppConfig {
            version: 2,
            window_width: width,
            window_height: height,
            auto_load_last_session: false,
            last_session_path: None,
        };
        if let Err(e) = config.save_to(&config_path) {
            eprintln!("Warning: Could not save config: {e}");
        }
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
        let temp_dir = std::env::temp_dir().join("function-plotter-test-parametric");
        let _ = fs::create_dir_all(&temp_dir);
        let config_path = temp_dir.join("config_test.json");
        
        // Создаём config с нужным размером
        let config = AppConfig {
            version: 2,
            window_width: 1200.0,
            window_height: 800.0,
            auto_load_last_session: false,
            last_session_path: None,
        };
        config.save_to(&config_path).ok();
        
        // Загружаем config и проверяем, что значения правильные
        let loaded = AppConfig::load_from(&config_path);
        assert_eq!(loaded.window_width, 1200.0, "window_width should be 1200.0");
        assert_eq!(loaded.window_height, 800.0, "window_height should be 800.0");
        
        // Чистим
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

    // --- Undo/Redo тесты ---

    #[test]
    fn test_undo_redo_basic() {
        let mut app = PlotApp::default();
        let initial_count = app.graphs.len();
        
        // Добавляем снимок и новый график
        app.save_snapshot();
        app.graphs.push(GraphEntry::new("f_test", Color32::RED, "x^2"));
        assert_eq!(app.graphs.len(), initial_count + 1);
        
        // Undo — должен вернуть начальное состояние
        assert!(app.undo());
        assert_eq!(app.graphs.len(), initial_count);
        
        // Redo — должен вернуть добавленный график
        assert!(app.redo());
        assert_eq!(app.graphs.len(), initial_count + 1);
    }

    #[test]
    fn test_undo_redo_multiple() {
        let mut app = PlotApp::default();
        let initial_count = app.graphs.len();
        
        // 3 действия
        app.save_snapshot();
        app.graphs.push(GraphEntry::new("f1", Color32::RED, "x"));
        
        app.save_snapshot();
        app.graphs.push(GraphEntry::new("f2", Color32::BLUE, "x^2"));
        
        app.save_snapshot();
        app.graphs.push(GraphEntry::new("f3", Color32::GREEN, "x^3"));
        
        assert_eq!(app.graphs.len(), initial_count + 3);
        
        // Undo 2 раза
        assert!(app.undo());
        assert_eq!(app.graphs.len(), initial_count + 2);
        assert!(app.undo());
        assert_eq!(app.graphs.len(), initial_count + 1);
        
        // Redo 1 раз
        assert!(app.redo());
        assert_eq!(app.graphs.len(), initial_count + 2);
    }

    #[test]
    fn test_undo_redo_clear() {
        let mut app = PlotApp::default();
        
        // Добавляем действие
        app.save_snapshot();
        app.graphs.push(GraphEntry::new("f1", Color32::RED, "x"));
        
        // Undo
        assert!(app.undo());
        assert_eq!(app.graphs.len(), 2); // начальное состояние
        
        // Новое действие после undo — redo должен очиститься
        app.save_snapshot();
        app.graphs.push(GraphEntry::new("f2", Color32::BLUE, "x^2"));
        
        // Redo должен быть пуст
        assert!(!app.redo());
    }

    #[test]
    fn test_undo_redo_limit() {
        let mut app = PlotApp::default();
        app.max_history = 3;
        
        // Начальное состояние: 2 графика
        let initial_count = app.graphs.len();
        assert_eq!(initial_count, 2);
        
        // Добавляем 5 действий
        for i in 0..5 {
            app.save_snapshot();
            app.graphs.push(GraphEntry::new(&format!("f{}", i), Color32::RED, "x"));
        }
        
        // Должно остаться только 3 последних (из-за max_history = 3)
        assert_eq!(app.undo_stack.len(), 3);
        
        // Undo 3 раза — должны восстановиться первые 2 графика
        // После 3 undos: 2 + (5-3) = 4 графика
        assert!(app.undo());
        assert!(app.undo());
        assert!(app.undo());
        assert_eq!(app.graphs.len(), initial_count + 2); // 2 + 2 = 4
        
        // Ещё один undo — стек пуст
        assert!(!app.undo());
    }

    #[test]
    fn test_undo_no_graphs() {
        let mut app = PlotApp::default();
        
        // Undo без снимков — ничего не происходит
        assert!(!app.undo());
        assert_eq!(app.graphs.len(), 2); // начальное состояние
    }

    // --- Тесты formula_type_info ---

    #[test]
    fn test_formula_type_regular() {
        let entry = GraphEntry::new("f1", Color32::RED, "x^2");
        let info = entry.formula_type_info();
        assert!(info.is_some());
        let (color, _icon, label) = info.unwrap();
        assert_eq!(label, "Обычная");
        assert_eq!(color, Color32::from_rgb(150, 150, 150));
    }

    #[test]
    fn test_formula_type_derivative() {
        let entry = GraphEntry::new("f1", Color32::RED, "deriv(sin(x))");
        let info = entry.formula_type_info();
        assert!(info.is_some());
        let (_color, _icon, label) = info.unwrap();
        assert_eq!(label, "Производная");
    }

    #[test]
    fn test_formula_type_integral() {
        let entry = GraphEntry::new("f1", Color32::RED, "integral(sin(x), 0, pi)");
        let info = entry.formula_type_info();
        assert!(info.is_some());
        let (_color, _icon, label) = info.unwrap();
        assert_eq!(label, "Интеграл");
    }

    #[test]
    fn test_formula_type_polar() {
        let entry = GraphEntry::new("f1", Color32::RED, "polar(cos(x), 0, 2*pi)");
        let info = entry.formula_type_info();
        assert!(info.is_some());
        let (_color, _icon, label) = info.unwrap();
        assert_eq!(label, "Полярная");
    }

    #[test]
    fn test_formula_type_parametric() {
        let entry = GraphEntry::new("f1", Color32::RED, "parametric(cos(t), sin(t), 0, 2*pi)");
        let info = entry.formula_type_info();
        assert!(info.is_some());
        let (_color, _icon, label) = info.unwrap();
        assert_eq!(label, "Параметрическая");
    }

    #[test]
    fn test_formula_type_parse_error() {
        let entry = GraphEntry::new("f1", Color32::RED, "invalid_syntax_here");
        // При ошибке парсинга parsed = None, поэтому formula_type_info = None
        assert!(entry.formula_type_info().is_none());
    }

    // --- Тесты таблицы значений ---

    #[test]
    fn test_generate_values_table_basic() {
        let mut app = PlotApp::default();
        app.graphs.clear();
        app.graphs.push(GraphEntry::new("f1", Color32::RED, "x"));
        app.recompute_all();
        
        let table = app.generate_values_table(0, 5);
        assert!(table.is_some());
        let table = table.unwrap();
        assert_eq!(table.len(), 5);
        
        // Для функции y = x, первая точка должна быть при x = viewport.x_min
        let first = table[0];
        assert!(first.0.abs() - app.viewport.x_min.abs() < 0.01);
    }

    #[test]
    fn test_generate_values_table_empty_graph() {
        let mut app = PlotApp::default();
        app.graphs.clear();
        // Не добавляем графики — таблица должна вернуть None
        let table = app.generate_values_table(0, 5);
        assert!(table.is_none());
    }

    #[test]
    fn test_generate_values_table_steps_range() {
        let mut app = PlotApp::default();
        app.graphs.clear();
        app.graphs.push(GraphEntry::new("f1", Color32::RED, "x"));
        app.recompute_all();
        
        // Минимальное количество шагов
        let table_min = app.generate_values_table(0, 2);
        assert!(table_min.is_some());
        assert_eq!(table_min.unwrap().len(), 2);
        
        // Максимальное количество шагов
        let table_max = app.generate_values_table(0, 10000);
        assert!(table_max.is_some());
        // Должно быть ограничено 1000
        assert_eq!(table_max.unwrap().len(), 1000);
    }

    #[test]
    fn test_generate_values_table_sin() {
        let mut app = PlotApp::default();
        app.graphs.clear();
        app.graphs.push(GraphEntry::new("f1", Color32::RED, "sin(x)"));
        app.recompute_all();
        
        let table = app.generate_values_table(0, 10);
        assert!(table.is_some());
        let table = table.unwrap();
        
        // Для sin(x) значения должны быть в диапазоне [-1, 1]
        for (_, y) in &table {
            if y.is_finite() {
                assert!(*y >= -1.0 - 1e-10, "Y должно быть >= -1");
                assert!(*y <= 1.0 + 1e-10, "Y должно быть <= 1");
            }
        }
    }
}
