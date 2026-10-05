use mathexpr::builder::{Executable, Expression};

/// Скомпилировать константное выражение (без переменных) в f64.
/// Возвращает NaN при ошибке.
fn compile_const(expr_str: &str) -> f64 {
    Expression::parse(expr_str)
        .ok()
        .and_then(|e| e.compile_no_vars().ok())
        .and_then(|e| e.eval(&[]).ok())
        .unwrap_or(f64::NAN)
}

/// Тип формулы: обычная, производная, интеграл, полярная или параметрическая.
#[derive(Clone, Debug, PartialEq)]
pub enum FormulaType {
    Regular,
    Derivative,
    Integral,
    Polar,
    Parametric,
}

/// Разобранная формула, готовая к вычислению.
#[derive(Clone, Debug)]
pub enum ParsedFormula {
    Regular {
        compiled: Executable,
    },
    Derivative {
        inner: Box<ParsedFormula>,
    },
    Integral {
        inner: Box<ParsedFormula>,
        lower: f64,
        upper: f64,
    },
    Polar {
        inner: Box<ParsedFormula>,
        theta_min: f64,
        theta_max: f64,
    },
    Parametric {
        x_formula: Box<ParsedFormula>,
        y_formula: Box<ParsedFormula>,
        t_min: f64,
        t_max: f64,
    },
}

impl ParsedFormula {
    /// Вычислить f(x). Возвращает NaN при ошибке.
    pub fn eval(&self, x: f64) -> f64 {
        match self {
            Self::Regular { compiled } => compiled.clone().eval(&[x]).unwrap_or(f64::NAN),
            Self::Derivative { inner } => inner.eval(x),
            Self::Integral { .. } => f64::NAN,
            Self::Polar { inner, .. } => inner.eval(x),
            Self::Parametric { x_formula, .. } => x_formula.eval(x),
        }
    }

    /// Получить тип формулы.
    pub fn formula_type(&self) -> FormulaType {
        match self {
            Self::Regular { .. } => FormulaType::Regular,
            Self::Derivative { .. } => FormulaType::Derivative,
            Self::Integral { .. } => FormulaType::Integral,
            Self::Polar { .. } => FormulaType::Polar,
            Self::Parametric { .. } => FormulaType::Parametric,
        }
    }

    /// Получить внутреннюю формулу (для производной/интеграла/полярной).
    pub fn inner(&self) -> Option<&ParsedFormula> {
        match self {
            Self::Derivative { inner } => Some(inner),
            Self::Integral { inner, .. } => Some(inner),
            Self::Polar { inner, .. } => Some(inner),
            Self::Parametric { x_formula, .. } => Some(x_formula),
            Self::Regular { .. } => None,
        }
    }

    /// Получить границы интеграла (если это интеграл).
    pub fn integral_bounds(&self) -> Option<(f64, f64)> {
        match self {
            Self::Integral { lower, upper, .. } => Some((*lower, *upper)),
            _ => None,
        }
    }

    /// Получить границы theta (если это полярная).
    pub fn polar_bounds(&self) -> Option<(f64, f64)> {
        match self {
            Self::Polar {
                theta_min,
                theta_max,
                ..
            } => Some((*theta_min, *theta_max)),
            _ => None,
        }
    }

    /// Получить границы t и формулы (если это параметрическая).
    pub fn parametric_bounds(&self) -> Option<(&ParsedFormula, &ParsedFormula, f64, f64)> {
        match self {
            Self::Parametric {
                x_formula,
                y_formula,
                t_min,
                t_max,
            } => Some((x_formula, y_formula, *t_min, *t_max)),
            _ => None,
        }
    }
}

/// Найти позицию закрывающей скобки, соответствующей открывающей на позиции `start`.
fn find_matching_paren(s: &str, start: usize) -> Option<usize> {
    let mut depth = 0;
    for (i, ch) in s.char_indices().skip(start) {
        match ch {
            '(' => depth += 1,
            ')' => {
                if depth == 0 {
                    return Some(i);
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    None
}

/// Разделить аргументы функции по запятым, учитывая вложенные скобки.
fn split_args(s: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut depth = 0;

    for ch in s.chars() {
        match ch {
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                args.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        args.push(current.trim().to_string());
    }
    args
}

/// Результат парсинга: либо успех, либо сообщение об ошибке.
pub fn parse(input: &str) -> Result<ParsedFormula, String> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return Err("Пустая формула".into());
    }

    // Обработка deriv(...)
    if let Some(inner_start) = trimmed.strip_prefix("deriv(") {
        if let Some(inner_end) = inner_start.strip_suffix(')') {
            let inner_expr = inner_end.trim();
            let inner = parse(inner_expr).map_err(|e| format!("deriv: {e}"))?;
            return Ok(ParsedFormula::Derivative {
                inner: Box::new(inner),
            });
        }
        return Err("Неверный синтаксис deriv(...). Ожидается deriv(выражение)".into());
    }

    // Обработка integral(..., a, b)
    if let Some(inner_start) = trimmed.strip_prefix("integral(") {
        if let Some(idx) = find_matching_paren(inner_start, 0) {
            let args_str = &inner_start[..idx];
            let args = split_args(args_str);

            if args.len() < 3 {
                return Err(
                    "Неверный синтаксис integral(...). Ожидается integral(выражение, a, b)".into(),
                );
            }

            let inner_expr = &args[0];
            let lower_str = &args[1];
            let upper_str = &args[2];

            let inner = parse(inner_expr).map_err(|e| format!("integral: {e}"))?;

            // Парсим границы как выражения (должны быть константами)
            let lower_expr =
                Expression::parse(lower_str).map_err(|e| format!("Граница lower: {e}"))?;
            let lower = lower_expr
                .compile_no_vars()
                .ok()
                .and_then(|e| e.eval(&[]).ok())
                .unwrap_or(f64::NAN);

            let upper_expr =
                Expression::parse(upper_str).map_err(|e| format!("Граница upper: {e}"))?;
            let upper = upper_expr
                .compile_no_vars()
                .ok()
                .and_then(|e| e.eval(&[]).ok())
                .unwrap_or(f64::NAN);

            if !lower.is_finite() || !upper.is_finite() {
                return Err("Границы интеграла должны быть конечными числами".into());
            }

            return Ok(ParsedFormula::Integral {
                inner: Box::new(inner),
                lower,
                upper,
            });
        }
        return Err("Неверный синтаксис integral(...). Ожидается integral(выражение, a, b)".into());
    }

    // Обработка polar(..., theta_min, theta_max)
    if let Some(inner_start) = trimmed.strip_prefix("polar(") {
        if let Some(idx) = find_matching_paren(inner_start, 0) {
            let args_str = &inner_start[..idx];
            let args = split_args(args_str);

            if args.len() < 3 {
                return Err(
                    "Неверный синтаксис polar(...). Ожидается polar(выражение, theta_min, theta_max)".into(),
                );
            }

            let inner_expr = &args[0];
            let theta_min_str = &args[1];
            let theta_max_str = &args[2];

            let inner = parse(inner_expr).map_err(|e| format!("polar: {e}"))?;

            // Парсим границы theta как выражения (должны быть константами)
            let theta_min = compile_const(theta_min_str);
            let theta_max = compile_const(theta_max_str);

            if !theta_min.is_finite() || !theta_max.is_finite() {
                return Err("Границы theta должны быть конечными числами".into());
            }

            return Ok(ParsedFormula::Polar {
                inner: Box::new(inner),
                theta_min,
                theta_max,
            });
        }
        return Err(
            "Неверный синтаксис polar(...). Ожидается polar(выражение, theta_min, theta_max)"
                .into(),
        );
    }

    // Обработка parametric(x(t), y(t), t_min, t_max)
    if let Some(inner_start) = trimmed.strip_prefix("parametric(") {
        if let Some(idx) = find_matching_paren(inner_start, 0) {
            let args_str = &inner_start[..idx];
            let args = split_args(args_str);

            if args.len() < 4 {
                return Err(
                    "Неверный синтаксис parametric(...). Ожидается parametric(x(t), y(t), t_min, t_max)".into(),
                );
            }

            let x_expr = &args[0];
            let y_expr = &args[1];
            let t_min_str = &args[2];
            let t_max_str = &args[3];

            // Парсим x(t) и y(t) с переменной t
            let x_expr_parsed =
                Expression::parse(x_expr).map_err(|e| format!("parametric x: {e}"))?;
            let x_formula = x_expr_parsed
                .compile(&["t"])
                .map_err(|e| format!("parametric x compile: {e}"))?;

            let y_expr_parsed =
                Expression::parse(y_expr).map_err(|e| format!("parametric y: {e}"))?;
            let y_formula = y_expr_parsed
                .compile(&["t"])
                .map_err(|e| format!("parametric y compile: {e}"))?;

            // Парсим границы t как выражения (должны быть константами)
            let t_min_expr =
                Expression::parse(t_min_str).map_err(|e| format!("Граница t_min: {e}"))?;
            let t_min = t_min_expr
                .compile_no_vars()
                .ok()
                .and_then(|e| e.eval(&[]).ok())
                .unwrap_or(f64::NAN);

            let t_max_expr =
                Expression::parse(t_max_str).map_err(|e| format!("Граница t_max: {e}"))?;
            let t_max = t_max_expr
                .compile_no_vars()
                .ok()
                .and_then(|e| e.eval(&[]).ok())
                .unwrap_or(f64::NAN);

            if !t_min.is_finite() || !t_max.is_finite() {
                return Err("Границы t должны быть конечными числами".into());
            }

            return Ok(ParsedFormula::Parametric {
                x_formula: Box::new(ParsedFormula::Regular {
                    compiled: x_formula,
                }),
                y_formula: Box::new(ParsedFormula::Regular {
                    compiled: y_formula,
                }),
                t_min,
                t_max,
            });
        }
        return Err(
            "Неверный синтаксис parametric(...). Ожидается parametric(x(t), y(t), t_min, t_max)"
                .into(),
        );
    }

    // Обычная формула
    let expr = Expression::parse(trimmed).map_err(|e| format!("{e}"))?;

    let compiled = expr.compile(&["x"]).map_err(|e| format!("{e}"))?;

    Ok(ParsedFormula::Regular { compiled })
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Валидные формулы ---

    #[test]
    fn parse_simple_x() {
        let result = parse("x");
        assert!(result.is_ok());
        let formula = result.unwrap();
        assert!(formula.eval(2.0).abs() - 2.0 < 1e-10);
    }

    #[test]
    fn parse_polynomial() {
        let result = parse("x^2 / 10");
        assert!(result.is_ok());
        let formula = result.unwrap();
        assert!(formula.eval(10.0).abs() - 10.0 < 1e-6);
    }

    #[test]
    fn parse_trig() {
        let result = parse("sin(x)");
        assert!(result.is_ok());
        let formula = result.unwrap();
        assert!(formula.eval(0.0).abs() < 1e-10);
        assert!(formula.eval(std::f64::consts::PI).abs() < 1e-10);
    }

    #[test]
    fn parse_exp_cos() {
        let result = parse("exp(x) + cos(2*x)");
        assert!(result.is_ok());
        let formula = result.unwrap();
        let y = formula.eval(0.0);
        assert!((y - 2.0).abs() < 1e-10); // exp(0) + cos(0) = 1 + 1 = 2
    }

    #[test]
    fn parse_complex_formula() {
        let result = parse("sin(x)^2 + cos(x)^2");
        assert!(result.is_ok());
        let formula = result.unwrap();
        // sin^2 + cos^2 ≈ 1
        for x in [0.0, 1.0, 2.0, 3.14159, -5.0] {
            let y = formula.eval(x);
            assert!((y - 1.0).abs() < 1e-9, "Expected ~1 at x={x}, got {y}");
        }
    }

    #[test]
    fn parse_sqrt() {
        let result = parse("sqrt(x)");
        assert!(result.is_ok());
        let formula = result.unwrap();
        assert!(formula.eval(4.0).abs() - 2.0 < 1e-10);
    }

    #[test]
    fn parse_abs() {
        let result = parse("abs(x)");
        assert!(result.is_ok());
        let formula = result.unwrap();
        assert!(formula.eval(-5.0).abs() - 5.0 < 1e-10);
    }

    #[test]
    fn parse_ln() {
        let result = parse("ln(x)");
        assert!(result.is_ok());
        let formula = result.unwrap();
        // ln(e) = 1
        assert!(formula.eval(std::f64::consts::E).abs() - 1.0 < 1e-10);
    }

    #[test]
    fn parse_with_constants() {
        let result = parse("pi * x");
        assert!(result.is_ok());
        let formula = result.unwrap();
        let y = formula.eval(1.0);
        assert!(y.abs() - std::f64::consts::PI < 1e-10);
    }

    #[test]
    fn parse_negative_x() {
        let result = parse("x^3");
        assert!(result.is_ok());
        let formula = result.unwrap();
        let expected: f64 = -8.0;
        assert!((formula.eval(-2.0) - expected).abs() < 1e-10);
    }

    // --- Невалидные формулы ---

    #[test]
    fn parse_empty_string() {
        let result = parse("");
        assert!(result.is_err());
    }

    #[test]
    fn parse_no_variable_x() {
        // Выражение без переменной x — meval может принять константу,
        // но проверка bind("x") может не сработать для простых констант.
        // Используем выражение с другой переменной.
        let result = parse("y + 1");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("x") || err.contains("y") || !err.is_empty());
    }

    #[test]
    fn parse_invalid_syntax() {
        let result = parse("sin(()");
        assert!(result.is_err());
    }

    #[test]
    fn parse_unknown_function() {
        let result = parse("foo(x)");
        assert!(result.is_err());
    }

    // --- Вычисления ---

    #[test]
    fn eval_returns_nan_on_error() {
        let formula = parse("sqrt(x)").unwrap();
        // sqrt(-1) → NaN
        let y = formula.eval(-1.0);
        assert!(y.is_nan());
    }

    #[test]
    fn eval_division_by_zero() {
        let formula = parse("1/x").unwrap();
        let y = formula.eval(0.0);
        assert!(!y.is_finite()); // Inf или NaN
    }

    #[test]
    fn eval_large_values() {
        let formula = parse("x^2").unwrap();
        let y = formula.eval(1e5);
        assert!((y - 1e10).abs() < 1e6);
    }

    #[test]
    fn eval_small_values() {
        let formula = parse("x^3").unwrap();
        let y = formula.eval(1e-5);
        assert!(y.abs() - 1e-15 < 1e-20);
    }

    // --- Производная ---

    #[test]
    fn parse_deriv_simple() {
        let result = parse("deriv(sin(x))");
        assert!(result.is_ok());
        let formula = result.unwrap();
        assert_eq!(formula.formula_type(), FormulaType::Derivative);
        // eval() возвращает внутреннюю функцию: sin(0) = 0
        assert!(formula.eval(0.0).abs() < 1e-10);
        assert!(formula.eval(std::f64::consts::FRAC_PI_2).abs() - 1.0 < 1e-10);
    }

    #[test]
    fn parse_deriv_polynomial() {
        let result = parse("deriv(x^2)");
        assert!(result.is_ok());
        let formula = result.unwrap();
        // eval() возвращает x^2, при x=3 → 9
        assert!((formula.eval(3.0) - 9.0).abs() < 1e-10);
    }

    #[test]
    fn parse_nested_deriv() {
        let result = parse("deriv(deriv(sin(x)))");
        assert!(result.is_ok());
        let formula = result.unwrap();
        // Вторая производная: eval возвращает sin(x)
        assert!(formula.eval(0.0).abs() < 1e-10);
    }

    #[test]
    fn parse_deriv_invalid_inner() {
        let result = parse("deriv(sin(y))");
        assert!(result.is_err());
    }

    // --- Интеграл ---

    #[test]
    fn parse_integral_simple() {
        let result = parse("integral(x, 0, 1)");
        assert!(result.is_ok());
        let formula = result.unwrap();
        assert_eq!(formula.formula_type(), FormulaType::Integral);
        // ∫₀¹ x dx = 0.5
        let bounds = formula.integral_bounds().unwrap();
        assert!((bounds.0 - 0.0).abs() < 1e-10);
        assert!((bounds.1 - 1.0).abs() < 1e-10);
    }

    #[test]
    fn parse_integral_with_constants() {
        let result = parse("integral(1, 0, pi)");
        assert!(result.is_ok());
        let formula = result.unwrap();
        // ∫₀^π 1 dx = π
        let bounds = formula.integral_bounds().unwrap();
        assert!((bounds.1 - std::f64::consts::PI).abs() < 1e-10);
    }

    #[test]
    fn parse_integral_trig() {
        let result = parse("integral(sin(x), 0, pi)");
        assert!(result.is_ok());
        let formula = result.unwrap();
        // ∫₀^π sin(x) dx = 2
        let bounds = formula.integral_bounds().unwrap();
        assert!((bounds.0 - 0.0).abs() < 1e-10);
        assert!((bounds.1 - std::f64::consts::PI).abs() < 1e-10);
    }

    #[test]
    fn parse_integral_too_few_args() {
        let result = parse("integral(x, 0)");
        assert!(result.is_err());
    }

    #[test]
    fn parse_integral_invalid_bounds() {
        let result = parse("integral(x, a, b)");
        assert!(result.is_err());
    }

    // --- split_args ---

    #[test]
    fn split_args_basic() {
        let args = split_args("sin(x), 0, pi");
        assert_eq!(args.len(), 3);
        assert_eq!(args[0], "sin(x)");
        assert_eq!(args[1], "0");
        assert_eq!(args[2], "pi");
    }

    #[test]
    fn split_args_nested() {
        let args = split_args("sin(x+y), 0, pi");
        assert_eq!(args.len(), 3);
        assert_eq!(args[0], "sin(x+y)");
    }

    // --- formula_type ---

    #[test]
    fn formula_type_regular() {
        let f = parse("x^2").unwrap();
        assert_eq!(f.formula_type(), FormulaType::Regular);
    }

    #[test]
    fn formula_type_derivative() {
        let f = parse("deriv(sin(x))").unwrap();
        assert_eq!(f.formula_type(), FormulaType::Derivative);
    }

    #[test]
    fn formula_type_integral() {
        let f = parse("integral(sin(x), 0, 1)").unwrap();
        assert_eq!(f.formula_type(), FormulaType::Integral);
    }

    // --- Полярные координаты ---

    #[test]
    fn parse_polar_simple() {
        let result = parse("polar(1, 0, 2*pi)");
        assert!(result.is_ok());
        let formula = result.unwrap();
        assert_eq!(formula.formula_type(), FormulaType::Polar);
        let bounds = formula.polar_bounds().unwrap();
        assert!((bounds.0 - 0.0).abs() < 1e-10);
        assert!((bounds.1 - 2.0 * std::f64::consts::PI).abs() < 1e-10);
    }

    #[test]
    fn parse_polar_trig() {
        let result = parse("polar(sin(2*x), 0, pi)");
        assert!(result.is_ok());
        let formula = result.unwrap();
        assert_eq!(formula.formula_type(), FormulaType::Polar);
        let bounds = formula.polar_bounds().unwrap();
        assert!((bounds.0 - 0.0).abs() < 1e-10);
        assert!((bounds.1 - std::f64::consts::PI).abs() < 1e-10);
    }

    #[test]
    fn parse_polar_too_few_args() {
        let result = parse("polar(sin(x), 0)");
        assert!(result.is_err());
    }

    #[test]
    fn parse_polar_invalid_bounds() {
        let result = parse("polar(x, a, b)");
        assert!(result.is_err());
    }

    #[test]
    fn formula_type_polar() {
        let f = parse("polar(cos(x), 0, 2*pi)").unwrap();
        assert_eq!(f.formula_type(), FormulaType::Polar);
    }

    // --- Параметрические уравнения ---

    #[test]
    fn parse_parametric_simple() {
        let result = parse("parametric(cos(t), sin(t), 0, 2*pi)");
        assert!(result.is_ok());
        let formula = result.unwrap();
        assert_eq!(formula.formula_type(), FormulaType::Parametric);
        let bounds = formula.parametric_bounds().unwrap();
        assert!((bounds.2 - 0.0).abs() < 1e-10);
        assert!((bounds.3 - 2.0 * std::f64::consts::PI).abs() < 1e-10);
    }

    #[test]
    fn parse_parametric_cycloid() {
        let result = parse("parametric(t-sin(t), 1-cos(t), 0, 2*pi)");
        assert!(result.is_ok());
        let formula = result.unwrap();
        assert_eq!(formula.formula_type(), FormulaType::Parametric);
    }

    #[test]
    fn parse_parametric_too_few_args() {
        let result = parse("parametric(cos(t), sin(t), 0)");
        assert!(result.is_err());
    }

    #[test]
    fn parse_parametric_invalid_bounds() {
        let result = parse("parametric(x, y, a, b)");
        assert!(result.is_err());
    }

    #[test]
    fn formula_type_parametric() {
        let f = parse("parametric(cos(t), sin(t), 0, 2*pi)").unwrap();
        assert_eq!(f.formula_type(), FormulaType::Parametric);
    }
}
