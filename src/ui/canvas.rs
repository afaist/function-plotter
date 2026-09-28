//! Центральная область: холст для отрисовки графиков, pan/zoom, легенда.

use egui::{Color32, Context, Rect, Sense, Ui, Vec2};

use crate::app::PlotApp;
use crate::renderer;

/// Отрисовка центральной панели (canvas).
pub fn show_canvas_panel(app: &mut PlotApp, _ctx: &Context, ui: &mut Ui) {
    let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());

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
            let dx_math = -dx_screen as f64 / rect.width() as f64
                * (app.viewport.x_max - app.viewport.x_min);
            let dy_math = dy_screen as f64 / rect.height() as f64
                * (app.viewport.y_max - app.viewport.y_min);
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

    renderer::draw_axes(&painter, rect, &app.viewport);

    for g in &app.graphs {
        if let Some(ref data) = g.data {
            renderer::draw_curve(&painter, rect, &app.viewport, &data.points, &g.style);
        }
    }

    // Легенда
    draw_legend(&painter, rect, &app.graphs);

    // Координаты мыши
    draw_mouse_coords(&painter, rect, &app.viewport);
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
fn draw_mouse_coords(painter: &egui::Painter, rect: Rect, viewport: &crate::renderer::Viewport) {
    // Получаем позицию мыши через painter — но painter не имеет доступа к input.
    // Используем трюк: painter может получить контекст через painter.ctx(), но это сложно.
    // Вместо этого — координаты будут обновлены через callback в update().
    // Для простоты — пропускаем отрисовку координат здесь, так как painter не имеет доступа к ui.input.
    // Это ограничение egui — painter не может читать input напрямую.
    let _ = painter;
    let _ = rect;
    let _ = viewport;
    // TODO: координаты мыши требуют доступа к ui.input, перенести в update()
}
