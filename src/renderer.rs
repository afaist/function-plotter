use egui::{Color32, Painter, Pos2, Rect, Stroke, Vec2};

/// Настройки отображения для одного графика.
#[derive(Clone)]
pub struct PlotStyle {
    pub color: Color32,
    pub label: String,
    pub visible: bool,
}

/// Параметры масштабирования, управляются пользователем.
#[derive(Clone, Debug)]
pub struct Viewport {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
}

impl Viewport {
    pub fn new() -> Self {
        Self {
            x_min: -10.0,
            x_max: 10.0,
            y_min: -5.0,
            y_max: 5.0,
        }
    }

    /// Масштабирование (zoom) с центром в точке экрана.
    pub fn zoom(&mut self, factor: f64, center_screen: Vec2, canvas_rect: Rect) {
        // Конвертируем Vec2 -> Pos2 (они просто разные типы с одинаковыми полями x, y)
        let center_pos = Pos2::new(center_screen.x, center_screen.y);

        let (cx, cy) = self.screen_to_math(center_pos, canvas_rect);

        self.x_min = cx - (cx - self.x_min) * factor;
        self.x_max = cx + (self.x_max - cx) * factor;
        self.y_min = cy - (cy - self.y_min) * factor;
        self.y_max = cy + (self.y_max - cy) * factor;
    }

    /// Сдвиг (pan) в математических координатах.
    pub fn pan(&mut self, dx_math: f64, dy_math: f64) {
        self.x_min += dx_math;
        self.x_max += dx_math;
        self.y_min += dy_math;
        self.y_max += dy_math;
    }

    /// Преобразование математических координат в экранные.
    pub fn math_to_screen(&self, x: f64, y: f64, rect: Rect) -> Pos2 {
        let px = ((x - self.x_min) / (self.x_max - self.x_min)) * rect.width() as f64;
        let py = ((self.y_max - y) / (self.y_max - self.y_min)) * rect.height() as f64;
        Pos2::new(rect.left() + px as f32, rect.top() + py as f32)
    }

    /// Преобразование экранных координат в математические.
    /// Возвращает (x, y) в f64.
    pub fn screen_to_math(&self, pos: Pos2, rect: Rect) -> (f64, f64) {
        let x =
            self.x_min + ((pos.x - rect.left()) / rect.width()) as f64 * (self.x_max - self.x_min);
        let y =
            self.y_max - ((pos.y - rect.top()) / rect.height()) as f64 * (self.y_max - self.y_min);
        (x, y)
    }
}

/// Отрисовка одного графика по точкам.
pub fn draw_curve(
    painter: &Painter,
    rect: Rect,
    viewport: &Viewport,
    points: &[(f64, f64)],
    style: &PlotStyle,
) {
    if !style.visible {
        return;
    }

    let stroke = Stroke::new(1.5_f32, style.color);
    let mut screen_points: Vec<Pos2> = Vec::with_capacity(points.len());

    for &(x, y) in points {
        if !y.is_finite() {
            // Разрыв линии — отрисовываем накопленный сегмент
            if screen_points.len() >= 2 {
                painter.add(egui::Shape::line(screen_points.clone(), stroke));
            }
            screen_points.clear();
            continue;
        }
        screen_points.push(viewport.math_to_screen(x, y, rect));
    }

    if screen_points.len() >= 2 {
        painter.add(egui::Shape::line(screen_points, stroke));
    }
}

/// Отрисовка осей координат и сетки.
pub fn draw_axes(painter: &Painter, rect: Rect, viewport: &Viewport) {
    let axis_color = Color32::from_gray(120);
    let grid_color = Color32::from_gray(40);
    let text_color = Color32::from_gray(180);

    // Базовое число делений — можно менять, если хочется больше/меньше линий
    let base_grid_count = 8;

    let width = viewport.x_max - viewport.x_min;
    let height = viewport.y_max - viewport.y_min;

    // Адаптивный шаг по X и Y: хотим примерно base_grid_count делений
    let step_x = width / base_grid_count as f64;
    let step_y = height / base_grid_count as f64;

    //
    // Подбираем «красивый» шаг (округлённый к 1, 2, 5, 10 и т.п.)
    fn nice_step(step: f64) -> f64 {
        if step == 0.0 {
            return 1.0;
        }
        let mag = 10.0_f64.powf(step.abs().log10().floor());
        let unit = step / mag;

        let candidates = [1.0, 2.0, 5.0];
        let target = unit;

        let best = candidates
            .iter()
            .min_by(|a, b| {
                // Разыменовываем ссылки: *a и *b
                let diff_a = (*a - target).abs();
                let diff_b = (*b - target).abs();
                diff_a
                    .partial_cmp(&diff_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied()
            .unwrap_or(1.0);

        best * mag
    }

    let nice_step_x = nice_step(step_x);
    let nice_step_y = nice_step(step_y);

    // Количество делений на основе красивого шага
    let n_grid_x = ((width / nice_step_x).abs() * 1.1).ceil() as usize;
    let n_grid_y = ((height / nice_step_y).abs() * 1.1).ceil() as usize;

    let grid_stroke = Stroke::new(0.5_f32, grid_color);
    let axis_stroke = Stroke::new(1.5_f32, axis_color);

    // Сетка по X (вертикальные линии)
    for i in 0..=n_grid_x {
        let x = viewport.x_min + nice_step_x * i as f64;
        let p1 = viewport.math_to_screen(x, viewport.y_min, rect);
        let p2 = viewport.math_to_screen(x, viewport.y_max, rect);
        painter.line_segment([p1, p2], grid_stroke);
    }

    // Сетка по Y (горизонтальные линии)
    for i in 0..=n_grid_y {
        let y = viewport.y_min + nice_step_y * i as f64;
        let p1 = viewport.math_to_screen(viewport.x_min, y, rect);
        let p2 = viewport.math_to_screen(viewport.x_max, y, rect);
        painter.line_segment([p1, p2], grid_stroke);
    }

    // Ось X (y = 0)
    if viewport.y_min <= 0.0 && viewport.y_max >= 0.0 {
        let p1 = viewport.math_to_screen(viewport.x_min, 0.0, rect);
        let p2 = viewport.math_to_screen(viewport.x_max, 0.0, rect);
        painter.line_segment([p1, p2], axis_stroke);
    }
    // Ось Y (x = 0)
    if viewport.x_min <= 0.0 && viewport.x_max >= 0.0 {
        let p1 = viewport.math_to_screen(0.0, viewport.y_min, rect);
        let p2 = viewport.math_to_screen(0.0, viewport.y_max, rect);
        painter.line_segment([p1, p2], axis_stroke);
    }

    // Подписи делений: адаптивная точность
    let font = egui::FontId::proportional(11.0);

    fn format_value(val: f64, step: f64) -> String {
        // Определяем, сколько знаков после запятой нужно, чтобы различать соседние метки
        if step <= 0.0001 {
            format!("{val:.6}")
        } else if step <= 0.001 {
            format!("{val:.5}")
        } else if step <= 0.01 {
            format!("{val:.4}")
        } else if step <= 0.1 {
            format!("{val:.3}")
        } else if step <= 1.0 {
            format!("{val:.2}")
        } else {
            format!("{val:.1}")
        }
    }

    // Метки по X
    for i in 0..=n_grid_x {
        let x = viewport.x_min + nice_step_x * i as f64;
        if x >= viewport.x_min && x <= viewport.x_max {
            let p = viewport.math_to_screen(x, 0.0, rect);
            let label = format_value(x, nice_step_x);
            painter.text(
                p + Vec2::new(2.0, 2.0),
                egui::Align2::LEFT_TOP,
                label,
                font.clone(),
                text_color,
            );
        }
    }

    // Метки по Y
    for i in 0..=n_grid_y {
        let y = viewport.y_min + nice_step_y * i as f64;
        if y >= viewport.y_min && y <= viewport.y_max {
            let p = viewport.math_to_screen(0.0, y, rect);
            let label = format_value(y, nice_step_y);
            // Смещение влево, выравнивание по правому краю, чтобы не наезжало на ось
            painter.text(
                p + Vec2::new(-4.0, -10.0),
                egui::Align2::RIGHT_TOP,
                label,
                font.clone(),
                text_color,
            );
        }
    }
}
/// Выбрать количество знаков после запятой в зависимости от масштаба (шага сетки).
pub fn format_coord(val: f64, step: f64) -> String {
    if step <= 0.0001 {
        format!("{val:.6}")
    } else if step <= 0.001 {
        format!("{val:.5}")
    } else if step <= 0.01 {
        format!("{val:.4}")
    } else if step <= 0.1 {
        format!("{val:.3}")
    } else if step <= 1.0 {
        format!("{val:.2}")
    } else {
        format!("{val:.1}")
    }
}
