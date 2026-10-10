/// Генерация иконки приложения.
///
/// Создаёт простую иконку с графиком синусоиды.

#[allow(dead_code)]
use image::{Rgba, RgbaImage};

/// Создать иконку 256x256 для приложения.
#[allow(dead_code)]
pub fn create_app_icon() -> RgbaImage {
    let mut img = RgbaImage::new(256, 256);

    // Фон — градиент от тёмно-синего к синему
    for y in 0..256 {
        for x in 0..256 {
            let t = y as f32 / 256.0;
            let r = (30.0 + t * 20.0) as u8;
            let g = (60.0 + t * 40.0) as u8;
            let b = (180.0 + t * 40.0) as u8;
            img.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }

    // Сетка (тонкие линии)
    for i in (0..256).step_by(32) {
        // Вертикальные
        for y in 0..256 {
            img.put_pixel(i as u32, y, Rgba([255, 255, 255, 20]));
        }
        // Горизонтальные
        for x in 0..256 {
            img.put_pixel(x as u32, i as u32, Rgba([255, 255, 255, 20]));
        }
    }

    // Оси
    for y in 0..256 {
        img.put_pixel(128, y, Rgba([255, 255, 255, 120])); // ось Y
    }
    for x in 0..256 {
        img.put_pixel(x as u32, 128, Rgba([255, 255, 255, 120])); // ось X
    }

    // График синусоиды (белая линия с подсветкой)
    for x in 0..256 {
        let t = (x as f32 / 256.0) * 2.0 * std::f32::consts::PI * 2.0;
        let y = 128.0 - (t.sin() * 60.0);
        
        // Основная линия
        img.put_pixel(x as u32, y as u32, Rgba([255, 255, 255, 255]));
        
        // Подсветка (свечение)
        if y > 1.0 {
            img.put_pixel(x as u32, y as u32 - 1, Rgba([100, 200, 255, 80]));
        }
        if y < 255.0 {
            img.put_pixel(x as u32, y as u32 + 1, Rgba([100, 200, 255, 80]));
        }
    }

    // Точки пересечения с осью X (зелёные маркеры)
    let zero_crossings = [
        64,  // sin(0)
        128, // sin(pi)
        192, // sin(2*pi)
    ];
    for &x in &zero_crossings {
        let cx = x as u32;
        let cy = 128u32;
        for dy in -2i32..=2 {
            for dx in -2i32..=2 {
                if dx.abs() + dy.abs() <= 2 {
                    let px = (cx as i32 + dx) as u32;
                    let py = (cy as i32 + dy) as u32;
                    if px < 256 && py < 256 {
                        img.put_pixel(px, py, Rgba([0, 255, 100, 200]));
                    }
                }
            }
        }
    }

    img
}

/// Сохранить иконку в PNG-файл.
#[allow(dead_code)]
pub fn save_icon_to_file(path: &str) -> Result<(), String> {
    let img = create_app_icon();
    img.save(path).map_err(|e| format!("Сохранение иконки: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icon_size() {
        let img = create_app_icon();
        assert_eq!(img.width(), 256);
        assert_eq!(img.height(), 256);
    }

    #[test]
    fn test_icon_save() {
        let img = create_app_icon();
        let path = "/tmp/function-plotter-icon-test.png";
        assert!(img.save(path).is_ok());
        std::fs::remove_file(path).ok();
    }
}
