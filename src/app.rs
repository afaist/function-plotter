use std::path::PathBuf;

use egui::{Color32, Context, Pos2, Sense, Ui, Vec2};

use crate::evaluator::PlotData;
use crate::export;
use crate::parser::{self, ParsedFormula};
use crate::renderer::{self, PlotStyle, Viewport};
use crate::session;

/// Один график: формула, стиль, вычисленные точки.
pub struct GraphEntry {
    pub formula_text: String,
    pub parsed: Option<ParsedFormula>,
    pub parse_error: Option<String>,
    pub style: PlotStyle,
    pub data: Option<PlotData>,
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
        };
        entry.reparse();
        entry
    }

    fn reparse(&mut self) {
        match parser::parse(&self.formula_text) {
            Ok(p) => {
                self.parsed = Some(p);
                self.parse_error = None;
            }
            Err(e) => {
                self.parsed = None;
                self.parse_error = Some(e);
            }
        }
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
    pub status_msg: Option<(String, bool)>,
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
    pub fn recompute_all(&mut self) {
        for g in &mut self.graphs {
            if let Some(ref f) = g.parsed {
                g.data = Some(PlotData::compute(f, self.x_min, self.x_max, self.n_points));
            } else {
                g.data = None;
            }
        }
        if self.auto_y {
            self.update_viewport_y();
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
        // --- Левая панель управления ---
        egui::SidePanel::left("controls").show(ctx, |ui| {
            ui.heading("Графики");
            ui.add_space(4.0);

            let mut need_recompute = false;
            let mut need_remove: Option<usize> = None;
            for i in 0..self.graphs.len() {
                let graph_count = self.graphs.len(); // ← захватываем до мутабельного заимства
                ui.push_id(i, |ui| {
                    let g = &mut self.graphs[i];
                    ui.horizontal(|ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 0.0, g.style.color);

                        ui.checkbox(&mut g.style.visible, g.style.label.clone());
                        if ui.button("✕").clicked() && graph_count > 1 {
                            // ← используем захваченное значение
                            need_remove = Some(i);
                        }
                    });
                    let resp = ui.text_edit_singleline(&mut g.formula_text);
                    if resp.lost_focus() {
                        g.reparse();
                        need_recompute = true;
                    }
                    if let Some(ref err) = g.parse_error {
                        ui.colored_label(Color32::from_rgb(255, 100, 100), err);
                    }
                    ui.add_space(4.0);
                });
            }

            if ui.button("+ Добавить график").clicked() {
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
                need_recompute = true;
            }

            if let Some(i) = need_remove {
                self.graphs.remove(i);
                need_recompute = true;
            }

            ui.add_space(8.0);
            ui.separator();
            ui.heading("Диапазон X");

            let mut x_min = self.x_min as f32;
            let mut x_max = self.x_max as f32;
            let mut n = self.n_points as i32;

            ui.horizontal(|ui| {
                ui.label("x min:");
                if ui.add(egui::DragValue::new(&mut x_min)).changed() {
                    self.x_min = x_min as f64;
                    need_recompute = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("x max:");
                if ui.add(egui::DragValue::new(&mut x_max)).changed() {
                    self.x_max = x_max as f64;
                    need_recompute = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("точки:");
                if ui
                    .add(egui::DragValue::new(&mut n).range(10..=10000))
                    .changed()
                {
                    self.n_points = n as usize;
                    need_recompute = true;
                }
            });

            ui.checkbox(&mut self.auto_y, "Авто-масштаб Y");
            if !self.auto_y {
                let mut y_min = self.viewport.y_min as f32;
                let mut y_max = self.viewport.y_max as f32;
                ui.horizontal(|ui| {
                    ui.label("y min:");
                    ui.add(egui::DragValue::new(&mut y_min));
                });
                ui.horizontal(|ui| {
                    ui.label("y max:");
                    ui.add(egui::DragValue::new(&mut y_max));
                });
                self.viewport.y_min = y_min as f64;
                self.viewport.y_max = y_max as f64;
            }

            ui.add_space(8.0);
            ui.separator();

            // --- Экспорт ---
            ui.heading("Экспорт");
            if ui.button("Сохранить CSV (текущий график)").clicked() {
                if let Some(g) = self.graphs.iter().find(|g| g.parsed.is_some()) {
                    if let Some(ref f) = g.parsed {
                        let path = save_dialog("graph.csv");
                        if let Some(p) = path {
                            match export::export_single(
                                f,
                                self.x_min,
                                self.x_max,
                                self.n_points,
                                &p,
                            ) {
                                Ok(()) => {
                                    self.status_msg =
                                        Some((format!("Сохранено: {}", p.display()), true));
                                }
                                Err(e) => {
                                    self.status_msg = Some((format!("Ошибка: {e}"), false));
                                }
                            }
                        }
                    }
                }
            }
            if ui.button("Сохранить CSV (все графики)").clicked() {
                let formulas: Vec<&ParsedFormula> = self
                    .graphs
                    .iter()
                    .filter_map(|g| g.parsed.as_ref())
                    .collect();
                let labels: Vec<&str> = self
                    .graphs
                    .iter()
                    .filter(|g| g.parsed.is_some())
                    .map(|g| g.style.label.as_str())
                    .collect();
                if !formulas.is_empty() {
                    let path = save_dialog("graphs.csv");
                    if let Some(p) = path {
                        match export::export_multi(
                            &formulas,
                            &labels,
                            self.x_min,
                            self.x_max,
                            self.n_points,
                            &p,
                        ) {
                            Ok(()) => {
                                self.status_msg =
                                    Some((format!("Сохранено: {}", p.display()), true));
                            }
                            Err(e) => {
                                self.status_msg = Some((format!("Ошибка: {e}"), false));
                            }
                        }
                    }
                }
            }

            ui.add_space(8.0);
            ui.separator();
            ui.heading("Сессия");

            // --- Сохранить ---
            if ui.button("Сохранить сессию").clicked() {
                let path = rfd::FileDialog::new()
                    .set_file_name("plot_session.json")
                    .add_filter("JSON", &["json"])
                    .save_file();

                if let Some(ref p) = path {
                    let session = session::SessionData::from_app(self);
                    match session.save_to_file(p) {
                        Ok(()) => {
                            self.session_path = Some(p.clone());
                            self.status_msg = Some((format!("Сохранено: {}", p.display()), true));
                        }
                        Err(e) => {
                            self.status_msg = Some((format!("Ошибка: {e}"), false));
                        }
                    }
                }
            }

            // --- Загрузить ---
            if ui.button("Загрузить сессию").clicked() {
                let path = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .pick_file();

                if let Some(ref p) = path {
                    match session::SessionData::load_from_file(p) {
                        Ok(session_data) => {
                            session_data.apply_to_app(self);
                            self.session_path = Some(p.clone());
                            self.status_msg = Some((format!("Загружено: {}", p.display()), true));
                        }
                        Err(e) => {
                            self.status_msg = Some((format!("Ошибка: {e}"), false));
                        }
                    }
                }
            }

            // --- Ctrl+S ---
            if ui
                .input(|i| i.key_pressed(egui::Key::S) && (i.modifiers.ctrl || i.modifiers.command))
            {
                if let Some(ref p) = self.session_path {
                    let session = session::SessionData::from_app(self);
                    match session.save_to_file(p) {
                        Ok(()) => {
                            self.status_msg = Some(("Сессия сохранена".to_string(), true));
                        }
                        Err(e) => {
                            self.status_msg = Some((format!("Ошибка: {e}"), false));
                        }
                    }
                }
            }

            // --- Статус-бар ---
            if let Some(ref msg) = self.status_msg {
                let (text, success) = msg;
                let color = if *success {
                    Color32::from_rgb(100, 255, 100)
                } else {
                    Color32::from_rgb(255, 100, 100)
                };
                ui.add_space(4.0);
                ui.colored_label(color, text);
            }

            if need_recompute {
                self.recompute_all();
            }
        });

        // --- Центральная область: холст для графиков ---
        egui::CentralPanel::default().show(ctx, |ui: &mut Ui| {
            let (rect, response) =
                ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());

            // Масштабирование (колесо мыши)
            if let Some(pos) = response.hover_pos() {
                let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll != 0.0 {
                    let factor = if scroll > 0.0 { 0.9 } else { 1.1 };
                    self.viewport.zoom(factor, pos.to_vec2(), rect);
                }
            }

            // Перетаскивание (pan)
            if response.dragged() {
                if let Some(start) = self.drag_start {
                    let current = response.interact_pointer_pos().unwrap_or(start);
                    let dx_screen = current.x - start.x;
                    let dy_screen = current.y - start.y;
                    let dx_math = -dx_screen as f64 / rect.width() as f64
                        * (self.viewport.x_max - self.viewport.x_min);
                    let dy_math = dy_screen as f64 / rect.height() as f64
                        * (self.viewport.y_max - self.viewport.y_min);
                    self.viewport.pan(dx_math, dy_math);
                    self.drag_start = Some(current);
                } else {
                    self.drag_start = response.interact_pointer_pos();
                }
            } else {
                self.drag_start = None;
            }

            // Синхронизация x_min/x_max с viewport
            self.x_min = self.viewport.x_min;
            self.x_max = self.viewport.x_max;

            // Отрисовка
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 0.0, Color32::from_rgb(30, 30, 35));

            renderer::draw_axes(&painter, rect, &self.viewport);

            for g in &self.graphs {
                if let Some(ref data) = g.data {
                    renderer::draw_curve(&painter, rect, &self.viewport, &data.points, &g.style);
                }
            }

            // Легенда
            let mut legend_y = rect.top() + 8.0;
            for g in &self.graphs {
                if g.style.visible {
                    let pos = egui::pos2(rect.left() + 8.0, legend_y);
                    painter.line_segment(
                        [pos, pos + Vec2::new(16.0, 0.0)],
                        egui::Stroke::new(2.0_f32, g.style.color),
                    );
                    painter.text(
                        pos + Vec2::new(22.0, -6.0),
                        egui::Align2::LEFT_TOP,
                        &g.style.label,
                        egui::FontId::proportional(12.0),
                        Color32::from_gray(200),
                    );
                    legend_y += 18.0;
                }
            }
            // --- Координаты мыши ---
            if let Some(pos) = response.hover_pos() {
                let (mx, my) = self.viewport.screen_to_math(pos, rect);
                let width = self.viewport.x_max - self.viewport.x_min;
                let height = self.viewport.y_max - self.viewport.y_min;
                let step_x = width / 8.0;
                let step_y = height / 8.0;
                let label = format!(
                    "x = {}, y = {}",
                    renderer::format_coord(mx, step_x),
                    renderer::format_coord(my, step_y)
                );
                painter.text(
                    rect.right_top() + Vec2::new(-8.0, 8.0),
                    egui::Align2::RIGHT_TOP,
                    label,
                    egui::FontId::proportional(14.0),
                    Color32::from_rgb(255, 255, 200),
                );
            }
        });
    }
}

/// Диалог сохранения файла через rfd.
fn save_dialog(default_name: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_file_name(default_name)
        .add_filter("CSV", &["csv"])
        .save_file()
}
