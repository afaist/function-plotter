use crate::parser::{FormulaType, ParsedFormula};

/// Точка с дополнительными метаданными для адаптивного алгоритма.
#[derive(Clone, Debug)]
struct AdaptivePoint {
    x: f64,
    y: f64,
    #[allow(dead_code)]
    curvature: f64, // Отклонение от линейной интерполяции
}

#[derive(Clone, Debug)]
pub struct PlotData {
    pub points: Vec<(f64, f64)>,
    pub y_min: f64,
    pub y_max: f64,
    /// Для интегралов — точки для закрашенной области.
    pub fill_points: Vec<(f64, f64)>,
}

impl PlotData {
    pub fn compute(
        formula: &ParsedFormula,
        x_min: f64,
        x_max: f64,
        n_points: usize,
    ) -> Self {
        let n = n_points.max(2);
        let dx = (x_max - x_min) / (n as f64 - 1.0);

        let mut points = Vec::with_capacity(n);
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        let mut fill_points = Vec::new();

        match formula.formula_type() {
            FormulaType::Derivative => {
                // Численная производная: f'(x) ≈ (f(x+h) - f(x-h)) / (2h)
                if let Some(inner) = formula.inner() {
                    let h = 1e-6;
                    for i in 0..n {
                        let x = x_min + dx * i as f64;
                        let y = (inner.eval(x + h) - inner.eval(x - h)) / (2.0 * h);
                        if y.is_finite() {
                            y_min = y_min.min(y);
                            y_max = y_max.max(y);
                            fill_points.push((x, y));
                        }
                        points.push((x, y));
                    }
                }
            }
            FormulaType::Integral => {
                if let Some((lower, _upper)) = formula.integral_bounds() {
                    let inner = formula.inner().unwrap();
                    // Вычисляем неопределённый интеграл F(x) = ∫_lower^x f(t) dt
                    // с помощью квадратурной формулы Симпсона
                    let n_simpson = 100; // точность интегрирования
                    let n = n.max(2);

                    for i in 0..n {
                        let x = x_min + dx * i as f64;
                        let y = Self::compute_indefinite_integral(inner, lower, x, n_simpson);
                        if y.is_finite() {
                            y_min = y_min.min(y);
                            y_max = y_max.max(y);
                            fill_points.push((x, y));
                        }
                        points.push((x, y));
                    }
                }
            }
            FormulaType::Regular => {
                for i in 0..n {
                    let x = x_min + dx * i as f64;
                    let y = formula.eval(x);

                    if y.is_finite() {
                        y_min = y_min.min(y);
                        y_max = y_max.max(y);
                    }
                    points.push((x, y));
                }
            }
        }

        if !y_min.is_finite() || !y_max.is_finite() {
            y_min = -1.0;
            y_max = 1.0;
        }

        PlotData {
            points,
            y_min,
            y_max,
            fill_points,
        }
    }

    /// Вычислить неопределённый интеграл ∫_lower^x f(t) dt с помощью правила Симпсона.
    fn compute_indefinite_integral(
        inner: &ParsedFormula,
        lower: f64,
        x: f64,
        n_steps: usize,
    ) -> f64 {
        let a = lower.min(x);
        let b = lower.max(x);
        let n = n_steps.max(2);
        let h = (b - a) / n as f64;

        // Правило Симпсона: ∫_a^b f(t) dt ≈ h/3 * [f(a) + 4Σf(a+(2i-1)h) + 2Σf(a+2ih) + f(b)]
        let mut sum = inner.eval(a) + inner.eval(b);

        for i in 1..n {
            let t = a + h * i as f64;
            let y = inner.eval(t);
            if i % 2 == 0 {
                sum += 2.0 * y;
            } else {
                sum += 4.0 * y;
            }
        }

        sum * h / 3.0
    }

    /// Вычислить график с адаптивным количеством точек (subdivision algorithm).
    pub fn compute_adaptive(
        formula: &ParsedFormula,
        x_min: f64,
        x_max: f64,
        max_refinements: usize,
        tolerance: f64,
    ) -> Self {
        let mut points: Vec<AdaptivePoint> = Vec::new();

        Self::adaptive_subdivision(
            formula,
            x_min,
            x_max,
            max_refinements,
            tolerance,
            &mut points,
        );

        // Преобразуем в (x, y) пары
        let n = points.len();
        let mut result_points = Vec::with_capacity(n);
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        let mut fill_points = Vec::new();

        for p in points {
            if p.y.is_finite() {
                y_min = y_min.min(p.y);
                y_max = y_max.max(p.y);
                fill_points.push((p.x, p.y));
            }
            result_points.push((p.x, p.y));
        }

        if !y_min.is_finite() || !y_max.is_finite() {
            y_min = -1.0;
            y_max = 1.0;
        }

        PlotData {
            points: result_points,
            y_min,
            y_max,
            fill_points,
        }
    }

    /// Рекурсивный subdivison: добавляем точки там где кривизна высока.
    fn adaptive_subdivision(
        formula: &ParsedFormula,
        x0: f64,
        x2: f64,
        max_depth: usize,
        tolerance: f64,
        points: &mut Vec<AdaptivePoint>,
    ) {
        let y0 = formula.eval(x0);
        let x1 = (x0 + x2) / 2.0;
        let y1 = formula.eval(x1);
        let x2_val = x2;
        let y2 = formula.eval(x2);

        // Вычисляем отклонение средней точки от линейной интерполяции
        let y_linear = (y0 + y2) / 2.0;
        let deviation = (y1 - y_linear).abs();

        if max_depth > 0 && deviation > tolerance {
            // Рекурсивно subdivison левую и правую половины
            Self::adaptive_subdivision(formula, x0, x1, max_depth - 1, tolerance, points);
            Self::adaptive_subdivision(formula, x1, x2_val, max_depth - 1, tolerance, points);
        } else {
            // Добавляем только концы отрезка (середина уже добавлена на предыдущем уровне)
            if y0.is_finite() {
                points.push(AdaptivePoint {
                    x: x0,
                    y: y0,
                    curvature: 0.0,
                });
            }
            if y2.is_finite() {
                points.push(AdaptivePoint {
                    x: x2_val,
                    y: y2,
                    curvature: deviation,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    #[test]
    fn compute_regular() {
        let formula = parser::parse("x^2 / 10").unwrap();
        let data = PlotData::compute(&formula, 0.0, 10.0, 11);
        assert_eq!(data.points.len(), 11);
        // y_min = 0, y_max = 10
        assert!(data.y_min.abs() < 1e-10);
        assert!((data.y_max - 10.0).abs() < 1e-6);
    }

    #[test]
    fn compute_derivative_at_zero() {
        let formula = parser::parse("deriv(sin(x))").unwrap();
        let data = PlotData::compute(&formula, -0.1, 0.1, 21);
        // Производная sin(x) = cos(x), cos(0) ≈ 1
        let mid = data.points.len() / 2;
        assert!((data.points[mid].1 - 1.0).abs() < 0.01);
    }

    #[test]
    fn compute_derivative_polynomial() {
        let formula = parser::parse("deriv(x^2)").unwrap();
        let data = PlotData::compute(&formula, 2.0, 4.0, 21);
        // Производная x^2 = 2x, при x=3 → 6
        let mid = data.points.len() / 2;
        assert!((data.points[mid].1 - 6.0).abs() < 0.05);
    }

    #[test]
    fn compute_integral_x() {
        let formula = parser::parse("integral(x, 0, 1)").unwrap();
        let data = PlotData::compute(&formula, 0.0, 1.0, 11);
        // ∫₀^x t dt = x²/2, при x=1 → 0.5
        let last = data.points.last().unwrap();
        assert!((last.1 - 0.5).abs() < 0.01);
    }

    #[test]
    fn compute_integral_sin() {
        let formula = parser::parse("integral(sin(x), 0, pi)").unwrap();
        let data = PlotData::compute(&formula, 0.0, std::f64::consts::PI, 101);
        // ∫₀^π sin(t) dt = 2
        let last = data.points.last().unwrap();
        assert!((last.1 - 2.0).abs() < 0.02);
    }

    #[test]
    fn compute_integral_constant() {
        let formula = parser::parse("integral(1, 0, 5)").unwrap();
        let data = PlotData::compute(&formula, 0.0, 5.0, 11);
        // ∫₀^x 1 dt = x, при x=5 → 5
        let last = data.points.last().unwrap();
        assert!((last.1 - 5.0).abs() < 1e-6);
    }

    #[test]
    fn fill_points_empty_for_regular() {
        let formula = parser::parse("x^2").unwrap();
        let data = PlotData::compute(&formula, -5.0, 5.0, 11);
        // Обычные графики не имеют fill_points
        assert!(data.fill_points.is_empty());
    }

    #[test]
    fn fill_points_not_populated_for_regular() {
        let formula = parser::parse("1/x").unwrap();
        let data = PlotData::compute(&formula, -1.0, 1.0, 101);
        // Обычные графики (даже с разрывами) не имеют fill_points
        assert!(data.fill_points.is_empty());
    }

    #[test]
    fn compute_adaptive_simple() {
        let formula = parser::parse("sin(x)").unwrap();
        let data = PlotData::compute_adaptive(&formula, 0.0, std::f64::consts::PI, 8, 0.01);
        // Адаптивный алгоритм должен дать больше точек чем равномерный для гладких функций
        assert!(data.points.len() >= 10);
        // Точки должны покрывать весь диапазон
        let first_x = data.points.first().unwrap().0;
        let last_x = data.points.last().unwrap().0;
        assert!((first_x - 0.0).abs() < 0.01);
        assert!((last_x - std::f64::consts::PI).abs() < 0.01);
    }

    #[test]
    fn compute_adaptive_vs_uniform() {
        let formula = parser::parse("x").unwrap(); // Линейная функция — адаптивный должен дать минимум точек
        let uniform = PlotData::compute(&formula, 0.0, 10.0, 100);
        let adaptive = PlotData::compute_adaptive(&formula, 0.0, 10.0, 8, 0.01);
        // Для линейной функции адаптивный должен дать минимум точек (2-3)
        assert!(adaptive.points.len() <= 5);
        assert!(adaptive.points.len() < uniform.points.len());
    }

    #[test]
    fn compute_adaptive_high_curvature() {
        let formula = parser::parse("sin(10*x)").unwrap();
        let adaptive = PlotData::compute_adaptive(&formula, 0.0, 1.0, 10, 0.001);
        // Высокая кривизна должна привести к большому количеству точек
        assert!(adaptive.points.len() > 50);
    }
}
