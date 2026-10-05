use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use egui::Rect;
use image::{self, Rgba};

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

/// Обрезать область скриншота и сохранить в PNG.
pub fn save_rect_to_png(
    screenshot: &egui::ColorImage,
    rect: Rect,
    path: &std::path::Path,
) -> Result<(), String> {
    let width = screenshot.width() as f32;
    let height = screenshot.height() as f32;

    // Переводим логические координаты egui в пиксели изображения
    let x = (rect.min.x.clamp(0.0, width)) as usize;
    let y = (rect.min.y.clamp(0.0, height)) as usize;
    let w = rect.width().min(width - rect.min.x) as usize;
    let h = rect.height().min(height - rect.min.y) as usize;

    if w == 0 || h == 0 {
        return Err("Область для сохранения пуста".into());
    }

    // Конвертируем ColorImage (RGBA) из egui в буфер для image
    let mut pixels = Vec::with_capacity(w * h * 4);
    for row in y..(y + h) {
        for col in x..(x + w) {
            let color = screenshot.pixels[row * screenshot.width() + col];
            pixels.extend_from_slice(&color.to_array());
        }
    }

    // Сохраняем область в файл
    if let Some(buffer) = image::ImageBuffer::<Rgba<u8>, _>::from_raw(w as u32, h as u32, pixels) {
        buffer
            .save(path)
            .map_err(|e| format!("Сохранение PNG: {e}"))?;
        println!("График сохранён в {}", path.display());
    } else {
        return Err("Не удалось создать изображение".into());
    }

    Ok(())
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
