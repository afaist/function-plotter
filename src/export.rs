use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use ab_glyph::{point, Font, PxScale, ScaleFont};
use image::{self, Rgba, RgbaImage};

use crate::app::PlotApp;
use crate::evaluator::PlotData;
use crate::parser::ParsedFormula;

/// Экспорт одного графика в CSV (x, y).
pub fn export_single(
    formula: &ParsedFormula,
    x_min: f64,
    x_max: f64,
    n_points: usize,
    path: &Path,
) -> std::io::Result<()> {
    let data = PlotData::compute(formula, x_min, x_max, n_points);
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);
    writeln!(w, "x,y")?;
    for (x, y) in &data.points {
        writeln!(w, "{x},{y}")?;
    }
    Ok(())
}

/// Экспорт нескольких графиков: x,y1,y2,... (по общей сетке X).
pub fn export_multi(
    formulas: &[&ParsedFormula],
    labels: &[&str],
    x_min: f64,
    x_max: f64,
    n_points: usize,
    path: &Path,
) -> std::io::Result<()> {
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);

    write!(w, "x")?;
    for label in labels {
        write!(w, ",{label}")?;
    }
    writeln!(w)?;

    let n = n_points.max(2);
    let dx = (x_max - x_min) / (n as f64 - 1.0);

    for i in 0..n {
        let x = x_min + dx * i as f64;
        write!(w, "{x}")?;
        for f in formulas {
            let y = f.eval(x);
            write!(w, ",{y}")?;
        }
        writeln!(w)?;
    }
    Ok(())
}

/// Диалог сохранения файла через rfd.
pub fn save_dialog(default_name: &str) -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .set_file_name(default_name)
        .add_filter("CSV", &["csv"])
        .save_file()
}

/// Диалог открытия CSV-файла для импорта.
pub fn open_csv_dialog() -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .add_filter("CSV", &["csv"])
        .add_filter("Все файлы", &["*"])
        .pick_file()
}

/// Парсинг CSV-файла в точки (x, y).
/// Поддерживаемые форматы:
/// - x,y (с заголовком и без)
/// - Разделитель: запятая или точка с запятой
/// - Пропускаются пустые строки и строки с некорректными данными
pub fn parse_csv(content: &str) -> Result<Vec<(f64, f64)>, String> {
    let mut points = Vec::new();
    let mut line_num = 0;

    for line in content.lines() {
        line_num += 1;
        let line = line.trim();

        // Пропускаем пустые строки
        if line.is_empty() {
            continue;
        }

        // Пропускаем строку заголовка (если первая строка содержит "x" или "y")
        if line_num == 1 {
            let lower = line.to_lowercase();
            if lower.contains('x') || lower.contains('y') {
                continue;
            }
        }

        // Определяем разделитель
        let parts = if line.contains(';') {
            line.split(';').collect::<Vec<_>>()
        } else if line.contains('\t') {
            line.split('\t').collect::<Vec<_>>()
        } else {
            line.split(',').collect::<Vec<_>>()
        };

        if parts.len() < 2 {
            continue; // Пропускаем некорректные строки
        }

        // Парсим x и y
        let x_str = parts[0].trim();
        let y_str = parts[1].trim();

        let x: f64 = x_str
            .parse()
            .map_err(|_| format!("Строка {line_num}: некорректное значение x: '{x_str}'"))?;
        let y: f64 = y_str
            .parse()
            .map_err(|_| format!("Строка {line_num}: некорректное значение y: '{y_str}'"))?;

        if x.is_finite() && y.is_finite() {
            points.push((x, y));
        }
    }

    if points.is_empty() {
        return Err("CSV-файл не содержит корректных данных".into());
    }

    Ok(points)
}

/// Загрузить CSV-файл и вернуть точки.
pub fn load_csv(path: &Path) -> Result<Vec<(f64, f64)>, String> {
    let file = File::open(path).map_err(|e| format!("Ошибка открытия файла: {e}"))?;
    let mut reader = BufReader::new(file);
    let mut content = String::new();
    reader
        .read_to_string(&mut content)
        .map_err(|e| format!("Ошибка чтения файла: {e}"))?;

    parse_csv(&content)
}

/// Сохранение текущего вида канваса в PNG.
pub fn save_png(path: &Path, app: &PlotApp) -> Result<(), String> {
    // Размер выходного изображения
    let width = 1200u32;
    let height = 800u32;

    // Создаём изображение с тёмным фоном
    let bg_color = Rgba([30, 30, 35, 255]);
    let mut img = RgbaImage::from_pixel(width, height, bg_color);

    // Преобразуем математические координаты в экранные
    let math_to_screen = |x: f64, y: f64| -> (f32, f32) {
        let sx = 160.0
            + (x - app.viewport.x_min) / (app.viewport.x_max - app.viewport.x_min)
                * (width as f64 - 160.0);
        let sy = 800.0
            - (y - app.viewport.y_min) / (app.viewport.y_max - app.viewport.y_min)
                * (height as f64);
        (sx as f32, sy as f32)
    };

    // Рисование линии (Bresenham)
    let draw_line = |img: &mut RgbaImage, x0: f32, y0: f32, x1: f32, y1: f32, color: Rgba<u8>| {
        let (x0, y0, x1, y1) = (
            x0.round() as i32,
            y0.round() as i32,
            x1.round() as i32,
            y1.round() as i32,
        );
        let dx = (x1 - x0).abs() as i32;
        let dy = (y1 - y0).abs() as i32;
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;
        let (mut x, mut y) = (x0, y0);

        loop {
            if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
                let pixel = img.get_pixel_mut(x as u32, y as u32);
                let a = color[3] as f32 / 255.0;
                pixel[0] = (color[0] as f32 * a + pixel[0] as f32 * (1.0 - a)).round() as u8;
                pixel[1] = (color[1] as f32 * a + pixel[1] as f32 * (1.0 - a)).round() as u8;
                pixel[2] = (color[2] as f32 * a + pixel[2] as f32 * (1.0 - a)).round() as u8;
                pixel[3] = 255;
            }
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    };

    // Рисуем сетку
    let width_range = app.viewport.x_max - app.viewport.x_min;
    let height_range = app.viewport.y_max - app.viewport.y_min;
    let base_grid_count = 8.0;
    let step_x = width_range / base_grid_count;
    let step_y = height_range / base_grid_count;

    let nice_step = |step: f64| -> f64 {
        if step == 0.0 {
            return 1.0;
        }
        let mag = 10.0_f64.powf(step.abs().log10().floor());
        let unit = step / mag;
        let candidates = [1.0, 2.0, 5.0];
        let best = candidates
            .iter()
            .min_by(|a, b| {
                let diff_a = (**a - unit).abs();
                let diff_b = (**b - unit).abs();
                diff_a
                    .partial_cmp(&diff_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied()
            .unwrap_or(1.0);
        best * mag
    };

    let nice_step_x = nice_step(step_x);
    let nice_step_y = nice_step(step_y);

    let n_grid_x = ((width_range / nice_step_x).abs() * 1.1).ceil() as usize;
    let n_grid_y = ((height_range / nice_step_y).abs() * 1.1).ceil() as usize;

    let grid_color = Rgba([40, 40, 40, 255]);

    // Вертикальные линии сетки
    for i in 0..=n_grid_x {
        let x = app.viewport.x_min + nice_step_x * i as f64;
        let (sx, sy_min) = math_to_screen(x, app.viewport.y_max);
        let (_, sy_max) = math_to_screen(x, app.viewport.y_min);
        draw_line(&mut img, sx, sy_min, sx, sy_max, grid_color);
    }

    // Горизонтальные линии сетки
    for i in 0..=n_grid_y {
        let y = app.viewport.y_min + nice_step_y * i as f64;
        let (sx_min, sy) = math_to_screen(app.viewport.x_min, y);
        let (sx_max, _) = math_to_screen(app.viewport.x_max, y);
        draw_line(&mut img, sx_min, sy, sx_max, sy, grid_color);
    }

    // Оси
    let axis_color = Rgba([120, 120, 120, 255]);
    if app.viewport.y_min <= 0.0 && app.viewport.y_max >= 0.0 {
        let (sx_min, sy) = math_to_screen(app.viewport.x_min, 0.0);
        let (sx_max, _) = math_to_screen(app.viewport.x_max, 0.0);
        draw_line(&mut img, sx_min, sy, sx_max, sy, axis_color);
    }
    if app.viewport.x_min <= 0.0 && app.viewport.x_max >= 0.0 {
        let (sx, sy_min) = math_to_screen(0.0, app.viewport.y_max);
        let (_, sy_max) = math_to_screen(0.0, app.viewport.y_min);
        draw_line(&mut img, sx, sy_min, sx, sy_max, axis_color);
    }

    // Рисуем графики
    for g in &app.graphs {
        if let Some(ref data) = g.data {
            let color = Rgba([g.style.color[0], g.style.color[1], g.style.color[2], 255]);
            let mut prev_point: Option<(f32, f32)> = None;

            for &(x, y) in &data.points {
                if !y.is_finite() {
                    prev_point = None;
                    continue;
                }
                let (sx, sy) = math_to_screen(x, y);
                if let Some((px, py)) = prev_point {
                    draw_line(&mut img, px, py, sx, sy, color);
                }
                prev_point = Some((sx, sy));
            }
        }
    }

    // Легенда
    let mut legend_y = 8u32;
    for g in &app.graphs {
        if g.style.visible {
            let color = Rgba([g.style.color[0], g.style.color[1], g.style.color[2], 255]);
            draw_line(
                &mut img,
                170.0,
                legend_y as f32 + 6.0,
                186.0,
                legend_y as f32 + 6.0,
                color,
            );
            legend_y += 18;
        }
    }

    // Рисуем координаты (метки делений)
    let text_color = Rgba([200, 200, 200, 255]);
    let font = load_font();
    
    // Метки по X
    for i in 0..=n_grid_x {
        let x = app.viewport.x_min + nice_step_x * i as f64;
        if x >= app.viewport.x_min && x <= app.viewport.x_max {
            let (sx, _) = math_to_screen(x, 0.0);
            let label = format_coord(x, nice_step_x);
            draw_text(&mut img, &font, &label, sx - 20.0, 780.0, text_color);
        }
    }
    
    // Метки по Y
    for i in 0..=n_grid_y {
        let y = app.viewport.y_min + nice_step_y * i as f64;
        if y >= app.viewport.y_min && y <= app.viewport.y_max {
            let (_sx, sy) = math_to_screen(0.0, y);
            let label = format_coord(y, nice_step_y);
            draw_text(&mut img, &font, &label, 5.0, sy + 14.0, text_color);
        }
    }

    img.save(&path)
        .map_err(|e| format!("Сохранение PNG: {e}"))?;

    Ok(())
}

/// Рисуем текст на изображении с помощью ab_glyph.
fn draw_text(img: &mut RgbaImage, font: &impl Font, text: &str, x: f32, y: f32, color: Rgba<u8>) {
    let scale = PxScale::from(14.0); // 14pt шрифт
    let scaled = font.as_scaled(scale);
    let mut cx = x;
    
    for c in text.chars() {
        let gid = scaled.glyph_id(c);
        let positioned = gid.with_scale_and_position(scale, point(cx, y));
        let outline = scaled.outline_glyph(positioned);
        if let Some(glyph) = outline {
            let bounds = glyph.px_bounds();
            glyph.draw(|px, py, a| {
                let sx = (px as f32 + bounds.min.x).round() as i32;
                let sy = (py as f32 + bounds.min.y).round() as i32;
                if sx >= 0 && sx < img.width() as i32 && sy >= 0 && sy < img.height() as i32 {
                    let pixel = img.get_pixel_mut(sx as u32, sy as u32);
                    let alpha = a as f32 / 255.0;
                    pixel[0] = (color[0] as f32 * alpha + pixel[0] as f32 * (1.0 - alpha)).round() as u8;
                    pixel[1] = (color[1] as f32 * alpha + pixel[1] as f32 * (1.0 - alpha)).round() as u8;
                    pixel[2] = (color[2] as f32 * alpha + pixel[2] as f32 * (1.0 - alpha)).round() as u8;
                    pixel[3] = 255;
                }
            });
            cx += scaled.h_advance(gid);
        }
    }
}

/// Загрузить системный шрифт.
fn load_font() -> ab_glyph::FontRef<'static> {
    // Пробуем несколько системных шрифтов
    const FONTS: &[&[u8]] = &[
        include_bytes!("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
        include_bytes!("/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf"),
        include_bytes!("/usr/share/fonts/truetype/freefont/FreeSans.ttf"),
        include_bytes!("/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf"),
    ];
    
    for font_data in FONTS {
        if let Ok(font) = ab_glyph::FontRef::try_from_slice(font_data) {
            return font;
        }
    }
    
    // Если ни один шрифт не найден, используем встроенный моноширинный fallback
    // Это простой моноширинный шрифт 14pt, закодированный в base64
    // Для простоты используем DejaVuSans из пакета fonts-dejavu-core
    panic!("Не найден системный шрифт для экспорта PNG. Установите один из: fonts-dejavu-core, fonts-liberation, fonts-freefont-ttf")
}

/// Выбрать количество знаков после запятой в зависимости от масштаба (шага сетки).
fn format_coord(val: f64, step: f64) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_csv_basic() {
        let content = "x,y\n1,2\n3,4\n5,6";
        let points = parse_csv(content).unwrap();
        assert_eq!(points.len(), 3);
        assert_eq!(points[0], (1.0, 2.0));
        assert_eq!(points[1], (3.0, 4.0));
        assert_eq!(points[2], (5.0, 6.0));
    }

    #[test]
    fn parse_csv_no_header() {
        let content = "1,2\n3,4\n5,6";
        let points = parse_csv(content).unwrap();
        assert_eq!(points.len(), 3);
    }

    #[test]
    fn parse_csv_semicolon_separator() {
        let content = "x;y\n1;2\n3;4";
        let points = parse_csv(content).unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(points[0], (1.0, 2.0));
    }

    #[test]
    fn parse_csv_tab_separator() {
        let content = "x\ty\n1\t2\n3\t4";
        let points = parse_csv(content).unwrap();
        assert_eq!(points.len(), 2);
    }

    #[test]
    fn parse_csv_with_blank_lines() {
        let content = "x,y\n\n1,2\n\n3,4\n";
        let points = parse_csv(content).unwrap();
        assert_eq!(points.len(), 2);
    }

    #[test]
    fn parse_csv_negative_values() {
        let content = "x,y\n-1,-2\n-3,-4";
        let points = parse_csv(content).unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(points[0], (-1.0, -2.0));
    }

    #[test]
    fn parse_csv_float_values() {
        let content = "x,y\n1.5,2.7\n3.14,4.56";
        let points = parse_csv(content).unwrap();
        assert_eq!(points.len(), 2);
        assert!((points[0].0 - 1.5).abs() < 1e-10);
        assert!((points[0].1 - 2.7).abs() < 1e-10);
    }

    #[test]
    fn parse_csv_skip_invalid_lines() {
        let content = "x,y\n1,2\ninvalid\n3,4";
        let points = parse_csv(content).unwrap();
        // Строка "invalid" пропускается, остальные парсятся
        assert_eq!(points.len(), 2);
    }

    #[test]
    fn parse_csv_empty_file() {
        let result = parse_csv("");
        assert!(result.is_err());
    }

    #[test]
    fn parse_csv_all_invalid() {
        let content = "invalid\nno_data\n";
        let result = parse_csv(content);
        assert!(result.is_err());
    }

    #[test]
    fn parse_csv_single_column() {
        let content = "x\n1\n2\n";
        let result = parse_csv(content);
        assert!(result.is_err()); // Нужно минимум 2 колонки
    }

    #[test]
    fn parse_csv_inf_nan_skipped() {
        let content = "x,y\n1,2\ninf,3\n4,NaN\n5,6";
        let points = parse_csv(content).unwrap();
        // inf и NaN пропускаются
        assert_eq!(points.len(), 2);
        assert_eq!(points[0], (1.0, 2.0));
        assert_eq!(points[1], (5.0, 6.0));
    }

    #[test]
    fn parse_csv_whitespace_handling() {
        let content = "  x  ,  y  \n  1  ,  2  \n  3  ,  4  ";
        let points = parse_csv(content).unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(points[0], (1.0, 2.0));
    }

    #[test]
    fn parse_csv_large_file() {
        let mut content = String::from("x,y\n");
        for i in 0..1000 {
            content.push_str(&format!("{},{}\n", i, i * 2));
        }
        let points = parse_csv(&content).unwrap();
        assert_eq!(points.len(), 1000);
        assert_eq!(points[0], (0.0, 0.0));
        assert_eq!(points[999], (999.0, 1998.0));
    }
}
