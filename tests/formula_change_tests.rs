//! Тесты для проверки корректности обработки изменений формулы.
//! 
//! Эти тесты симулируют поведение пользователя:
//! 1. Создаётся график с формулой "x"
//! 2. Формула меняется на невалидную (например, "abc")
//! 3. Вызывается reparse()
//! 4. Проверяется, что parse_error установлен, parsed = None

use function_plotter::app::{GraphEntry, PlotApp};
use egui::Color32;

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
    
    // Симулируем ввод пользователем невалидной формулы "abc"
    graph.formula_text = "abc".to_string();
    graph.reparse();
    
    assert!(graph.parsed.is_none(), "Невалидная формула 'abc' не должна парситься");
    assert!(graph.parse_error.is_some(), "parse_error должен быть установлен для 'abc'");
    assert!(graph.dirty, "График должен быть dirty после изменения формулы");
}

#[test]
fn test_graph_change_to_valid_formula() {
    let mut graph = GraphEntry::new("f1", Color32::RED, "x");
    
    // Симулируем ввод пользователем валидной формулы "sin(x)"
    graph.formula_text = "sin(x)".to_string();
    graph.reparse();
    
    assert!(graph.parsed.is_some(), "Формула 'sin(x)' должна парситься");
    assert!(graph.parse_error.is_none(), "parse_error должен быть None для 'sin(x)'");
    assert!(graph.dirty, "График должен быть dirty после изменения формулы");
}

#[test]
fn test_graph_change_to_empty_formula() {
    let mut graph = GraphEntry::new("f1", Color32::RED, "x");
    
    // Симулируем удаление формулы (пустая строка)
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
    
    // Симулируем изменение формулы первого графика на невалидную
    app.graphs[0].formula_text = "!!!".to_string();
    app.graphs[0].reparse();
    
    assert!(app.graphs[0].parsed.is_none(), "Первый график не должен парситься");
    assert!(app.graphs[0].parse_error.is_some(), "Первый график должен иметь parse_error");
    assert!(app.graphs[1].parsed.is_some(), "Второй график должен оставаться валидным");
    assert!(app.graphs[1].parse_error.is_none(), "Второй график не должен иметь parse_error");
    
    // Симулируем изменение первого графика на валидную
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
