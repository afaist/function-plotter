//! Левая панель управления: графики, диапазон, экспорт, сессии.

use std::time::Instant;

use egui::{Color32, Context, Ui};

use crate::app::{GraphEntry, PlotApp, StatusMessage};
use crate::export;
use crate::export::{save_dialog, save_png};
use crate::parser::ParsedFormula;

/// Отрисовка одной записи графика в левой панели.
fn render_graph_entry(
    ui: &mut Ui,
    i: usize,
    color: Color32,
    label: &str,
    visible: &mut bool,
    formula: &mut String,
    parse_error: &Option<String>,
    selected_graph: &mut Option<usize>,
    need_remove: &mut Option<usize>,
    need_recompute: &mut bool,
) {
    ui.horizontal(|ui| {
        let (rect, resp) =
            ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::click());
        if resp.clicked() {
            *selected_graph = Some(i);
        }
        ui.painter().rect_filled(rect, 0.0, color);
        ui.checkbox(visible, label.to_string());
        if ui.button("✕").clicked() {
            *need_remove = Some(i);
        }
    });

    let text_edit = egui::TextEdit::singleline(formula).desired_width(f32::INFINITY);
    let text_output = text_edit.show(ui);
    let resp = &text_output.response;

    if let Some(ref err) = parse_error {
        ui.painter().rect_stroke(
            resp.rect,
            4.0,
            egui::Stroke::new(1.5_f32, Color32::from_rgb(255, 80, 80)),
            egui::StrokeKind::Inside,
        );
        ui.colored_label(Color32::from_rgb(255, 100, 100), err);
    }

    if resp.lost_focus() {
        *need_recompute = true;
    }
    ui.add_space(4.0);
}

/// Отрисовка левой панели управления.
pub fn show_controls_panel(app: &mut PlotApp, ctx: &Context, ui: &mut Ui) {
    ui.heading("Графики");
    ui.add_space(4.0);

    let mut need_recompute = false;
    let mut need_remove: Option<usize> = None;

    // Список графиков
    for i in 0..app.graphs.len() {
        let color = app.graphs[i].style.color;
        let label = app.graphs[i].style.label.clone();
        let mut visible = app.graphs[i].style.visible;
        let mut formula = app.graphs[i].formula_text.clone();
        let parse_error = app.graphs[i].parse_error.clone();

        ui.push_id(i, |ui| {
            if app.selected_graph == Some(i) {
                let frame = egui::Frame {
                    fill: Color32::from_rgb(40, 40, 50),
                    stroke: egui::Stroke::new(1.5_f32, color),
                    ..Default::default()
                };
                frame.show(ui, |ui| {
                    render_graph_entry(
                        ui, i, color, &label, &mut visible, &mut formula, &parse_error,
                        &mut app.selected_graph, &mut need_remove, &mut need_recompute,
                    );
                });
            } else {
                render_graph_entry(
                    ui, i, color, &label, &mut visible, &mut formula, &parse_error,
                    &mut app.selected_graph, &mut need_remove, &mut need_recompute,
                );
            }
        });

        // Применяем изменения
        if let Some(g) = app.graphs.get_mut(i) {
            g.style.visible = visible;
            g.style.label = label;
            g.formula_text = formula;
            g.reparse();
        }
    }

    // Добавить график
    if ui.button("+ Добавить график").clicked() {
        let palette = [
            Color32::from_rgb(150, 255, 150),
            Color32::from_rgb(255, 255, 100),
            Color32::from_rgb(200, 100, 255),
            Color32::from_rgb(100, 255, 200),
        ];
        let idx = app.graphs.len();
        let color = palette[idx % palette.len()];
        app.graphs
            .push(GraphEntry::new(&format!("f{}", idx + 1), color, "x"));
        need_recompute = true;
    }

    if let Some(i) = need_remove {
        app.graphs.remove(i);
        need_recompute = true;
    }

    // Диапазон X
    ui.add_space(8.0);
    ui.separator();
    ui.heading("Диапазон X");

    let mut x_min = app.x_min as f32;
    let mut x_max = app.x_max as f32;
    let mut n = app.n_points as i32;

    ui.horizontal(|ui| {
        ui.label("x min:");
        if ui.add(egui::DragValue::new(&mut x_min)).changed() {
            app.x_min = x_min as f64;
            app.mark_all_dirty();
        }
    });
    ui.horizontal(|ui| {
        ui.label("x max:");
        if ui.add(egui::DragValue::new(&mut x_max)).changed() {
            app.x_max = x_max as f64;
            app.mark_all_dirty();
        }
    });
    ui.horizontal(|ui| {
        ui.label("точки:");
        if ui
            .add(egui::DragValue::new(&mut n).range(10..=10000))
            .changed()
        {
            app.n_points = n as usize;
            app.mark_all_dirty();
        }
    });

    // Диапазон Y
    ui.add_space(8.0);
    ui.separator();
    ui.heading("Диапазон Y");
    ui.checkbox(&mut app.auto_y, "Авто-масштаб Y");
    
    // Адаптивный алгоритм
    ui.add_space(4.0);
    ui.checkbox(&mut app.adaptive, "Адаптивная плотность");
    if app.adaptive {
        ui.horizontal(|ui| {
            ui.add(egui::Slider::new(&mut app.adaptive_tolerance, 0.0001..=1.0)
                .text("точность"));
        });
    }
    
    if !app.auto_y {
        let mut y_min = app.viewport.y_min as f32;
        let mut y_max = app.viewport.y_max as f32;
        ui.horizontal(|ui| {
            ui.label("y min:");
            ui.add(egui::DragValue::new(&mut y_min));
        });
        ui.horizontal(|ui| {
            ui.label("y max:");
            ui.add(egui::DragValue::new(&mut y_max));
        });
        app.viewport.y_min = y_min as f64;
        app.viewport.y_max = y_max as f64;
    }

    // Экспорт
    ui.add_space(8.0);
    ui.separator();
    ui.heading("Экспорт");

    show_export_ui(app, ctx, ui, &mut need_recompute);

    // Сессия
    ui.add_space(8.0);
    ui.separator();
    ui.heading("Сессия");
    show_session_ui(app, ctx, ui);

    // Статус-бар
    show_status_bar(app, ui);

    if need_recompute {
        app.recompute_all();
    }
}

/// Панель экспорта (CSV, PNG).
fn show_export_ui(app: &mut PlotApp, ctx: &Context, ui: &mut Ui, _need_recompute: &mut bool) {
    if ui.button("Сохранить CSV (текущий график)").clicked() {
        if let Some(g) = app.graphs.iter().find(|g| g.parsed.is_some()) {
            if let Some(ref f) = g.parsed {
                let path = save_dialog("graph.csv");
                if let Some(p) = path {
                    match export::export_single(f, app.x_min, app.x_max, app.n_points, &p) {
                        Ok(()) => {
                            app.status_msg = Some(StatusMessage::Info(format!(
                                "Сохранено: {}",
                                p.display()
                            )));
                            app.status_time = Some(Instant::now());
                        }
                        Err(e) => {
                            app.status_msg = Some(StatusMessage::Error(format!(
                                "Ошибка: {e}"
                            )));
                            app.status_time = Some(Instant::now());
                        }
                    }
                }
            }
        }
    }

    if ui.button("Сохранить CSV (все графики)").clicked() {
        let formulas: Vec<&ParsedFormula> = app
            .graphs
            .iter()
            .filter_map(|g| g.parsed.as_ref())
            .collect();
        let labels: Vec<&str> = app
            .graphs
            .iter()
            .filter(|g| g.parsed.is_some())
            .map(|g| g.style.label.as_str())
            .collect();
        if !formulas.is_empty() {
            let path = save_dialog("graphs.csv");
            if let Some(p) = path {
                match export::export_multi(&formulas, &labels, app.x_min, app.x_max, app.n_points, &p) {
                    Ok(()) => {
                        app.status_msg = Some(StatusMessage::Info(format!(
                            "Сохранено: {}",
                            p.display()
                        )));
                        app.status_time = Some(Instant::now());
                    }
                    Err(e) => {
                        app.status_msg = Some(StatusMessage::Error(format!(
                            "Ошибка: {e}"
                        )));
                        app.status_time = Some(Instant::now());
                    }
                }
            }
        }
    }

    if ui.button("Сохранить PNG").clicked() {
        let path = rfd::FileDialog::new()
            .set_file_name("plot.png")
            .add_filter("PNG", &["png"])
            .save_file();

        if let Some(ref p) = path {
            match save_png(ctx, app) {
                Ok(()) => {
                    app.status_msg = Some(StatusMessage::Info(format!(
                        "PNG сохранён: {}",
                        p.display()
                    )));
                    app.status_time = Some(Instant::now());
                }
                Err(e) => {
                    app.status_msg = Some(StatusMessage::Error(format!(
                        "Ошибка PNG: {e}"
                    )));
                    app.status_time = Some(Instant::now());
                }
            }
        }
    }
}

/// Панель сессий (сохранить/загрузить).
fn show_session_ui(app: &mut PlotApp, _ctx: &Context, ui: &mut Ui) {
    if ui.button("Сохранить сессию").clicked() {
        let path = rfd::FileDialog::new()
            .set_file_name("plot_session.json")
            .add_filter("JSON", &["json"])
            .save_file();

        if let Some(ref p) = path {
            let session = crate::session::SessionData::from_app(app);
            match session.save_to_file(p) {
                Ok(()) => {
                    app.session_path = Some(p.clone());
                    app.status_msg = Some(StatusMessage::Info(format!(
                        "Сохранено: {}",
                        p.display()
                    )));
                    app.status_time = Some(Instant::now());
                }
                Err(e) => {
                    app.status_msg = Some(StatusMessage::Error(format!(
                        "Ошибка: {e}"
                    )));
                    app.status_time = Some(Instant::now());
                }
            }
        }
    }

    if ui.button("Загрузить сессию").clicked() {
        let path = rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .pick_file();

        if let Some(ref p) = path {
            match crate::session::SessionData::load_from_file(p) {
                Ok(session_data) => {
                    session_data.apply_to_app(app);
                    app.session_path = Some(p.clone());
                    app.status_msg = Some(StatusMessage::Info(format!(
                        "Загружено: {}",
                        p.display()
                    )));
                    app.status_time = Some(Instant::now());
                }
                Err(e) => {
                    app.status_msg = Some(StatusMessage::Error(format!(
                        "Ошибка: {e}"
                    )));
                    app.status_time = Some(Instant::now());
                }
            }
        }
    }

    // Ctrl+S
    if ui
        .input(|i| i.key_pressed(egui::Key::S) && (i.modifiers.ctrl || i.modifiers.command))
    {
        if let Some(ref p) = app.session_path {
            let session = crate::session::SessionData::from_app(app);
            match session.save_to_file(p) {
                Ok(()) => {
                    app.status_msg = Some(StatusMessage::Info("Сессия сохранена".to_string()));
                    app.status_time = Some(Instant::now());
                }
                Err(e) => {
                    app.status_msg = Some(StatusMessage::Error(format!(
                        "Ошибка: {e}"
                    )));
                    app.status_time = Some(Instant::now());
                }
            }
        }
    }
}

/// Статус-бар с автоматическим затуханием.
fn show_status_bar(app: &mut PlotApp, ui: &mut Ui) {
    if let Some(ref msg) = app.status_msg {
        let color = msg.color();
        ui.add_space(4.0);
        ui.colored_label(color, msg.text());

        if let Some(start) = app.status_time {
            let elapsed = start.elapsed();
            if elapsed >= app.status_duration {
                app.status_msg = None;
                app.status_time = None;
            }
        }
    }
}
