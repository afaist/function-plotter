//! Центральная область: холст для отрисовки графиков, pan/zoom, легенда.

use egui::{pos2, Context, Rect, Sense, Stroke, Ui, Vec2};

use crate::app::PlotApp;
use crate::renderer;

/// Отрисовка центральной панели (canvas).
pub fn show_canvas_panel(app: &mut PlotApp, ctx: &Context, ui: &mut Ui) {
    // Получаем реальный размер экрана и вычитаем левую панель
    let screen_rect = ctx
        .input(|i| i.raw.screen_rect)
        .unwrap_or(egui::Rect::NOTHING);
    let left = ui.max_rect().left();
    let rect = Rect::from_x_y_ranges(left..=screen_rect.right(), screen_rect.y_range());
    let response = ui.allocate_rect(rect, Sense::click_and_drag());
    ui.advance_cursor_after_rect(rect);

    // Отслеживаем изменение размера и пересчитываем графики
    if let Some(prev) = app.last_canvas_rect {
        if prev.size() != rect.size() {
            app.mark_all_dirty();
            ctx.request_repaint();
            // Сохраняем новый размер окна
            app.window_size = [rect.width(), rect.height()];
            app.save_window_size();
        }
    }
    app.last_canvas_rect = Some(rect);
    // Сохраняем координаты холста для скриншота
    app.graph_rect = Some(rect);

    // Масштабирование (колесо мыши)
    if let Some(pos) = response.hover_pos() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            let factor = if scroll > 0.0 { 0.9 } else { 1.1 };
            app.viewport.zoom(factor, pos.to_vec2(), rect);
            app.mark_all_dirty();
        }
    }

    // Перетаскивание (pan)
    if response.dragged() {
        if let Some(start) = app.drag_start {
            let current = response.interact_pointer_pos().unwrap_or(start);
            let dx_screen = current.x - start.x;
            let dy_screen = current.y - start.y;
            let dx_math =
                -dx_screen as f64 / rect.width() as f64 * (app.viewport.x_max - app.viewport.x_min);
            let dy_math =
                dy_screen as f64 / rect.height() as f64 * (app.viewport.y_max - app.viewport.y_min);
            app.viewport.pan(dx_math, dy_math);
            app.mark_all_dirty();
            app.drag_start = Some(current);
        } else {
            app.drag_start = response.interact_pointer_pos();
        }
    } else {
        app.drag_start = None;
    }

    // Синхронизация x_min/x_max с viewport
    app.x_min = app.viewport.x_min;
    app.x_max = app.viewport.x_max;

    // Отрисовка
    let painter = ui.painter_at(rect);
    let theme = app.current_theme.resolve(ui.ctx());
    painter.rect_filled(rect, 0.0, theme.canvas_bg);

    if app.polar_mode {
        renderer::draw_polar_grid(&painter, rect, &app.viewport, &theme);
    } else {
        renderer::draw_axes(&painter, rect, &app.viewport, &theme);
    }

    for g in &app.graphs {
        if let Some(ref data) = g.data {
            // Для графиков из CSV (без parsed) используем Regular тип
            let formula_type = g.parsed.as_ref().map(|p| p.formula_type()).unwrap_or(crate::parser::FormulaType::Regular);
            renderer::draw_curve_with_type(
                &painter,
                rect,
                &app.viewport,
                &data.points,
                &g.style,
                formula_type,
            );
            // Для интегралов — закрашенная область
            if !data.fill_points.is_empty() {
                renderer::draw_filled_curve(
                    &painter,
                    rect,
                    &app.viewport,
                    &data.fill_points,
                    &g.style,
                    &theme,
                );
            }
        }
    }

    // Вершина и корни для каждого графика
    for g in &app.graphs {
        if g.style.visible {
            if let Some(ref data) = g.data {
                crate::renderer::draw_special_points(
                    &painter,
                    rect,
                    &app.viewport,
                    data.vertex,
                    &data.roots,
                    &g.style,
                    &theme,
                );
            }
        }
    }

    // Школьная сетка 1:1
    if app.school_grid {
        draw_school_grid(&painter, rect, &app.viewport, &theme);
    }

    // Легенда
    draw_legend(&painter, rect, &app.graphs, &theme);

    // Точки пересечения
    draw_intersections(&painter, rect, &app.viewport, &app.find_intersections(), &theme);

    // Координаты мыши
    draw_mouse_coords(ui, &painter, rect, &app.viewport, &theme);
}

/// Отрисовка школьной сетки 1:1 (клетка = 1 единица).
fn draw_school_grid(painter: &egui::Painter, rect: Rect, viewport: &crate::renderer::Viewport, theme: &crate::theme::Theme) {
    let grid_color = theme.grid_color;
    let stroke = Stroke::new(0.5_f32, grid_color);

    // Определяем границы сетки
    let x_min = viewport.x_min.floor() as i32;
    let x_max = viewport.x_max.ceil() as i32;
    let y_min = viewport.y_min.floor() as i32;
    let y_max = viewport.y_max.ceil() as i32;

    // Вертикальные линии
    for x in x_min..=x_max {
        let start = viewport.math_to_screen(x as f64, viewport.y_min, rect);
        let end = viewport.math_to_screen(x as f64, viewport.y_max, rect);
        painter.line_segment([start, end], stroke);
    }

    // Горизонтальные линии
    for y in y_min..=y_max {
        let start = viewport.math_to_screen(viewport.x_min, y as f64, rect);
        let end = viewport.math_to_screen(viewport.x_max, y as f64, rect);
        painter.line_segment([start, end], stroke);
    }

    // Подписи осей
    let text_color = theme.text_secondary;
    for x in x_min..=x_max {
        if x % 5 == 0 && x != 0 {
            let pos = viewport.math_to_screen(x as f64, 0.0, rect);
            if rect.contains(pos) {
                painter.text(
                    pos + Vec2::new(2.0, 12.0),
                    egui::Align2::LEFT_BOTTOM,
                    format!("{}", x),
                    egui::FontId::monospace(9.0),
                    text_color,
                );
            }
        }
    }
    for y in y_min..=y_max {
        if y % 5 == 0 && y != 0 {
            let pos = viewport.math_to_screen(0.0, y as f64, rect);
            if rect.contains(pos) {
                painter.text(
                    pos + Vec2::new(-8.0, 4.0),
                    egui::Align2::RIGHT_CENTER,
                    format!("{}", y),
                    egui::FontId::monospace(9.0),
                    text_color,
                );
            }
        }
    }
}

/// Отрисовка легенды (список видимых графиков).
fn draw_legend(painter: &egui::Painter, rect: Rect, graphs: &[crate::app::GraphEntry], theme: &crate::theme::Theme) {
    let mut legend_y = rect.top() + 8.0;
    for g in graphs {
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
                theme.text_primary,
            );
            legend_y += 18.0;
        }
    }
}

/// Отрисовка координат курсора мыши.
fn draw_mouse_coords(
    ui: &Ui,
    painter: &egui::Painter,
    rect: Rect,
    viewport: &crate::renderer::Viewport,
    theme: &crate::theme::Theme,
) {
    if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
        if hover_pos.x >= rect.left()
            && hover_pos.x <= rect.right()
            && hover_pos.y >= rect.top()
            && hover_pos.y <= rect.bottom()
        {
            let (math_x, math_y) = viewport.screen_to_math(hover_pos, rect);
            let label = format!("{:.3}, {:.3}", math_x, math_y);
            painter.text(
                pos2(rect.left() + 8.0, rect.bottom() - 24.0),
                egui::Align2::LEFT_TOP,
                label,
                egui::FontId::monospace(12.0),
                theme.text_primary,
            );
        }
    }
}

/// Отрисовка точек пересечения графиков.
fn draw_intersections(
    painter: &egui::Painter,
    rect: Rect,
    viewport: &crate::renderer::Viewport,
    intersections: &[crate::app::IntersectionPoint],
    theme: &crate::theme::Theme,
) {
    // Отрисовка всех найденных точек
    for point in intersections {
        let screen_pos = viewport.math_to_screen(point.x, point.y, rect);
        // Маленькая точка с обводкой (радиус 2.5)
        painter.circle_filled(screen_pos, 2.5, theme.intersection_dot);
        painter.circle_stroke(
            screen_pos,
            2.5,
            egui::Stroke::new(1.0_f32, egui::Color32::BLACK),
        );
        // Подпись с координатами
        let label = format!("({:.3}, {:.3})", point.x, point.y);
        painter.text(
            screen_pos + egui::vec2(10.0, -8.0),
            egui::Align2::LEFT_TOP,
            label,
            egui::FontId::monospace(10.0),
            theme.intersection_label_bg,
        );
    }
}
