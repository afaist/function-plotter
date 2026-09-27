use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

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
