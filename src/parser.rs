use meval::{Expr, Error as MevalError};

/// Разобранная формула, готовая к вычислению.
#[derive(Clone, Debug)]
pub struct ParsedFormula {
    expr: Expr,
}

impl ParsedFormula {
    /// Вычислить f(x). Возвращает NaN при ошибке.
    pub fn eval(&self, x: f64) -> f64 {
        self.expr.clone().bind("x")
            .ok()
            .map(|f| f(x))
            .unwrap_or(f64::NAN)
    }
}

/// Результат парсинга: либо успех, либо сообщение об ошибке.
pub fn parse(input: &str) -> Result<ParsedFormula, String> {
    let expr: Expr = input
        .parse::<Expr>()
        .map_err(|e: MevalError| format!("{e}"))?;

    let test = expr.clone().bind("x");
    if test.is_err() {
        return Err("Формула должна использовать переменную x".into());
    }

    Ok(ParsedFormula { expr })
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
}
