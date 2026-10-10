//! Левая панель управления: графики, диапазон, экспорт, сессии.

use std::time::Instant;

use egui::{Color32, Context, ScrollArea, Ui};

use crate::app::{GraphEntry, PlotApp, StatusMessage};
use crate::config;
use crate::export;
use crate::export::save_dialog;
use crate::parser::ParsedFormula;
use crate::renderer::Viewport;
use crate::svg_renderer;
use crate::theme::{FontConfig, Theme};

/// Отрисовка одной записи графика в левой панели.
/// Возвращает true, если были изменения.
#[allow(clippy::too_many_arguments)]
fn render_graph_entry(
    ui: &mut Ui,
    app: &mut PlotApp,
    i: usize,
    color: Color32,
    _label: &str,
    visible: &mut bool,
    formula: &mut String,
    parse_error: &Option<String>,
    selected_graph: &mut Option<usize>,
    need_remove: &mut Option<usize>,
    need_recompute: &mut bool,
    formula_type_info: Option<(Color32, String, String)>,
    has_raw_data: bool,
    use_sliders: &mut bool,
    slider_a: &mut f64,
    slider_b: &mut f64,
    slider_c: &mut f64,
    need_auto_extract: &mut bool,
    need_reparse: &mut bool,
    is_editing: &mut bool,
    theme: &Theme,
    group: &mut usize,
) -> bool {
    let mut changed = false;

    // Верхняя строка: цвет + кнопка видимости + кнопка удаления
    ui.horizontal(|ui| {
        // Цветной индикатор побольше
        let (rect, _resp) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 3.0, color);

        // Кнопка видимости — «глаз»
        let eye_icon = if *visible { "👁" } else { "🚫" };
        let eye_resp = ui.button(eye_icon);
        if eye_resp.clicked() {
            *visible = !*visible;
            changed = true;
        }
        eye_resp.clone().on_hover_text("Показать/скрыть график");

        let del_resp = ui.button("✕");
        if del_resp.clicked() {
            *need_remove = Some(i);
            changed = true;
        }
        del_resp.on_hover_text("Удалить график");
    });

    // Индикатор типа формулы
    if let Some((type_color, _icon, type_label)) = &formula_type_info {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(type_label)
                    .color(*type_color)
                    .monospace()
                    .size(11.0),
            );
        });
    }

    // Выбор группы
    ui.horizontal(|ui| {
        ui.label("Группа:");
        let mut group_str = format!("{}", group);
        egui::ComboBox::from_id_salt(("group", i))
            .selected_text(format!("Группа {}", group))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut group_str, "0".to_string(), "Без группы");
                ui.selectable_value(&mut group_str, "1".to_string(), "Группа 1");
                ui.selectable_value(&mut group_str, "2".to_string(), "Группа 2");
                ui.selectable_value(&mut group_str, "3".to_string(), "Группа 3");
            });
        if let Ok(new_group) = group_str.parse() {
            if new_group != *group {
                *group = new_group;
                changed = true;
            }
        }
    });
    ui.label("Группа").on_hover_text("Назначьте график группе для управления видимостью всей группы");

    // Строка формулы — клик выделяет график
    // formula — это &mut String, egui обновляет его напрямую
    let text_edit = egui::TextEdit::singleline(formula).desired_width(f32::INFINITY);
    let text_output = text_edit.show(ui);

    // Логика авто-скрытия при редактировании формулы
    if !has_raw_data {
        // При получении фокуса — скрываем график и запоминаем старую формулу
        if text_output.response.gained_focus() && !*is_editing {
            *visible = false;
            *is_editing = true;
            changed = true;
            // Запоминаем формулу ДО редактирования в app state
            app.graphs[i].formula_before_edit = formula.clone();
        }

        // При потере фокуса — пересчитываем и показываем
        if text_output.response.lost_focus() && *is_editing {
            *visible = true; // показываем после редактирования
            *is_editing = false;
            changed = true;
            // Помечаем, что нужно пересчитать график
            *need_recompute = true;
            // Если формула изменилась — вызываем reparse
            if formula.trim() != app.graphs[i].formula_before_edit {
                *need_reparse = true;
            }
        }

        // Если формула изменилась и поле НЕ в фокусе — пересчитываем
        if !text_output.response.has_focus() && *is_editing && !formula.is_empty() {
            *is_editing = false;
            *need_reparse = true;
            *need_recompute = true;
        }
    }

    // Кнопка включения/выключения слайдеров
    if !has_raw_data {
        ui.horizontal(|ui| {
            // При включении слайдеров — извлекаем коэффициенты из формулы
            let resp = ui.checkbox(use_sliders, "Слайдеры");
            resp.clone().on_hover_text("Быстрая подстановка коэффициентов a, b, c");
            if resp.changed() && *use_sliders {
                *need_auto_extract = true;
                changed = true;
            }
            if *use_sliders {
                ui.add_space(8.0);
                if ui.button("Авто").clicked() {
                    *need_auto_extract = true;
                    changed = true;
                }
                ui.button("Авто").on_hover_text("Быстрая подстановка коэффициентов a, b, c");
            }
        });
    }

    // Клик по формуле или по типу формулы выделяет график
    if text_output.response.clicked() {
        *selected_graph = Some(i);
    }

    // Отображаем ошибку парсинга только для не-C графиков
    if !has_raw_data {
        if let Some(err) = parse_error {
            ui.painter().rect_stroke(
                text_output.response.rect,
                4.0,
                egui::Stroke::new(1.5_f32, theme.error_color),
                egui::StrokeKind::Inside,
            );
            ui.colored_label(theme.error_bg, err);
        }
    }

    ui.add_space(4.0);

    // Слайдеры для квадратичной функции
    if *use_sliders && !has_raw_data {
        ui.label(egui::RichText::new("a·x² + b·x + c").size(11.0).monospace());
        ui.horizontal(|ui| {
            ui.label("a:");
            if ui
                .add(
                    egui::Slider::new(slider_a, -10.0..=10.0)
                        .logarithmic(false)
                        .show_value(true),
                )
                .changed()
            {
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("b:");
            if ui
                .add(
                    egui::Slider::new(slider_b, -10.0..=10.0)
                        .logarithmic(false)
                        .show_value(true),
                )
                .changed()
            {
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("c:");
            if ui
                .add(
                    egui::Slider::new(slider_c, -10.0..=10.0)
                        .logarithmic(false)
                        .show_value(true),
                )
                .changed()
            {
                changed = true;
            }
        });
        ui.add_space(2.0);
    }

    changed
}

/// Отрисовка левой панели управления.
pub fn show_controls_panel(app: &mut PlotApp, ctx: &Context, ui: &mut Ui) {
    let theme = app.current_theme.resolve(ui.ctx());

    ui.heading("Графики");
    ui.add_space(4.0);

    let mut need_recompute = false;
    let mut need_remove: Option<usize> = None;
    let mut need_save_snapshot = false;

    // Кнопка шаблонов
    let templates_resp = ui.button(egui::RichText::new("📋 Шаблоны").size(13.0));
    templates_resp.clone().on_hover_text("Выберите готовую функцию из шаблонов");
    if templates_resp.clicked() {
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
    let mut any_reparse = false;
    let mut selected_graph = app.selected_graph; // выносим в локальную переменную для избежания borrow conflicts
    // Предварительно вычисляем типы формул для всех графиков (чтобы избежать borrow conflicts)
    let formula_types: Vec<Option<(Color32, String, String)>> = app
        .graphs
        .iter()
        .map(|g| {
            g.formula_type_info()
                .map(|(c, _icon, l)| (c, l.to_string(), l.to_string()))
        })
        .collect();
    // Предварительно вычисляем has_raw_data
    let has_raw_data: Vec<bool> = app.graphs.iter().map(|g| g.raw_data.is_some()).collect();
    for i in 0..app.graphs.len() {
        let color = app.graphs[i].style.color;
        let label = app.graphs[i].style.label.clone();
        let mut visible = app.graphs[i].style.visible;
        let mut parse_error = app.graphs[i].parse_error.clone();
        let mut use_sliders = app.graphs[i].use_sliders;
        let mut slider_a = app.graphs[i].slider_a;
        let mut slider_b = app.graphs[i].slider_b;
        let mut slider_c = app.graphs[i].slider_c;
        let mut need_auto_extract = false;
        let mut need_reparse = false;
        let mut is_editing = app.graphs[i].is_editing;
        // Локальная копия формулы для egui
        let mut formula = app.graphs[i].formula_text.clone();
        let mut group = app.graphs[i].group;

        let mut changed = false;
        ui.push_id(i, |ui| {
            if app.selected_graph == Some(i) {
                let frame = egui::Frame {
                    fill: theme.frame_fill,
                    stroke: egui::Stroke::new(1.5_f32, color),
                    ..Default::default()
                };
                frame.show(ui, |ui| {
                    changed = render_graph_entry(
                        ui,
                        app,
                        i,
                        color,
                        &label,
                        &mut visible,
                        &mut formula,
                        &parse_error,
                        &mut selected_graph,
                        &mut need_remove,
                        &mut need_recompute,
                        formula_types[i].clone(),
                        has_raw_data[i],
                        &mut use_sliders,
                        &mut slider_a,
                        &mut slider_b,
                        &mut slider_c,
                        &mut need_auto_extract,
                        &mut need_reparse,
                        &mut is_editing,
                        &theme,
                        &mut group,
                    );
                    // Копируем актуальные значения обратно
                    app.graphs[i].group = group;
                });
            } else {
                changed = render_graph_entry(
                    ui,
                    app,
                    i,
                    color,
                    &label,
                    &mut visible,
                    &mut formula,
                    &parse_error,
                    &mut selected_graph,
                    &mut need_remove,
                    &mut need_recompute,
                    formula_types[i].clone(),
                    has_raw_data[i],
                    &mut use_sliders,
                    &mut slider_a,
                    &mut slider_b,
                    &mut slider_c,
                    &mut need_auto_extract,
                    &mut need_reparse,
                    &mut is_editing,
                    &theme,
                    &mut group,
                );
                // Копируем актуальные значения обратно
                app.graphs[i].group = group;
            }
        });

        // Синхронизируем формулу обратно в app после egui
        app.graphs[i].formula_text = formula.clone();

        // Применяем изменения
        if let Some(g) = app.graphs.get_mut(i) {
            g.style.visible = visible;
            g.style.label = label.clone();
            g.use_sliders = use_sliders;
            g.is_editing = is_editing;
            // Обновляем parse_error из локальной переменной
            g.parse_error = parse_error.clone();

            // Обновляем формулу и вызываем reparse если нужно
            if g.raw_data.is_none() {
                // Если нажата кнопка "Авто" — извлекаем коэффициенты из формулы
                if need_auto_extract {
                    g.extract_quadratic_coeffs();
                    // Обновляем локальные переменные слайдеров после извлечения
                    slider_a = g.slider_a;
                    slider_b = g.slider_b;
                    slider_c = g.slider_c;
                }

                // Сначала копируем значения из UI в g
                let sliders_changed = use_sliders
                    && (slider_a != g.slider_a || slider_b != g.slider_b || slider_c != g.slider_c);
                if sliders_changed {
                    g.slider_a = slider_a;
                    g.slider_b = slider_b;
                    g.slider_c = slider_c;
                    g.apply_sliders();
                    // Обновляем parse_error после reparse
                    parse_error = g.parse_error.clone();
                    any_reparse = true;
                }
                // Вызываем reparse при потере фокуса (need_reparse == true)
                else if need_reparse {
                    g.reparse();
                    // Обновляем parse_error после reparse
                    parse_error = g.parse_error.clone();
                    any_reparse = true;
                }

                // Копируем актуальные значения слайдеров обратно
                g.slider_a = slider_a;
                g.slider_b = slider_b;
                g.slider_c = slider_c;
            }
        }
        // Сохраняем снимок при изменении формулы или видимости
        if changed {
            app.has_unsaved_changes = true;
            need_save_snapshot = true;
            any_changed = true;
        }
    }
    app.selected_graph = selected_graph;

    // Если был reparse — пересчитываем графики
    if any_reparse {
        app.recompute_all();
    }

    // Сохраняем снимок после применения изменений к графикам
    if any_changed || need_save_snapshot {
        app.save_snapshot();
    }

    // Добавить график — увеличенный шрифт
    let add_resp = ui.button(egui::RichText::new("+ Добавить график").size(13.0));
    add_resp.clone().on_hover_text("Добавить новый график функций");
    if add_resp.clicked() {
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

    // Разделитель
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);

    // Кнопки управления группами
    ui.add_space(8.0);
    ui.separator();
    ui.horizontal(|ui| {
        let groups_label = ui.label("Группы:");
        groups_label.on_hover_text("Нажмите на 👁/🚫 чтобы показать/скрыть все графики группы");
        for g in 1..=3 {
            // Проверяем есть ли графики в этой группе
            let has_graphs_in_group = app.graphs.iter().any(|graph| graph.group == g);
            if has_graphs_in_group {
                // Проверяем видимость всех графиков в группе
                let all_visible = app.graphs.iter()
                    .filter(|graph| graph.group == g)
                    .all(|graph| graph.style.visible);
                
                let btn_text = if all_visible { "👁" } else { "🚫" };
                let btn = ui.button(btn_text);
                btn.clone().on_hover_text(&format!("Показать/скрыть все графики группы {}", g));
                if btn.clicked() {
                    for graph in app.graphs.iter_mut() {
                        if graph.group == g {
                            graph.style.visible = !graph.style.visible;
                        }
                    }
                    any_changed = true;
                }
            }
        }
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);

    // Режим зума выделением
    let zoom_resp = ui
        .checkbox(&mut app.zoom_mode, "Зум выделением");
    zoom_resp.clone().on_hover_text("Включите режим и выделите участок мышью на холсте для зума");
    if zoom_resp.changed() {
        app.has_unsaved_changes = true;
        if !app.zoom_mode {
            // Сброс выделения при выходе из режима
            app.selection_start = None;
            app.selection_end = None;
        }
    }

    // Кнопка настроек — ниже "Зум выделением", с разделителем
    ui.add_space(8.0);
    ui.separator();
    ui.heading("Настройки");
    let settings_frame = egui::Frame::default()
        .fill(theme.frame_fill.gamma_multiply(0.8))
        .stroke(egui::Stroke::new(1.5, theme.selection_stroke))
        .inner_margin(egui::Margin::symmetric(12, 8));
    settings_frame.show(ui, |ui| {
        let settings_resp = ui.button("⚙️ Настройки");
        settings_resp.clone().on_hover_text("Открыть окно настроек приложения");
        if settings_resp.clicked() {
            app.show_settings_window = true;
        }
    });

    // Диапазон X
    ui.add_space(8.0);
    ui.separator();
    ui.heading("Диапазон X");

    let mut x_min = app.x_min as f32;
    let mut x_max = app.x_max as f32;
    let mut n = app.n_points as i32;

    ui.horizontal(|ui| {
        ui.label("x min:");
        let resp = ui.add(egui::DragValue::new(&mut x_min));
        resp.clone().on_hover_text("Границы отображения по оси X");
        if resp.changed() {
            app.x_min = x_min as f64;
            app.has_unsaved_changes = true;
            app.mark_all_dirty();
            need_save_snapshot = true;
        }
    });
    ui.horizontal(|ui| {
        ui.label("x max:");
        let resp = ui.add(egui::DragValue::new(&mut x_max));
        resp.clone().on_hover_text("Границы отображения по оси X");
        if resp.changed() {
            app.x_max = x_max as f64;
            app.has_unsaved_changes = true;
            app.mark_all_dirty();
            need_save_snapshot = true;
        }
    });
    ui.horizontal(|ui| {
        ui.label("точки:");
        let resp = ui.add(egui::DragValue::new(&mut n).range(10..=10000));
        resp.clone().on_hover_text("Количество точек для отрисовки (больше = плавнее)");
        if resp.changed() {
            app.n_points = n as usize;
            app.has_unsaved_changes = true;
            app.mark_all_dirty();
            need_save_snapshot = true;
        }
    });

    // Диапазон Y
    ui.add_space(8.0);
    ui.separator();
    ui.heading("Диапазон Y");
    let auto_y_resp = ui.checkbox(&mut app.auto_y, "Авто-масштаб Y");
    auto_y_resp.clone().on_hover_text("Автоматически подбирать масштаб по Y");
    if auto_y_resp.changed() {
        app.has_unsaved_changes = true;
        need_save_snapshot = true;
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
        let reset_resp = ui.button("⟲ Сбросить масштаб");
        reset_resp.clone().on_hover_text("Вернуть масштаб и позицию по умолчанию");
        if reset_resp.clicked() {
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
        let auto_y_btn = ui
            .button(if app.auto_y {
                "⟳ Авто Y"
            } else {
                "⟳ Авто Y (выкл)"
            });
        auto_y_btn.clone().on_hover_text("Переключить на автоматический масштаб по Y");
        if auto_y_btn.clicked() {
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
        let help_resp = ui.button("❓ Помощь");
        help_resp.clone().on_hover_text("Открыть справку с описанием команд");
        if help_resp.clicked() {
            app.show_help_window = true;
        }
        let about_resp = ui.button("ℹ️ О программе");
        about_resp.clone().on_hover_text("Показать информацию о программе");
        if about_resp.clicked() {
            app.show_about_window = true;
        }
        let values_resp = ui.button("📊 Таблица");
        values_resp.clone().on_hover_text("Показать таблицу значений для выбранного графика");
        if values_resp.clicked() {
            app.show_values_table = true;
            app.values_table_graph_index = app.selected_graph;
        }
    });

    if need_recompute {
        app.recompute_all();
    }
}

/// Панель экспорта/импорта (CSV, PNG).
fn show_export_ui(app: &mut PlotApp, ctx: &Context, ui: &mut Ui, need_recompute: &mut bool) {
    // Импорт CSV
    let import_resp = ui.button("📥 Импорт CSV");
    import_resp.clone().on_hover_text("Загрузить данные из CSV файла для отображения точек");
    if import_resp.clicked() {
        let path = export::open_csv_dialog();
        if let Some(ref p) = path {
            match export::load_csv(p) {
                Ok(points) => {
                    let count = points.len();
                    let palette = [
                        Color32::from_rgb(150, 255, 150),
                        Color32::from_rgb(255, 255, 100),
                        Color32::from_rgb(200, 100, 255),
                        Color32::from_rgb(100, 255, 200),
                    ];
                    let idx = app.graphs.len();
                    let color = palette[idx % palette.len()];
                    let label = format!("data{}", idx + 1);
                    app.graphs
                        .push(GraphEntry::from_raw_data(&label, color, points));
                    app.selected_graph = Some(app.graphs.len() - 1);
                    app.has_unsaved_changes = true;
                    *need_recompute = true;
                    app.status_msg = Some(StatusMessage::Info(format!(
                        "Импортировано {} точек из {}",
                        count,
                        p.display()
                    )));
                    app.status_time = Some(Instant::now());
                }
                Err(e) => {
                    app.status_msg = Some(StatusMessage::Error(format!("Ошибка импорта: {e}")));
                    app.status_time = Some(Instant::now());
                }
            }
        }
    }

    ui.separator();

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

    let png_btn = ui.button("Сохранить PNG");
    png_btn.clone().on_hover_text("Сохранить скриншот всего окна в PNG");
    if png_btn.clicked() {
        let path = rfd::FileDialog::new()
            .set_file_name("plot.png")
            .add_filter("PNG", &["png"])
            .save_file();

        if let Some(ref p) = path {
            app.save_path = Some(p.clone());
            app.should_capture = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData {
                data: None,
            }));
        }
    }

    ui.separator();

    let png_canvas_btn = ui.button("Сохранить PNG холста");
    png_canvas_btn.clone().on_hover_text("Сохранить только область графика (без панелей)");
    if png_canvas_btn.clicked() {
        let path = rfd::FileDialog::new()
            .set_file_name("plot_canvas.png")
            .add_filter("PNG", &["png"])
            .save_file();

        if let Some(ref p) = path {
            app.save_path = Some(p.clone());
            app.should_capture = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData {
                data: None,
            }));
        }
    }

    ui.separator();

    let svg_btn = ui.button("Сохранить SVG");
    svg_btn.clone().on_hover_text("Экспорт всех видимых графиков в векторный SVG");
    if svg_btn.clicked() {
        let path = rfd::FileDialog::new()
            .set_file_name("plot.svg")
            .add_filter("SVG", &["svg"])
            .save_file();

        if let Some(ref p) = path {
            let theme = app.current_theme.resolve(ui.ctx());
            match svg_renderer::export_svg(
                &app.graphs,
                &app.viewport,
                &theme,
                &app.font_config,
                p,
            ) {
                Ok(()) => {
                    app.status_msg =
                        Some(StatusMessage::Info(format!("SVG сохранён: {}", p.display())));
                    app.status_time = Some(Instant::now());
                }
                Err(e) => {
                    app.status_msg = Some(StatusMessage::Error(format!("Ошибка SVG: {e}")));
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
        ui.label("Ctrl+Z");
        ui.separator();
        ui.label("Отменить");
    });
    ui.horizontal(|ui| {
        ui.label("Ctrl+Shift+Z / Ctrl+Y");
        ui.separator();
        ui.label("Повторить");
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
pub fn render_about_content(ui: &mut Ui, _current_theme: &crate::theme::ThemeKind) {
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
    let save_resp = ui.button("Сохранить сессию");
    save_resp.clone().on_hover_text("Сохранить все графики и настройки в файл");
    if save_resp.clicked() {
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

    let load_resp = ui.button("Загрузить сессию");
    load_resp.clone().on_hover_text("Загрузить сохранённую сессию");
    if load_resp.clicked() {
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
    if ui
        .checkbox(&mut auto_load, "Автозагрузка последней сессии")
        .changed()
    {
        let mut new_config = config;
        new_config.auto_load_last_session = auto_load;
        if let Err(e) = new_config.save() {
            app.status_msg = Some(StatusMessage::Error(format!(
                "Ошибка сохранения настроек: {e}"
            )));
            app.status_time = Some(Instant::now());
        }
    }
}

/// Отрисовка окна настроек.
pub fn show_settings_window(app: &mut PlotApp, ctx: &Context) {
    // Извлекаем значения для сравнения
    let current_theme = app.current_theme.to_string();
    let current_font_family = app.font_config.family.clone();
    let current_font_size = app.font_config.size;
    let current_x_min = app.x_min;
    let current_x_max = app.x_max;

    // Собираем изменения для применения после закрытия окна
    let mut pending_theme: Option<crate::theme::ThemeKind> = None;
    let mut pending_font_family = current_font_family.clone();
    let mut pending_font_size = current_font_size;
    let mut pending_x_min = current_x_min;
    let mut pending_x_max = current_x_max;
    let mut pending_mark_dirty = false;

    egui::Window::new("⚙️ Настройки")
        .resizable(true)
        .open(&mut app.show_settings_window)
        .show(ctx, |ui| {
            ui.heading("Тема");
            let mut selected_theme = current_theme;
            egui::ComboBox::from_id_salt("settings_theme")
                .selected_text(selected_theme)
                .show_ui(ui, |ui| {
                    for theme in crate::theme::ThemeKind::all() {
                        ui.selectable_value(&mut selected_theme, theme.to_string(), theme.to_string());
                    }
                });
            if selected_theme != current_theme {
                if let Some(theme_kind) = crate::theme::ThemeKind::from_string(&selected_theme) {
                    pending_theme = Some(theme_kind);
                }
            }
            ui.separator();

            ui.heading("Шрифт");
            
            // Семейство шрифтов
            let mut font_family = current_font_family.clone();
            ui.horizontal(|ui| {
                ui.label("Семейство:");
                egui::ComboBox::from_id_salt("settings_font_family")
                    .selected_text(font_family.clone())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut font_family, "proportional".to_string(), "Proportional");
                        ui.selectable_value(&mut font_family, "monospace".to_string(), "Monospace");
                        if app.font_config.is_custom() {
                            ui.selectable_value(&mut font_family, "custom".to_string(), "Custom (loaded)");
                        }
                    });
            });
            if font_family != current_font_family {
                pending_font_family = font_family;
                app.has_unsaved_changes = true;
            }

            // Размер шрифта
            let mut font_size = current_font_size;
            ui.horizontal(|ui| {
                ui.label("Размер:");
                ui.add(egui::DragValue::new(&mut font_size).range(8.0..=24.0).speed(0.5));
            });
            if (font_size - current_font_size).abs() > 0.01 {
                pending_font_size = font_size;
                app.has_unsaved_changes = true;
                ui.ctx().request_repaint();
            }

            // Загрузка кастомного шрифта
            ui.horizontal(|ui| {
                let load_btn = ui.button("📂 Загрузить шрифт");
                load_btn.clone().on_hover_text("Выбрать .ttf файл для использования в качестве основного шрифта");
                if load_btn.clicked() {
                    let path = rfd::FileDialog::new()
                        .add_filter("TrueType Font", &["ttf"])
                        .add_filter("All Files", &["*"])
                        .pick_file();
                    if let Some(ref p) = path {
                        match FontConfig::load_custom_font(p) {
                            Ok(custom_font) => {
                                app.font_config = custom_font;
                                app.has_unsaved_changes = true;
                                app.status_msg = Some(StatusMessage::Info(format!(
                                    "Шрифт загружен: {}",
                                    p.file_name().unwrap_or_default().to_string_lossy()
                                )));
                                app.status_time = Some(Instant::now());
                            }
                            Err(e) => {
                                app.status_msg = Some(StatusMessage::Error(format!(
                                    "Ошибка загрузки шрифта: {e}"
                                )));
                                app.status_time = Some(Instant::now());
                            }
                        }
                    }
                }
                
                let reset_btn = ui.button("↺ Сбросить");
                reset_btn.clone().on_hover_text("Вернуть настройки шрифта к значениям по умолчанию");
                if reset_btn.clicked() {
                    app.font_config = FontConfig::default();
                    app.has_unsaved_changes = true;
                    ui.ctx().request_repaint();
                }
            });
            ui.separator();

            ui.heading("Отображение");
            
            // Школьная сетка
            if ui.checkbox(&mut app.school_grid, "Школьная сетка 1:1").changed() {
                app.has_unsaved_changes = true;
            }

            // Полярные координаты
            if ui.checkbox(&mut app.polar_mode, "Полярные координаты").changed() {
                app.has_unsaved_changes = true;
            }

            // Адаптивная плотность
            if ui.checkbox(&mut app.adaptive, "Адаптивная плотность").changed() {
                app.has_unsaved_changes = true;
            }

            // Количество точек
            let mut n = app.n_points as u32;
            ui.horizontal(|ui| {
                ui.label("Точки:");
                ui.add(egui::DragValue::new(&mut n).range(10..=10000));
            });
            if (n as usize) != app.n_points {
                app.n_points = n as usize;
                app.has_unsaved_changes = true;
            }

            // Авто-масштаб Y
            if ui.checkbox(&mut app.auto_y, "Авто-масштаб Y").changed() {
                app.has_unsaved_changes = true;
            }
            ui.separator();

            ui.heading("Диапазон X");
            let mut x_min = current_x_min;
            let mut x_max = current_x_max;
            ui.horizontal(|ui| {
                ui.label("min:");
                ui.add(egui::DragValue::new(&mut x_min));
                ui.label("max:");
                ui.add(egui::DragValue::new(&mut x_max));
            });
            if (x_min - current_x_min).abs() > 0.001 || (x_max - current_x_max).abs() > 0.001 {
                pending_x_min = x_min;
                pending_x_max = x_max;
                pending_mark_dirty = true;
                app.has_unsaved_changes = true;
            }
        });

    // Применяем изменения после закрытия окна
    if let Some(theme_kind) = pending_theme {
        app.set_theme(theme_kind);
    }
    if pending_font_family != current_font_family {
        app.font_config.family = pending_font_family;
    }
    if (pending_font_size - current_font_size).abs() > 0.01 {
        app.font_config.size = pending_font_size;
    }
    if pending_mark_dirty {
        app.x_min = pending_x_min;
        app.x_max = pending_x_max;
        app.viewport.x_min = pending_x_min;
        app.viewport.x_max = pending_x_max;
        app.mark_all_dirty();
    }
}

/// Статус-бар с автоматическим затуханием.
fn show_status_bar(app: &mut PlotApp, ui: &mut Ui) {
    if let Some(ref msg) = app.status_msg {
        let theme = app.current_theme.resolve(ui.ctx());
        let color = match msg {
            StatusMessage::Info(_) => theme.status_info,
            StatusMessage::Error(_) => theme.status_error,
        };
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

/// Отрисовка окна быстрого старта.
pub fn show_quick_start_window(app: &mut PlotApp, ctx: &Context) {
    let mut should_close = false;

    egui::Window::new("🚀 Быстрый старт")
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .resizable(true)
        .open(&mut app.show_quick_start)
        .show(ctx, |ui| {
            ui.heading("Добро пожаловать в Function Plotter!");
            ui.separator();

            ui.heading("1. Как добавить функцию");
            ui.horizontal(|ui| {
                ui.label("1.");
                ui.label("Нажмите \"+ Добавить график\" в левой панели");
            });
            ui.horizontal(|ui| {
                ui.label("2.");
                ui.label("Введите формулу, например:");
                ui.code("sin(x)");
                ui.label(",");
                ui.code("x^2");
                ui.label(",");
                ui.code("sqrt(x)");
            });
            ui.horizontal(|ui| {
                ui.label("3.");
                ui.label("Нажмите Enter или измените диапазон для пересчёта");
            });
            ui.separator();

            ui.heading("2. Как масштабировать график");
            ui.horizontal(|ui| {
                ui.label("•");
                ui.label("Колесико мыши — масштабирование к курсору");
            });
            ui.horizontal(|ui| {
                ui.label("•");
                ui.label("Зажать левую кнопку мыши и тянуть — перемещение");
            });
            ui.horizontal(|ui| {
                ui.label("•");
                ui.label("Включите \"Зум выделением\" и выделите участок для зума");
            });
            ui.horizontal(|ui| {
                ui.label("•");
                ui.label("Кнопка \"⟲ Сбросить масштаб\" — вернуть к начальному виду");
            });
            ui.separator();

            ui.heading("3. Какие функции доступны");
            ui.horizontal(|ui| {
                ui.label("Базовые:");
                ui.code("sin, cos, tan, sqrt, abs, log, exp");
            });
            ui.horizontal(|ui| {
                ui.label("Производные:");
                ui.code("deriv(sin(x))");
            });
            ui.horizontal(|ui| {
                ui.label("Интегралы:");
                ui.code("integral(sin(x), 0, 3.14)");
            });
            ui.horizontal(|ui| {
                ui.label("Полярные:");
                ui.code("polar(cos(x), 0, 2*pi)");
            });
            ui.horizontal(|ui| {
                ui.label("Параметрические:");
                ui.code("parametric(cos(t), sin(t), 0, 2*pi)");
            });
            ui.separator();

            ui.heading("4. Как экспортировать");
            ui.horizontal(|ui| {
                ui.label("•");
                ui.label("\"Сохранить PNG\" — скриншот всего окна");
            });
            ui.horizontal(|ui| {
                ui.label("•");
                ui.label("\"Сохранить PNG холста\" — только область графика");
            });
            ui.horizontal(|ui| {
                ui.label("•");
                ui.label("\"Сохранить SVG\" — векторный экспорт графиков");
            });
            ui.horizontal(|ui| {
                ui.label("•");
                ui.label("\"Сохранить CSV\" — данные в текстовом формате");
            });
            ui.separator();

            ui.heading("Горячие клавиши");
            ui.horizontal(|ui| {
                ui.code("Ctrl+N");
                ui.label("— Новый график");
            });
            ui.horizontal(|ui| {
                ui.code("Ctrl+S");
                ui.label("— Сохранить сессию");
            });
            ui.horizontal(|ui| {
                ui.code("Ctrl+O");
                ui.label("— Загрузить сессию");
            });
            ui.horizontal(|ui| {
                ui.code("F5");
                ui.label("— Пересчитать графики");
            });
            ui.horizontal(|ui| {
                ui.code("Delete");
                ui.label("— Удалить выбранный график");
            });
            ui.horizontal(|ui| {
                ui.code("R");
                ui.label("— Сбросить масштаб");
            });
            ui.separator();

            ui.label("Нажмите \"Понятно\" чтобы закрыть это окно.");
            
            // Чекбокс "Показывать при запуске" — используем отдельное поле
            ui.checkbox(&mut app.quick_start_show_on_start, "Показывать при запуске");
            
            if ui.button("Понятно").clicked() {
                should_close = true;
            }
        });

    if should_close {
        app.show_quick_start = false;
        // Сохраняем состояние чекбокса в config
        let mut config = config::AppConfig::load();
        config.show_quick_start = app.quick_start_show_on_start;
        let _ = config.save();
    }
}
