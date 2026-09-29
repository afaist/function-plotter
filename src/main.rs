mod app;
mod evaluator;
mod export;
mod parser;
mod renderer;
mod session;
mod ui;

pub use export::save_dialog;
pub use export::save_png;

#[cfg(test)]
mod app_tests {
    use egui::Color32;

    use crate::app::{GraphEntry, PlotApp};

    #[test]
    fn test_find_intersections_x_and_sin() {
        let mut app = PlotApp::default();
        app.graphs.clear();
        app.graphs.push(GraphEntry::new("f1", Color32::RED, "x"));
        app.graphs
            .push(GraphEntry::new("f2", Color32::BLUE, "sin(x)"));
        app.recompute_all();

        let intersections = app.find_intersections();
        assert!(!intersections.is_empty(), "Ожидаем пересечение x=sin(x)");
        let first = &intersections[0];
        assert!(
            first.x.abs() < 0.5,
            "Пересечение должно быть около 0, got {}",
            first.x
        );
        assert!(
            first.y.abs() < 0.5,
            "Y пересечения должно быть около 0, got {}",
            first.y
        );
    }

    #[test]
    fn test_find_intersections_no_intersection() {
        let mut app = PlotApp::default();
        app.graphs.clear();
        app.graphs
            .push(GraphEntry::new("f1", Color32::RED, "x + 10"));
        app.graphs
            .push(GraphEntry::new("f2", Color32::BLUE, "x - 10"));
        app.recompute_all();

        let intersections = app.find_intersections();
        assert!(intersections.is_empty(), "Не должно быть пересечений");
    }

    #[test]
    fn test_find_intersections_single_graph() {
        let mut app = PlotApp::default();
        app.graphs.clear();
        app.graphs.push(GraphEntry::new("f1", Color32::RED, "x"));
        app.recompute_all();

        let intersections = app.find_intersections();
        assert!(intersections.is_empty(), "Нужны минимум 2 графика");
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Function Plotter",
        options,
        Box::new(|_cc| Ok(Box::new(app::PlotApp::default()))),
    )
}
