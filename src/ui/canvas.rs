//! Центральная область: холст для отрисовки графиков, pan/zoom, легенда.

use egui::{pos2, Color32, Context, Rect, Sense, Ui, Vec2};

use crate::app::PlotApp;
use crate::renderer;

/// Отрисовка центральной панели (canvas).
pub fn show_canvas_panel(app: &mut PlotApp, ctx: &Context, ui: &mut Ui) {
    // Получаем реальный размер экрана и вычитаем левую панель
    let screen_rect = ctx
        .input(|i| i.raw.screen_rect)
        .unwrap_or_else(|| egui::Rect::NOTHING);
    let left = ui.max_rect().left();
    let rect = Rect::from_x_y_ranges(left..=screen_rect.right(), screen_rect.y_range());
    let response = ui.allocate_rect(rect, Sense::click_and_drag());
    ui.advance_cursor_after_rect(rect);

    // Отслеживаем изменение размера и пересчитываем графики
    if let Some(prev) = app._last_canvas_rect {
        if prev.size() != rect.size() {
            app.mark_all_dirty();
            ctx.request_repaint();
            // Сохраняем новый размер окна
            app.window_size = [rect.width(), rect.height()];
            app.save_window_size();
        }
    }
    app._last_canvas_rect = Some(rect);

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
    painter.rect_filled(rect, 0.0, Color32::from_rgb(30, 30, 35));

    if app.polar_mode {
        renderer::draw_polar_grid(&painter, rect, &app.viewport);
    } else {
        renderer::draw_axes(&painter, rect, &app.viewport);
    }

    for g in &app.graphs {
        if let Some(ref data) = g.data {
            if let Some(ref parsed) = g.parsed {
                let formula_type = parsed.formula_type();
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
                    );
                }
            }
        }
    }

    // Легенда
    draw_legend(&painter, rect, &app.graphs);

    // Точки пересечения
    draw_intersections(&painter, rect, &app.viewport, &app.graphs);

    // Координаты мыши
    draw_mouse_coords(ui, &painter, rect, &app.viewport);
}

/// Отрисовка легенды (список видимых графиков).
fn draw_legend(painter: &egui::Painter, rect: Rect, graphs: &[crate::app::GraphEntry]) {
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
                Color32::from_gray(200),
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
                Color32::from_gray(200),
            );
        }
    }
}

/// Отрисовка точек пересечения графиков.
fn draw_intersections(
    painter: &egui::Painter,
    rect: Rect,
    viewport: &crate::renderer::Viewport,
    graphs: &[crate::app::GraphEntry],
) {
    let visible: Vec<&crate::app::GraphEntry> = graphs
        .iter()
        .filter(|g| g.style.visible && g.data.is_some())
        .collect();

    if visible.len() < 2 {
        return;
    }

    let mut found: Vec<(f64, f64)> = Vec::new();

    for i in 0..visible.len() {
        for j in (i + 1)..visible.len() {
            let data_a = visible[i].data.as_ref().unwrap();
            let data_b = visible[j].data.as_ref().unwrap();

            let min_len = data_a.points.len().min(data_b.points.len());
            for k in 1..min_len {
                let (xa_a, ya_a) = data_a.points[k - 1];
                let (_xa_b, yb_a) = data_b.points[k - 1];
                let (xa_c, ya_c) = data_a.points[k];
                let (_xa_d, yb_c) = data_b.points[k];

                let diff_prev = ya_a - yb_a;
                let diff_curr = ya_c - yb_c;

                if diff_prev * diff_curr < 0.0 {
                    let dx = xa_c - xa_a;
                    if dx.abs() > 1e-15 {
                        let t = diff_prev / (diff_prev - diff_curr);
                        let ix = xa_a + t * dx;
                        let slope_a = (ya_c - ya_a) / dx;
                        let slope_b = (yb_c - yb_a) / dx;
                        let iy_a = ya_a + slope_a * (ix - xa_a);
                        let iy_b = yb_a + slope_b * (ix - xa_a);
                        let iy = (iy_a + iy_b) / 2.0;

                        if ix.is_finite()
                            && iy.is_finite()
                            && ix >= viewport.x_min
                            && ix <= viewport.x_max
                            && iy >= viewport.y_min
                            && iy <= viewport.y_max
                        {
                            // Проверяем дубликаты
                            let is_dup = found.iter().any(|(fx, fy)| {
                                (fx - ix).abs() < 0.01 * (viewport.x_max - viewport.x_min)
                                    && (fy - iy).abs() < 0.01 * (viewport.y_max - viewport.y_min)
                            });
                            if !is_dup {
                                found.push((ix, iy));
                            }
                        }
                    }
                }
            }
        }
    }

    // Отрисовка всех найденных точек
    for (ix, iy) in &found {
        let screen_pos = viewport.math_to_screen(*ix, *iy, rect);
        // Маленькая белая точка с обводкой (радиус 2.5)
        painter.circle_filled(screen_pos, 2.5, egui::Color32::WHITE);
        painter.circle_stroke(
            screen_pos,
            2.5,
            egui::Stroke::new(1.0_f32, egui::Color32::BLACK),
        );
        // Подпись с координатами
        let label = format!("({:.3}, {:.3})", ix, iy);
        painter.text(
            screen_pos + egui::vec2(10.0, -8.0),
            egui::Align2::LEFT_TOP,
            label,
            egui::FontId::monospace(10.0),
            Color32::from_rgb(255, 255, 100),
        );
    }
}
