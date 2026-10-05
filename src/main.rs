pub mod app;
pub mod config;
pub mod evaluator;
pub mod export;
pub mod parser;
pub mod renderer;
pub mod session;
pub mod templates;
pub mod ui;

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

    // --- Тесты формулы: симуляция пользовательского ввода ---

    #[test]
    fn test_graph_initial_formula_x() {
        let graph = GraphEntry::new("f1", Color32::RED, "x");
        assert!(graph.parsed.is_some(), "Формула 'x' должна парситься");
        assert!(graph.parse_error.is_none(), "parse_error должен быть None");
        assert!(graph.dirty, "График должен быть dirty после создания");
    }

    #[test]
    fn test_graph_change_to_invalid_formula() {
        let mut graph = GraphEntry::new("f1", Color32::RED, "x");
        graph.formula_text = "abc".to_string();
        graph.reparse();
        assert!(graph.parsed.is_none(), "Невалидная формула 'abc' не должна парситься");
        assert!(graph.parse_error.is_some(), "parse_error должен быть установлен для 'abc'");
        assert!(graph.dirty, "График должен быть dirty после изменения формулы");
    }

    #[test]
    fn test_graph_change_to_valid_formula() {
        let mut graph = GraphEntry::new("f1", Color32::RED, "x");
        graph.formula_text = "sin(x)".to_string();
        graph.reparse();
        assert!(graph.parsed.is_some(), "Формула 'sin(x)' должна парситься");
        assert!(graph.parse_error.is_none(), "parse_error должен быть None для 'sin(x)'");
        assert!(graph.dirty, "График должен быть dirty после изменения формулы");
    }

    #[test]
    fn test_graph_change_to_empty_formula() {
        let mut graph = GraphEntry::new("f1", Color32::RED, "x");
        graph.formula_text = "".to_string();
        graph.reparse();
        assert!(graph.parsed.is_none(), "Пустая строка не должна парситься");
        assert!(graph.parse_error.is_some(), "parse_error должен быть установлен для пустой строки");
    }

    #[test]
    fn test_graph_formula_text_persists() {
        let mut graph = GraphEntry::new("f1", Color32::RED, "x^2");
        assert_eq!(graph.formula_text, "x^2");
        graph.formula_text = "x^3 + 2*x".to_string();
        assert_eq!(graph.formula_text, "x^3 + 2*x");
        graph.reparse();
        assert!(graph.parsed.is_some(), "Формула 'x^3 + 2*x' должна парситься");
    }

    #[test]
    fn test_plotapp_multiple_graphs_formula_change() {
        let mut app = PlotApp::default();
        assert_eq!(app.graphs[0].formula_text, "sin(x)");
        assert_eq!(app.graphs[1].formula_text, "x^2 / 10");
        
        app.graphs[0].formula_text = "!!!".to_string();
        app.graphs[0].reparse();
        assert!(app.graphs[0].parsed.is_none(), "Первый график не должен парситься");
        assert!(app.graphs[0].parse_error.is_some(), "Первый график должен иметь parse_error");
        assert!(app.graphs[1].parsed.is_some(), "Второй график должен оставаться валидным");
        assert!(app.graphs[1].parse_error.is_none(), "Второй график не должен иметь parse_error");
        
        app.graphs[0].formula_text = "cos(x)".to_string();
        app.graphs[0].reparse();
        assert!(app.graphs[0].parsed.is_some(), "cos(x) должен парситься");
        assert!(app.graphs[0].parse_error.is_none(), "parse_error должен быть None для cos(x)");
    }

    #[test]
    fn test_recompute_all_after_formula_change() {
        let mut app = PlotApp::default();
        app.graphs[0].formula_text = "bad!!".to_string();
        app.graphs[0].reparse();
        app.recompute_all();
        assert!(app.graphs[0].data.is_none(), "Невалидный график не должен иметь данных");
        assert!(app.graphs[1].data.is_some(), "Валидный график должен иметь данные");
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
