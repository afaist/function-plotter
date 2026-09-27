use meval::{Expr, Error as MevalError};

/// Разобранная формула, готовая к вычислению.
#[derive(Clone)]
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
