mod app;
mod config;
mod evaluator;
mod export;
mod parser;
mod renderer;
mod session;
mod templates;
mod ui;

use std::path::PathBuf;

pub use export::save_dialog;
pub use export::open_csv_dialog;
pub use export::load_csv;

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
    // Загружаем конфигурацию
    let config = config::AppConfig::load();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size((config.window_width as f32, config.window_height as f32))
            .with_min_inner_size((600.0, 400.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Function Plotter",
        options,
        Box::new(|_cc| {
            let mut app = app::PlotApp::default();

            // Загружаем конфигурацию и инициализируем last_session_path
            let config = config::AppConfig::load();
            if let Some(ref path_str) = config.last_session_path {
                app.last_session_path = Some(PathBuf::from(path_str));
            }

            // Восстанавливаем размер окна из config
            app.restore_window_size();

            // Проверяем auto-load
            if config.auto_load_last_session {
                app.try_load_last_session();
            }

            // Устанавливаем флаг: если сессия не сохранена — нужно предложить при выходе
            if app.session_path.is_none() {
                app.pending_save_on_exit = true;
            }

            Ok(Box::new(app))
        }),
    )
}
