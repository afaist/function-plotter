//! Левая панель управления: графики, диапазон, экспорт, сессии.

use std::time::Instant;

use egui::{Color32, Context, ScrollArea, Ui};

use crate::app::{GraphEntry, PlotApp, StatusMessage};
use crate::config;
use crate::export;
use crate::export::{save_dialog, save_png};
use crate::parser::ParsedFormula;
use crate::renderer::Viewport;

/// Отрисовка одной записи графика в левой панели.
/// Возвращает true, если были изменения.
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
    formula_type_info: Option<(Color32, String, String)>,
) -> bool {
    let mut changed = false;
    
    ui.horizontal(|ui| {
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::click());
        if resp.clicked() {
            *selected_graph = Some(i);
        }
        ui.painter().rect_filled(rect, 0.0, color);
        if ui.checkbox(visible, label.to_string()).changed() {
            changed = true;
        }
        if ui.button("✕").clicked() {
            *need_remove = Some(i);
            changed = true;
        }
    });

    // Индикатор типа формулы
    if let Some((type_color, _icon, type_label)) = &formula_type_info {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(type_label)
                .color(*type_color)
                .monospace()
                .size(11.0));
        });
    }

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
        changed = true;
    }
    ui.add_space(4.0);
    
    changed
}

/// Отрисовка левой панели управления.
pub fn show_controls_panel(app: &mut PlotApp, ctx: &Context, ui: &mut Ui) {
    ui.heading("Графики");
    ui.add_space(4.0);

    let mut need_recompute = false;
    let mut need_remove: Option<usize> = None;
    let mut need_save_snapshot = false;

    // Кнопка шаблонов
    if ui.button("📋 Шаблоны").clicked() {
        app.show_templates_window = true;
    }

    // Окно шаблонов
    egui::Window::new("Шаблоны функций")
        .resizable(true)
        .open(&mut app.show_templates_window)
        .show(ctx, |ui| {
            ui.label("Выберите шаблон для добавления графика:");
            ui.separator();

            ScrollArea::vertical().show(ui, |ui| {
                for category in crate::templates::categories() {
                    ui.heading(category);
                    let templates = crate::templates::templates_by_category(category);
                    for tmpl in templates {
                        if ui.button(tmpl.name).clicked() {
                            let palette = [
                                Color32::from_rgb(150, 255, 150),
                                Color32::from_rgb(255, 255, 100),
                                Color32::from_rgb(200, 100, 255),
                                Color32::from_rgb(100, 255, 200),
                            ];
                            let idx = app.graphs.len();
                            let color = palette[idx % palette.len()];
                            app.graphs.push(GraphEntry::new(
                                &format!("f{}", idx + 1),
                                color,
                                tmpl.formula,
                            ));
                            app.selected_graph = Some(app.graphs.len() - 1);
                            app.has_unsaved_changes = true;
                            need_save_snapshot = true;
                            need_recompute = true;
                        }
                    }
                    ui.add_space(2.0);
                }
            });
        });

    // Список графиков
    let mut any_changed = false;
    // Предварительно вычисляем типы формул для всех графиков (чтобы избежать borrow conflicts)
    let formula_types: Vec<Option<(Color32, String, String)>> = app.graphs.iter().map(|g| {
        g.formula_type_info().map(|(c, _icon, l)| (c, l.to_string(), l.to_string()))
    }).collect();
    for i in 0..app.graphs.len() {
        let color = app.graphs[i].style.color;
        let label = app.graphs[i].style.label.clone();
        let mut visible = app.graphs[i].style.visible;
        let mut formula = app.graphs[i].formula_text.clone();
        let parse_error = app.graphs[i].parse_error.clone();

        let mut changed = false;
        ui.push_id(i, |ui| {
            if app.selected_graph == Some(i) {
                let frame = egui::Frame {
                    fill: Color32::from_rgb(40, 40, 50),
                    stroke: egui::Stroke::new(1.5_f32, color),
                    ..Default::default()
                };
                frame.show(ui, |ui| {
                    changed = render_graph_entry(
                        ui,
                        i,
                        color,
                        &label,
                        &mut visible,
                        &mut formula,
                        &parse_error,
                        &mut app.selected_graph,
                        &mut need_remove,
                        &mut need_recompute,
                        formula_types[i].clone(),
                    );
                });
            } else {
                changed = render_graph_entry(
                    ui,
                    i,
                    color,
                    &label,
                    &mut visible,
                    &mut formula,
                    &parse_error,
                    &mut app.selected_graph,
                    &mut need_remove,
                    &mut need_recompute,
                    formula_types[i].clone(),
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
        // Сохраняем снимок при изменении формулы или видимости
        if changed {
            app.has_unsaved_changes = true;
            need_save_snapshot = true;
            any_changed = true;
        }
    }

    // Сохраняем снимок после применения изменений к графикам
    if any_changed || need_save_snapshot {
        app.save_snapshot();
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
        app.has_unsaved_changes = true;
        need_save_snapshot = true;
        need_recompute = true;
    }

    if let Some(i) = need_remove {
        app.graphs.remove(i);
        app.has_unsaved_changes = true;
        need_save_snapshot = true;
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
            app.has_unsaved_changes = true;
            app.mark_all_dirty();
            need_save_snapshot = true;
        }
    });
    ui.horizontal(|ui| {
        ui.label("x max:");
        if ui.add(egui::DragValue::new(&mut x_max)).changed() {
            app.x_max = x_max as f64;
            app.has_unsaved_changes = true;
            app.mark_all_dirty();
            need_save_snapshot = true;
        }
    });
    ui.horizontal(|ui| {
        ui.label("точки:");
        if ui
            .add(egui::DragValue::new(&mut n).range(10..=10000))
            .changed()
        {
            app.n_points = n as usize;
            app.has_unsaved_changes = true;
            app.mark_all_dirty();
            need_save_snapshot = true;
        }
    });

    // Полярные координаты
    ui.add_space(4.0);
    if ui.checkbox(&mut app.polar_mode, "Полярные координаты").changed() {
        app.has_unsaved_changes = true;
        need_save_snapshot = true;
        app.mark_all_dirty();
    }

    // Диапазон Y
    ui.add_space(8.0);
    ui.separator();
    ui.heading("Диапазон Y");
    if ui.checkbox(&mut app.auto_y, "Авто-масштаб Y").changed() {
        app.has_unsaved_changes = true;
        need_save_snapshot = true;
    }

    // Адаптивный алгоритм
    ui.add_space(4.0);
    if ui.checkbox(&mut app.adaptive, "Адаптивная плотность").changed() {
        app.has_unsaved_changes = true;
        need_save_snapshot = true;
    }
    if app.adaptive {
        let mut tolerance = app.adaptive_tolerance;
        ui.horizontal(|ui| {
            if ui
                .add(egui::Slider::new(&mut tolerance, 0.0001..=1.0).text("точность"))
                .changed()
            {
                app.adaptive_tolerance = tolerance;
                app.has_unsaved_changes = true;
                need_save_snapshot = true;
            }
        });
    }

    if !app.auto_y {
        let mut y_min = app.viewport.y_min as f32;
        let mut y_max = app.viewport.y_max as f32;
        ui.horizontal(|ui| {
            ui.label("y min:");
            if ui.add(egui::DragValue::new(&mut y_min)).changed() {
                app.has_unsaved_changes = true;
                need_save_snapshot = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("y max:");
            if ui.add(egui::DragValue::new(&mut y_max)).changed() {
                app.has_unsaved_changes = true;
                need_save_snapshot = true;
            }
        });
        app.viewport.y_min = y_min as f64;
        app.viewport.y_max = y_max as f64;
    }

    // Кнопки управления масштабом
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.button("⟲ Сбросить масштаб").clicked() {
            app.viewport = Viewport::new();
            app.x_min = app.viewport.x_min;
            app.x_max = app.viewport.x_max;
            app.auto_y = true;
            app.has_unsaved_changes = true;
            need_save_snapshot = true;
            need_recompute = true;
        }
    });
    ui.horizontal(|ui| {
        if ui.button(app.auto_y.then_some("⟳ Авто Y").unwrap_or("⟳ Авто Y (выкл)")).clicked() {
            app.auto_y = !app.auto_y;
            app.has_unsaved_changes = true;
            need_save_snapshot = true;
            need_recompute = true;
        }
    });

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

    // Кнопки помощи
    ui.add_space(8.0);
    ui.separator();
    ui.horizontal(|ui| {
        if ui.button("❓ Помощь").clicked() {
            app.show_help_window = true;
        }
        if ui.button("ℹ️ О программе").clicked() {
            app.show_about_window = true;
        }
        if ui.button("📊 Таблица").clicked() {
            app.show_values_table = true;
            app.values_table_graph_index = app.selected_graph;
        }
    });

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
                            app.status_msg =
                                Some(StatusMessage::Info(format!("Сохранено: {}", p.display())));
                            app.status_time = Some(Instant::now());
                        }
                        Err(e) => {
                            app.status_msg = Some(StatusMessage::Error(format!("Ошибка: {e}")));
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
                match export::export_multi(
                    &formulas,
                    &labels,
                    app.x_min,
                    app.x_max,
                    app.n_points,
                    &p,
                ) {
                    Ok(()) => {
                        app.status_msg =
                            Some(StatusMessage::Info(format!("Сохранено: {}", p.display())));
                        app.status_time = Some(Instant::now());
                    }
                    Err(e) => {
                        app.status_msg = Some(StatusMessage::Error(format!("Ошибка: {e}")));
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
                    app.status_msg = Some(StatusMessage::Error(format!("Ошибка PNG: {e}")));
                    app.status_time = Some(Instant::now());
                }
            }
        }
    }
}

/// Содержимое окна помощи.
pub fn render_help_content(ui: &mut Ui) {
    ui.heading("Горячие клавиши");
    ui.horizontal(|ui| {
        ui.label("Ctrl+N");
        ui.separator();
        ui.label("Новый график");
    });
    ui.horizontal(|ui| {
        ui.label("Ctrl+O");
        ui.separator();
        ui.label("Загрузить сессию");
    });
    ui.horizontal(|ui| {
        ui.label("Ctrl+S");
        ui.separator();
        ui.label("Сохранить сессию");
    });
    ui.horizontal(|ui| {
        ui.label("Delete");
        ui.separator();
        ui.label("Удалить выбранный график");
    });
    ui.horizontal(|ui| {
        ui.label("R");
        ui.separator();
        ui.label("Сбросить масштаб");
    });
    ui.horizontal(|ui| {
        ui.label("F5 / Ctrl+Enter");
        ui.separator();
        ui.label("Пересчитать графики");
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);

    ui.heading("Примеры функций");
    ui.label("sin(x)");
    ui.label("cos(x)");
    ui.label("tan(x)");
    ui.label("x^2");
    ui.label("sqrt(x)");
    ui.label("exp(x)");
    ui.label("log(x)");
    ui.label("abs(x)");
    ui.label("sin(x) / x");

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);

    ui.heading("Специальные функции");
    ui.label("deriv(sin(x)) — производная");
    ui.label("integral(sin(x), 0, 3.14) — определённый интеграл");
}

/// Содержимое окна "О программе".
pub fn render_about_content(ui: &mut Ui) {
    ui.heading("Function Plotter");
    ui.label(format!("Версия: {}", env!("CARGO_PKG_VERSION")));
    ui.add_space(4.0);
    ui.label("Интерактивная программа для построения графиков функций");
    ui.label("с поддержкой производных, интегралов и экспорта.");
    ui.add_space(4.0);
    ui.label("Rust + eframe/egui");
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
                    app.last_session_path = Some(p.clone());
                    app.pending_save_on_exit = false;
                    app.has_unsaved_changes = false;
                    // Сохраняем путь в config
                    let mut config = config::AppConfig::load();
                    config.last_session_path = Some(p.to_string_lossy().to_string());
                    let _ = config.save();
                    app.status_msg =
                        Some(StatusMessage::Info(format!("Сохранено: {}", p.display())));
                    app.status_time = Some(Instant::now());
                }
                Err(e) => {
                    app.status_msg = Some(StatusMessage::Error(format!("Ошибка: {e}")));
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
                    app.last_session_path = Some(p.clone());
                    app.pending_save_on_exit = false;
                    app.has_unsaved_changes = false;
                    // Сохраняем путь в config
                    let mut config = config::AppConfig::load();
                    config.last_session_path = Some(p.to_string_lossy().to_string());
                    let _ = config.save();
                    app.status_msg =
                        Some(StatusMessage::Info(format!("Загружено: {}", p.display())));
                    app.status_time = Some(Instant::now());
                }
                Err(e) => {
                    app.status_msg = Some(StatusMessage::Error(format!("Ошибка: {e}")));
                    app.status_time = Some(Instant::now());
                }
            }
        }
    }

    // Ctrl+S
    if ui.input(|i| i.key_pressed(egui::Key::S) && (i.modifiers.ctrl || i.modifiers.command)) {
        if let Some(ref p) = app.session_path {
            let session = crate::session::SessionData::from_app(app);
            match session.save_to_file(p) {
                Ok(()) => {
                    app.last_session_path = app.session_path.clone();
                    app.pending_save_on_exit = false;
                    app.has_unsaved_changes = false;
                    // Сохраняем путь в config
                    let mut config = config::AppConfig::load();
                    config.last_session_path = Some(p.to_string_lossy().to_string());
                    let _ = config.save();
                    app.status_msg = Some(StatusMessage::Info("Сессия сохранена".to_string()));
                    app.status_time = Some(Instant::now());
                }
                Err(e) => {
                    app.status_msg = Some(StatusMessage::Error(format!("Ошибка: {e}")));
                    app.status_time = Some(Instant::now());
                }
            }
        }
    }

    // Настройки сессии
    ui.add_space(4.0);
    ui.separator();

    let config = config::AppConfig::load();
    let mut auto_load = config.auto_load_last_session;
    if ui.checkbox(&mut auto_load, "Автозагрузка последней сессии").changed() {
        let mut new_config = config;
        new_config.auto_load_last_session = auto_load;
        if let Err(e) = new_config.save() {
            app.status_msg = Some(StatusMessage::Error(format!("Ошибка сохранения настроек: {e}")));
            app.status_time = Some(Instant::now());
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
