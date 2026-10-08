//! Генерация SVG для графиков функций.
//!
//! Создает векторный SVG с осями, сеткой, кривыми и легендой.
//! Без внешних зависимостей — чистый SVG XML.

use crate::app::GraphEntry;
use crate::parser;
use crate::renderer::Viewport;
use crate::theme::Theme;

/// Данные для экспорта одного графика.
pub struct ExportGraph {
    pub label: String,
    pub color: egui::Color32,
    pub points: Vec<(f64, f64)>,
    pub visible: bool,
    pub is_derivative: bool,
    pub is_integral: bool,
}

/// Параметры SVG экспорта.
pub struct SvgExportParams {
    pub width: u32,
    pub height: u32,
    pub padding: f64,
    pub show_grid: bool,
    pub show_labels: bool,
    pub show_legend: bool,
}

impl Default for SvgExportParams {
    fn default() -> Self {
        Self {
            width: 1200,
            height: 800,
            padding: 60.0,
            show_grid: true,
            show_labels: true,
            show_legend: true,
        }
    }
}

/// Сгенерировать SVG для одного графика.
pub fn generate_svg(
    graphs: &[ExportGraph],
    viewport: &Viewport,
    params: &SvgExportParams,
    theme: &Theme,
    font_config: &crate::theme::FontConfig,
) -> String {
    let w = params.width as f64;
    let h = params.height as f64;
    let pad = params.padding;

    let plot_x = pad;
    let plot_y = pad;
    let plot_w = w - 2.0 * pad;
    let plot_h = h - 2.0 * pad;

    // Масштабирование: математические координаты -> экранные
    let math_to_screen = |x: f64, y: f64| -> (f64, f64) {
        let sx = plot_x + (x - viewport.x_min) / (viewport.x_max - viewport.x_min) * plot_w;
        let sy = plot_y + plot_h - (y - viewport.y_min) / (viewport.y_max - viewport.y_min) * plot_h;
        (sx, sy)
    };

    let mut svg = String::new();

    // Заголовок SVG
    svg.push_str(&format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\"\n\
              width=\"{:.0}\" height=\"{:.0}\" viewBox=\"0 0 {:.0} {:.0}\">\n",
        w, h, w, h
    ));

    // Фон
    svg.push_str(&format!(
        "  <rect width=\"{:.0}\" height=\"{:.0}\" fill=\"#{}\"/>\n",
        w,
        h,
        theme.canvas_color_hex()
    ));

    // Сетка
    if params.show_grid {
        svg.push_str(&format!(
            "  <g stroke=\"#{}\" stroke-width=\"0.5\" opacity=\"0.3\">\n",
            theme.grid_color_hex()
        ));

        let step_x = nice_step(viewport.x_max - viewport.x_min, 8);
        let step_y = nice_step(viewport.y_max - viewport.y_min, 8);

        // Вертикальные линии
        let mut x = viewport.x_min.floor() / step_x * step_x;
        while x <= viewport.x_max {
            if x >= viewport.x_min && x <= viewport.x_max {
                let (sx, _) = math_to_screen(x, 0.0);
                svg.push_str(&format!(
                    "    <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\"/>\n",
                    sx, plot_y, sx, plot_y + plot_h
                ));
            }
            x += step_x;
        }

        // Горизонтальные линии
        let mut y = viewport.y_min.floor() / step_y * step_y;
        while y <= viewport.y_max {
            if y >= viewport.y_min && y <= viewport.y_max {
                let (_, sy) = math_to_screen(0.0, y);
                svg.push_str(&format!(
                    "    <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\"/>\n",
                    plot_x, sy, plot_x + plot_w, sy
                ));
            }
            y += step_y;
        }

        svg.push_str("  </g>\n");
    }

    // Оси
    svg.push_str(&format!(
        "  <g stroke=\"#{}\" stroke-width=\"1.5\">\n",
        theme.axis_color_hex()
    ));

    // Ось X (y = 0)
    if viewport.y_min <= 0.0 && viewport.y_max >= 0.0 {
        let (_, sy) = math_to_screen(0.0, 0.0);
        svg.push_str(&format!(
            "    <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\"/>\n",
            plot_x, sy, plot_x + plot_w, sy
        ));
    }

    // Ось Y (x = 0)
    if viewport.x_min <= 0.0 && viewport.x_max >= 0.0 {
        let (sx, _) = math_to_screen(0.0, 0.0);
        svg.push_str(&format!(
            "    <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\"/>\n",
            sx, plot_y, sx, plot_y + plot_h
        ));
    }

    svg.push_str("  </g>\n");

    // Подписи осей
    if params.show_labels {
        let font_size = font_config.size * 0.9;
        svg.push_str(&format!(
            "  <g font-family=\"monospace\" font-size=\"{:.1}\" fill=\"#{}\">\n",
            font_size,
            theme.text_color_hex()
        ));

        let step_x = nice_step(viewport.x_max - viewport.x_min, 8);
        let step_y = nice_step(viewport.y_max - viewport.y_min, 8);

        // Метки по X
        let mut x = viewport.x_min.floor() / step_x * step_x;
        while x <= viewport.x_max {
            if x >= viewport.x_min && x <= viewport.x_max {
                let (sx, sy) = math_to_screen(x, 0.0);
                if sx >= plot_x && sx <= plot_x + plot_w {
                    let label = format_coord(x, step_x);
                    svg.push_str(&format!(
                        "    <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" dominant-baseline=\"hanging\">{}</text>\n",
                        sx, sy + 4.0, escape_xml(&label)
                    ));
                }
            }
            x += step_x;
        }

        // Метки по Y
        let mut y = viewport.y_min.floor() / step_y * step_y;
        while y <= viewport.y_max {
            if y >= viewport.y_min && y <= viewport.y_max {
                let (sx, sy) = math_to_screen(0.0, y);
                if sy >= plot_y && sy <= plot_y + plot_h {
                    let label = format_coord(y, step_y);
                    svg.push_str(&format!(
                        "    <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\" dominant-baseline=\"central\">{}</text>\n",
                        sx - 4.0, sy, escape_xml(&label)
                    ));
                }
            }
            y += step_y;
        }

        svg.push_str("  </g>\n");
    }

    // Графики
    for graph in graphs.iter().filter(|g| g.visible && !g.points.is_empty()) {
        let color = graph.color;
        let stroke_width = if graph.is_derivative {
            1.5
        } else if graph.is_integral {
            1.0
        } else {
            2.0
        };
        let stroke_dash = if graph.is_derivative {
            " stroke-dasharray=\"6,4\""
        } else if graph.is_integral {
            ""
        } else {
            ""
        };

        svg.push_str(&format!(
            "  <g stroke=\"#{}\" stroke-width=\"{:.1}\" fill=\"none\"{}>\n",
            color_hex(color),
            stroke_width,
            stroke_dash
        ));

        // Рисуем кривую сегментами (разрывы при inf/nan)
        let mut segments: Vec<Vec<(f64, f64)>> = vec![Vec::new()];
        for &(x, y) in &graph.points {
            if !y.is_finite() {
                segments.push(Vec::new());
            } else {
                let last = segments.last_mut().unwrap();
                last.push((x, y));
            }
        }

        for segment in segments.iter().filter(|s| s.len() >= 2) {
            let mut path_data = String::new();
            for (i, &(x, y)) in segment.iter().enumerate() {
                let (sx, sy) = math_to_screen(x, y);
                if i == 0 {
                    path_data.push_str(&format!("M {:.2} {:.2}", sx, sy));
                } else {
                    path_data.push_str(&format!(" L {:.2} {:.2}", sx, sy));
                }
            }
            svg.push_str(&format!("    <path d=\"{}\"/>\n", path_data));
        }

        svg.push_str("  </g>\n");
    }

    // Легенда
    if params.show_legend {
        let mut legend_y = plot_y + 16.0;
        svg.push_str(&format!(
            "  <g font-family=\"{}\" font-size=\"{:.1}\" fill=\"#{}\">\n",
            if font_config.family == "monospace" {
                "monospace"
            } else {
                "sans-serif"
            },
            font_config.size,
            theme.text_color_hex()
        ));

        for graph in graphs.iter().filter(|g| g.visible) {
            let color = graph.color;
            // Линия легенды
            svg.push_str(&format!(
                "    <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#{}\" stroke-width=\"2\"/>\n",
                plot_x + 10.0,
                legend_y,
                plot_x + 30.0,
                legend_y,
                color_hex(color)
            ));
            // Текст легенды
            svg.push_str(&format!(
                "    <text x=\"{:.1}\" y=\"{:.1}\" dominant-baseline=\"central\">{}</text>\n",
                plot_x + 36.0,
                legend_y,
                escape_xml(&graph.label)
            ));
            legend_y += 20.0;
        }

        svg.push_str("  </g>\n");
    }

    svg.push_str("</svg>\n");

    svg
}

/// Форматирование координаты для подписей.
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

/// Подобрать "красивый" шаг сетки.
fn nice_step(range: f64, target_count: usize) -> f64 {
    if range <= 0.0 {
        return 1.0;
    }
    let raw = range / target_count as f64;
    let mag = 10.0_f64.powf(raw.abs().log10().floor());
    let unit = raw / mag;

    let candidates = [1.0, 2.0, 5.0];
    let best = candidates
        .iter()
        .min_by(|a, b| {
            let diff_a = (*a - unit).abs();
            let diff_b = (*b - unit).abs();
            diff_a
                .partial_cmp(&diff_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .copied()
        .unwrap_or(1.0);

    best * mag
}

/// HEX код цвета из egui::Color32.
fn color_hex(color: egui::Color32) -> String {
    format!("{:02x}{:02x}{:02x}", color.r(), color.g(), color.b())
}

/// Экранирование XML спецсимволов.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Собрать данные для экспорта из графических записей.
pub fn collect_export_data(graphs: &[GraphEntry]) -> Vec<ExportGraph> {
    graphs
        .iter()
        .filter_map(|g| {
            if let Some(ref data) = g.data {
                Some(ExportGraph {
                    label: g.style.label.clone(),
                    color: g.style.color,
                    points: data.points.clone(),
                    visible: g.style.visible,
                    is_derivative: g.parsed.as_ref().map_or(false, |p| {
                        p.formula_type() == parser::FormulaType::Derivative
                    }),
                    is_integral: g.parsed.as_ref().map_or(false, |p| {
                        p.formula_type() == parser::FormulaType::Integral
                    }),
                })
            } else {
                None
            }
        })
        .collect()
}

/// Экспорт всех видимых графиков в SVG.
pub fn export_svg(
    graphs: &[GraphEntry],
    viewport: &Viewport,
    theme: &Theme,
    font_config: &crate::theme::FontConfig,
    path: &std::path::Path,
) -> Result<(), String> {
    let export_data = collect_export_data(graphs);
    let svg = generate_svg(&export_data, viewport, &SvgExportParams::default(), theme, font_config);

    std::fs::write(path, &svg)
        .map_err(|e| format!("Запись SVG: {e}"))?;

    Ok(())
}
