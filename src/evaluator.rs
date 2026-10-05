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
    /// Вершина параболы (x0, y0) — для квадратичных функций
    pub vertex: Option<(f64, f64)>,
    /// Корни уравнения (точки пересечения с осью X)
    pub roots: Vec<f64>,
}

impl PlotData {
    pub fn compute(formula: &ParsedFormula, x_min: f64, x_max: f64, n_points: usize) -> Self {
        let n = n_points.max(2);
        let dx = (x_max - x_min) / (n as f64 - 1.0);

        let mut points = Vec::with_capacity(n);
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        let mut fill_points = Vec::new();
        let mut vertex: Option<(f64, f64)> = None;
        let mut roots = Vec::new();

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
            FormulaType::Polar => {
                // Полярные координаты: r = f(theta), x = r*cos(theta), y = r*sin(theta)
                if let Some((theta_min, theta_max)) = formula.polar_bounds() {
                    let inner = formula.inner().unwrap();
                    let n = n.max(2);
                    let dtheta = (theta_max - theta_min) / (n as f64 - 1.0);

                    let mut x_min = f64::INFINITY;
                    let mut x_max = f64::NEG_INFINITY;

                    for i in 0..n {
                        let theta = theta_min + dtheta * i as f64;
                        let r = inner.eval(theta);

                        if r.is_finite() {
                            let x = r * theta.cos();
                            let y = r * theta.sin();

                            if x.is_finite() && y.is_finite() {
                                x_min = x_min.min(x);
                                x_max = x_max.max(x);
                                y_min = y_min.min(y);
                                y_max = y_max.max(y);
                                points.push((x, y));
                            }
                        } else {
                            points.push((f64::NAN, f64::NAN));
                        }
                    }

                    // Обновляем x_min/x_max для viewport
                    if x_min.is_finite() {
                        // Для полярных координат x_min/x_max передаются как theta_min/theta_max,
                        // но нам нужно обновить их в caller'е. Здесь мы просто используем
                        // стандартный подход.
                    }
                }
            }
            FormulaType::Parametric => {
                // Параметрические уравнения: x = f(t), y = g(t)
                if let Some((x_formula, y_formula, t_min, t_max)) = formula.parametric_bounds() {
                    let n = n.max(2);
                    let dt = (t_max - t_min) / (n as f64 - 1.0);

                    for i in 0..n {
                        let t = t_min + dt * i as f64;
                        let x = x_formula.eval(t);
                        let y = y_formula.eval(t);

                        if x.is_finite() && y.is_finite() {
                            y_min = y_min.min(y);
                            y_max = y_max.max(y);
                            points.push((x, y));
                        } else {
                            points.push((f64::NAN, f64::NAN));
                        }
                    }
                }
            }
        }

        if !y_min.is_finite() || !y_max.is_finite() {
            y_min = -1.0;
            y_max = 1.0;
        }

        // Вычисляем вершину и корни для квадратичных функций
        if formula.formula_type() == FormulaType::Regular && !points.is_empty() {
            // Ищем вершину и корни среди вычисленных точек
            // Вершина — точка с минимальным y
            if let Some((v_x, v_y)) = points.iter()
                .filter(|(_, y)| y.is_finite())
                .min_by(|a, b| a.1.partial_cmp(&b.1).expect("finite y values"))
            {
                vertex = Some((*v_x, *v_y));
            }
            
            // Корни — ищем смену знака между соседними точками (метод бисекции)
            let mut last_root_x: Option<f64> = None;
            for i in 0..points.len().saturating_sub(1) {
                let (x1, y1) = points[i];
                let (x2, y2) = points[i + 1];
                
                if !y1.is_finite() || !y2.is_finite() {
                    continue;
                }
                
                // Смена знака — корень между x1 и x2
                if (y1 <= 0.0 && y2 > 0.0) || (y1 > 0.0 && y2 <= 0.0) {
                    // Линейная интерполяция для точного значения
                    let root_x = x1 - y1 * (x2 - x1) / (y2 - y1);
                    
                    // Проверяем, чтобы не добавлять дубликаты (корни дальше 0.5 по x)
                    if let Some(last) = last_root_x {
                        if (root_x - last).abs() < 0.5 {
                            continue;
                        }
                    }
                    
                    last_root_x = Some(root_x);
                    roots.push(root_x);
                }
            }
        }

        PlotData {
            points,
            y_min,
            y_max,
            fill_points,
            vertex,
            roots,
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
        // Заполняем fill_points только для интегралов
        let mut fill_points = Vec::new();
        let is_integral = matches!(formula.formula_type(), FormulaType::Integral);

        for p in points {
            if p.y.is_finite() {
                y_min = y_min.min(p.y);
                y_max = y_max.max(p.y);
                if is_integral {
                    fill_points.push((p.x, p.y));
                }
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
            vertex: None,
            roots: Vec::new(),
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
        let y2 = formula.eval(x2);

        // Вычисляем отклонение средней точки от линейной интерполяции
        let y_linear = (y0 + y2) / 2.0;
        let deviation = (y1 - y_linear).abs();

        if max_depth > 0 && deviation > tolerance {
            // Рекурсивно subdivison левую и правую половины
            Self::adaptive_subdivision(formula, x0, x1, max_depth - 1, tolerance, points);
            Self::adaptive_subdivision(formula, x1, x2, max_depth - 1, tolerance, points);
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
                    x: x2,
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

    // --- Тесты точности численных методов ---

    #[test]
    fn test_derivative_cos_accuracy() {
        // Производная sin(x) = cos(x)
        let formula = parser::parse("deriv(sin(x))").unwrap();
        let data = PlotData::compute(&formula, 0.0, 1.0, 11);
        
        // Проверяем несколько точек
        for i in 0..data.points.len() {
            let x = data.points[i].0;
            let y = data.points[i].1;
            let expected = x.cos(); // cos(x)
            
            if y.is_finite() {
                let error = (y - expected).abs();
                assert!(error < 1e-10, "При x={:.4}: got {:.10}, expected {:.10}, error={:.2e}",
                    x, y, expected, error);
            }
        }
    }

    #[test]
    fn test_derivative_linear_accuracy() {
        // Производная 3x^2 + 2x = 6x + 2
        let formula = parser::parse("deriv(3*x^2 + 2*x)").unwrap();
        let data = PlotData::compute(&formula, 0.0, 5.0, 21);
        
        for i in 0..data.points.len() {
            let x = data.points[i].0;
            let y = data.points[i].1;
            let expected = 6.0 * x + 2.0;
            
            if y.is_finite() {
                let error = (y - expected).abs();
                assert!(error < 1e-8, "При x={:.4}: got {:.10}, expected {:.10}, error={:.2e}",
                    x, y, expected, error);
            }
        }
    }

    #[test]
    fn test_derivative_exp_accuracy() {
        // Производная e^x = e^x
        let formula = parser::parse("deriv(exp(x))").unwrap();
        let data = PlotData::compute(&formula, 0.0, 1.0, 11);
        
        for i in 0..data.points.len() {
            let x = data.points[i].0;
            let y = data.points[i].1;
            let expected = x.exp();
            
            if y.is_finite() {
                let error = (y - expected).abs();
                assert!(error < 1e-9, "При x={:.4}: got {:.10}, expected {:.10}, error={:.2e}",
                    x, y, expected, error);
            }
        }
    }

    #[test]
    fn test_integral_x_squared_accuracy() {
        // ∫₀^x t² dt = x³/3
        let formula = parser::parse("integral(x^2, 0, 1)").unwrap();
        let data = PlotData::compute(&formula, 0.0, 1.0, 21);
        
        for i in 0..data.points.len() {
            let x = data.points[i].0;
            let y = data.points[i].1;
            let expected = x * x * x / 3.0;
            
            if y.is_finite() {
                let error = (y - expected).abs();
                assert!(error < 1e-6, "При x={:.4}: got {:.10}, expected {:.10}, error={:.2e}",
                    x, y, expected, error);
            }
        }
    }

    #[test]
    fn test_integral_cos_accuracy() {
        // ∫₀^x cos(t) dt = sin(x)
        let formula = parser::parse("integral(cos(x), 0, 1)").unwrap();
        let data = PlotData::compute(&formula, 0.0, std::f64::consts::PI, 51);
        
        for i in 0..data.points.len() {
            let x = data.points[i].0;
            let y = data.points[i].1;
            let expected = x.sin();
            
            if y.is_finite() {
                let error = (y - expected).abs();
                assert!(error < 1e-5, "При x={:.4}: got {:.10}, expected {:.10}, error={:.2e}",
                    x, y, expected, error);
            }
        }
    }

    #[test]
    fn test_integral_exp_accuracy() {
        // ∫₀^x e^t dt = e^x - 1
        let formula = parser::parse("integral(exp(x), 0, 1)").unwrap();
        let data = PlotData::compute(&formula, 0.0, 1.0, 21);
        
        for i in 0..data.points.len() {
            let x = data.points[i].0;
            let y = data.points[i].1;
            let expected = x.exp() - 1.0;
            
            if y.is_finite() {
                let error = (y - expected).abs();
                assert!(error < 1e-5, "При x={:.4}: got {:.10}, expected {:.10}, error={:.2e}",
                    x, y, expected, error);
            }
        }
    }

    #[test]
    fn test_adaptive_linear_efficiency() {
        // Для линейной функции адаптивный алгоритм должен дать минимальное количество точек
        let formula = parser::parse("2*x + 3").unwrap();
        let adaptive = PlotData::compute_adaptive(&formula, 0.0, 10.0, 10, 0.01);
        
        // Линейная функция — максимум 3 точки (концы + середина, если нужно)
        assert!(adaptive.points.len() <= 5,
            "Для линейной функции адаптивный алгоритм дал {} точек (ожидается <= 5)",
            adaptive.points.len());
        
        // Точки должны покрывать весь диапазон
        let first_x = adaptive.points.first().unwrap().0;
        let last_x = adaptive.points.last().unwrap().0;
        assert!((first_x - 0.0).abs() < 0.01);
        assert!((last_x - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_adaptive_quadratic_accuracy() {
        // Для x^2 адаптивный алгоритм должен точно воспроизвести кривую
        let formula = parser::parse("x^2").unwrap();
        let adaptive = PlotData::compute_adaptive(&formula, 0.0, 5.0, 10, 0.01);
        
        // Проверяем, что точки лежат на кривой y = x^2
        for (x, y) in &adaptive.points {
            if y.is_finite() {
                let expected = x * x;
                let error = (y - expected).abs();
                assert!(error < 0.1, "При x={:.4}: got {:.4}, expected {:.4}, error={:.4}",
                    x, y, expected, error);
            }
        }
    }

    #[test]
    fn test_adaptive_vs_uniform_efficiency() {
        // Адаптивный алгоритм должен использовать меньше точек для гладких функций
        let formula = parser::parse("sin(x)").unwrap();
        
        let uniform = PlotData::compute(&formula, 0.0, std::f64::consts::PI, 100);
        let adaptive = PlotData::compute_adaptive(&formula, 0.0, std::f64::consts::PI, 10, 0.01);
        
        // Адаптивный должен дать меньше точек
        assert!(adaptive.points.len() < uniform.points.len(),
            "Адаптивный: {} точек, равномерный: {} точек",
            adaptive.points.len(), uniform.points.len());
        
        // Но обе должны примерно совпадать по значениям
        let uniform_last = uniform.points.last().unwrap().1;
        let adaptive_last = adaptive.points.last().unwrap().1;
        
        assert!(uniform_last.abs() < 0.1, "Равномерный: sin(PI) = {:.6}", uniform_last);
        assert!(adaptive_last.abs() < 0.1, "Адаптивный: sin(PI) = {:.6}", adaptive_last);
    }

    #[test]
    fn test_adaptive_high_frequency() {
        // Для высокочастотной функции адаптивный алгоритм должен добавить больше точек
        let formula = parser::parse("sin(10*x)").unwrap();
        let adaptive = PlotData::compute_adaptive(&formula, 0.0, 1.0, 15, 0.0001);
        
        // Должно быть много точек для высокочастотной функции
        assert!(adaptive.points.len() > 50,
            "Для высокочастотной функции ожидалось > 50 точек, получено {}",
            adaptive.points.len());
    }

    #[test]
    fn test_numerical_derivative_at_multiple_points() {
        // Тест производной cos(x) в нескольких точках
        let formula = parser::parse("deriv(sin(x))").unwrap();
        let data = PlotData::compute(&formula, -std::f64::consts::PI, std::f64::consts::PI, 101);
        
        // Проверяем все точки в данных
        for &(x, y) in &data.points {
            if y.is_finite() {
                let expected = x.cos();
                let error = (y - expected).abs();
                assert!(error < 1e-9,
                    "При x={:.4}: got {:.12}, expected {:.12}, error={:.2e}",
                    x, y, expected, error);
            }
        }
    }

    #[test]
    fn test_numerical_integral_cumulative() {
        // Проверка, что интеграл монотонно растёт для положительной функции
        let formula = parser::parse("integral(x^2, 0, 1)").unwrap();
        let data = PlotData::compute(&formula, 0.0, 1.0, 11);
        
        let mut prev_y = f64::NEG_INFINITY;
        for &(x, y) in &data.points {
            if y.is_finite() && x >= 0.0 {
                assert!(y >= prev_y, "Интеграл от x^2 должен монотонно расти: x={:.4}, y={:.6}", x, y);
                prev_y = y;
            }
        }
        
        // Финальное значение должно быть близко к 1/3
        let last_y = data.points.last().unwrap().1;
        assert!((last_y - 1.0/3.0).abs() < 1e-5,
            "∫₀¹ x² dx = {:.10}, ожидалось {:.10}", last_y, 1.0/3.0);
    }

    #[test]
    fn test_adaptive_tolerance_effect() {
        // Меньший tolerance должен дать больше точек
        let formula = parser::parse("sin(5*x)").unwrap();
        
        let loose = PlotData::compute_adaptive(&formula, 0.0, std::f64::consts::PI, 10, 0.1);
        let strict = PlotData::compute_adaptive(&formula, 0.0, std::f64::consts::PI, 10, 0.001);
        
        assert!(strict.points.len() > loose.points.len(),
            "Более строгий tolerance должен дать больше точек: loose={}, strict={}",
            loose.points.len(), strict.points.len());
    }
}
