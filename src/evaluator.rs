use crate::parser::ParsedFormula;

#[derive(Clone, Debug)]
pub struct PlotData {
    pub points: Vec<(f64, f64)>,
    pub y_min: f64,
    pub y_max: f64,
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

        for i in 0..n {
            let x = x_min + dx * i as f64;
            let y = formula.eval(x);

            if y.is_finite() {
                y_min = y_min.min(y);
                y_max = y_max.max(y);
            }
            points.push((x, y));
        }

        if !y_min.is_finite() || !y_max.is_finite() {
            y_min = -1.0;
            y_max = 1.0;
        }

        PlotData { points, y_min, y_max }
    }
}
