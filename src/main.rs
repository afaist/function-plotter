mod app;
mod config;
mod evaluator;
mod export;
mod parser;
mod renderer;
mod session;
mod templates;
pub mod theme;
mod ui;

use std::path::PathBuf;

pub use export::save_dialog;
pub use export::open_csv_dialog;
pub use export::load_csv;

fn main() -> eframe::Result {
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

            // Инициализируем тему из config
            app.current_theme = theme::ThemeKind::from_string(&config.theme)
                .unwrap_or(theme::ThemeKind::Dark);

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

    #[test]
    fn test_extract_coeffs_with_minus_c() {
        let mut graph = GraphEntry::new("f1", Color32::RED, "x^2-5");
        assert!(graph.is_quadratic());

        let extracted = graph.extract_quadratic_coeffs();
        assert!(extracted, "Коэффициенты должны извлечься");

        assert_eq!(graph.slider_a, 1.0, "a должно быть 1");
        assert_eq!(graph.slider_b, 0.0, "b должно быть 0");
        assert_eq!(graph.slider_c, -5.0, "c должно быть -5");
    }

    #[test]
    fn test_extract_coeffs_with_plus_c() {
        let mut graph = GraphEntry::new("f1", Color32::RED, "x^2+3");
        assert!(graph.is_quadratic());

        let extracted = graph.extract_quadratic_coeffs();
        assert!(extracted, "Коэффициенты должны извлечься");

        assert_eq!(graph.slider_a, 1.0, "a должно быть 1");
        assert_eq!(graph.slider_b, 0.0, "b должно быть 0");
        assert_eq!(graph.slider_c, 3.0, "c должно быть 3");
    }

    #[test]
    fn test_extract_coeffs_negative_c() {
        let mut graph = GraphEntry::new("f1", Color32::RED, "2*x^2+3*x-7");
        assert!(graph.is_quadratic());

        let extracted = graph.extract_quadratic_coeffs();
        assert!(extracted, "Коэффициенты должны извлечься");

        assert_eq!(graph.slider_a, 2.0, "a должно быть 2");
        assert_eq!(graph.slider_b, 3.0, "b должно быть 3");
        assert_eq!(graph.slider_c, -7.0, "c должно быть -7");
    }

    #[test]
    fn test_extract_coeffs_with_spaces() {
        let mut graph = GraphEntry::new("f1", Color32::RED, "4*x^2 + 3*x - 3");
        assert!(graph.is_quadratic());

        let extracted = graph.extract_quadratic_coeffs();
        assert!(extracted, "Коэффициенты должны извлечься");

        assert_eq!(graph.slider_a, 4.0, "a должно быть 4");
        assert_eq!(graph.slider_b, 3.0, "b должно быть 3");
        assert_eq!(graph.slider_c, -3.0, "c должно быть -3");
    }

    #[test]
    fn test_apply_sliders_updates_formula() {
        let mut graph = GraphEntry::new("f1", Color32::RED, "x^2");
        graph.slider_a = 2.0;
        graph.slider_b = 3.0;
        graph.slider_c = -5.0;
        graph.use_sliders = true;

        graph.apply_sliders();

        assert!(graph.formula_text.contains("2"), "Формула должна содержать 2*x^2");
        assert!(graph.formula_text.contains("3"), "Формула должна содержать 3*x");
        assert!(graph.formula_text.contains("5"), "Формула должна содержать 5");
        assert!(graph.parsed.is_some(), "Формула должна парситься");
    }

    #[test]
    fn test_extract_coeffs_fraction_a() {
        let mut graph = GraphEntry::new("f1", Color32::RED, "1/10*x^2");
        assert!(graph.is_quadratic());
        
        let extracted = graph.extract_quadratic_coeffs();
        assert!(extracted, "Коэффициенты должны извлечься");
        
        assert!((graph.slider_a - 0.1).abs() < 1e-10, "a должно быть 0.1, got {}", graph.slider_a);
    }

    #[test]
    fn test_extract_coeffs_fraction_b() {
        let mut graph = GraphEntry::new("f1", Color32::RED, "x^2 + 1/3*x");
        assert!(graph.is_quadratic());
        
        let extracted = graph.extract_quadratic_coeffs();
        assert!(extracted, "Коэффициенты должны извлечься");
        
        assert!((graph.slider_b - 1.0/3.0).abs() < 1e-10, "b должно быть 1/3, got {}", graph.slider_b);
    }

    #[test]
    fn test_extract_coeffs_fraction_c() {
        let mut graph = GraphEntry::new("f1", Color32::RED, "x^2 - 1/2");
        assert!(graph.is_quadratic());
        
        let extracted = graph.extract_quadratic_coeffs();
        assert!(extracted, "Коэффициенты должны извлечься");
        
        assert!((graph.slider_c + 0.5).abs() < 1e-10, "c должно быть -0.5, got {}", graph.slider_c);
    }

    #[test]
    fn test_extract_coeffs_mixed_fractions() {
        let mut graph = GraphEntry::new("f1", Color32::RED, "1/2*x^2 + 2/3*x - 3/4");
        assert!(graph.is_quadratic());
        
        let extracted = graph.extract_quadratic_coeffs();
        assert!(extracted, "Коэффициенты должны извлечься");
        
        assert!((graph.slider_a - 0.5).abs() < 1e-10, "a = 0.5, got {}", graph.slider_a);
        assert!((graph.slider_b - 2.0/3.0).abs() < 1e-10, "b = 2/3, got {}", graph.slider_b);
        assert!((graph.slider_c + 0.75).abs() < 1e-10, "c = -0.75, got {}", graph.slider_c);
    }
}
