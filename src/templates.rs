/// Библиотека шаблонов функций для быстрого добавления.

/// Шаблон функции.
#[derive(Clone, Debug)]
pub struct FunctionTemplate {
    /// Отображаемое имя.
    pub name: &'static str,
    /// Формула для вставки.
    pub formula: &'static str,
    /// Категория шаблона.
    pub category: &'static str,
}

/// Все доступные шаблоны, сгруппированные по категориям.
pub fn all_templates() -> &'static [FunctionTemplate] {
    &[
        // Тригонометрия
        FunctionTemplate {
            name: "sin(x)",
            formula: "sin(x)",
            category: "Тригонометрия",
        },
        FunctionTemplate {
            name: "cos(x)",
            formula: "cos(x)",
            category: "Тригонометрия",
        },
        FunctionTemplate {
            name: "tan(x)",
            formula: "tan(x)",
            category: "Тригонометрия",
        },
        FunctionTemplate {
            name: "sin(x)/x",
            formula: "sin(x)/x",
            category: "Тригонометрия",
        },
        FunctionTemplate {
            name: "sin(2*x)",
            formula: "sin(2*x)",
            category: "Тригонометрия",
        },
        FunctionTemplate {
            name: "cos(x)^2",
            formula: "cos(x)^2",
            category: "Тригонометрия",
        },
        // Полиномы
        FunctionTemplate {
            name: "x",
            formula: "x",
            category: "Полиномы",
        },
        FunctionTemplate {
            name: "x^2",
            formula: "x^2",
            category: "Полиномы",
        },
        FunctionTemplate {
            name: "x^3",
            formula: "x^3",
            category: "Полиномы",
        },
        FunctionTemplate {
            name: "x^2/10",
            formula: "x^2/10",
            category: "Полиномы",
        },
        FunctionTemplate {
            name: "x^3 - 3*x",
            formula: "x^3 - 3*x",
            category: "Полиномы",
        },
        FunctionTemplate {
            name: "x^4",
            formula: "x^4",
            category: "Полиномы",
        },
        // Экспоненты и логарифмы
        FunctionTemplate {
            name: "exp(x)",
            formula: "exp(x)",
            category: "Экспоненты",
        },
        FunctionTemplate {
            name: "e^(-x^2)",
            formula: "e^(-x^2)",
            category: "Экспоненты",
        },
        FunctionTemplate {
            name: "ln(x)",
            formula: "ln(x)",
            category: "Логарифмы",
        },
        FunctionTemplate {
            name: "log(x)",
            formula: "log(x)",
            category: "Логарифмы",
        },
        FunctionTemplate {
            name: "1/x",
            formula: "1/x",
            category: "Логарифмы",
        },
        // Корни и модули
        FunctionTemplate {
            name: "sqrt(x)",
            formula: "sqrt(x)",
            category: "Корни",
        },
        FunctionTemplate {
            name: "cbrt(x)",
            formula: "x^(1.0/3.0)",
            category: "Корни",
        },
        FunctionTemplate {
            name: "abs(x)",
            formula: "abs(x)",
            category: "Модули",
        },
        // Специальные
        FunctionTemplate {
            name: "1/(1+e^(-x))",
            formula: "1/(1+exp(-x))",
            category: "Специальные",
        },
        FunctionTemplate {
            name: "sin(x)*exp(-x)",
            formula: "sin(x)*exp(-x)",
            category: "Специальные",
        },
        FunctionTemplate {
            name: "sin(x)/x",
            formula: "sin(x)/x",
            category: "Специальные",
        },
        // Параметрические
        FunctionTemplate {
            name: "Циклоида: x=t-sin(t), y=1-cos(t)",
            formula: "parametric(t-sin(t), 1-cos(t), 0, 2*pi)",
            category: "Параметрические",
        },
        FunctionTemplate {
            name: "Окружность: x=cos(t), y=sin(t)",
            formula: "parametric(cos(t), sin(t), 0, 2*pi)",
            category: "Параметрические",
        },
        FunctionTemplate {
            name: "Спираль: x=t*cos(t), y=t*sin(t)",
            formula: "parametric(t*cos(t), t*sin(t), 0, 4*pi)",
            category: "Параметрические",
        },
        FunctionTemplate {
            name: "Эллипс: x=3*cos(t), y=2*sin(t)",
            formula: "parametric(3*cos(t), 2*sin(t), 0, 2*pi)",
            category: "Параметрические",
        },
    ]
}

/// Получить категории, отсортированные в определённом порядке.
pub fn categories() -> Vec<&'static str> {
    let mut cats: Vec<&str> = all_templates()
        .iter()
        .map(|t| t.category)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    cats.sort();
    cats
}

/// Получить шаблоны для указанной категории.
pub fn templates_by_category(category: &str) -> Vec<&FunctionTemplate> {
    all_templates()
        .iter()
        .filter(|t| t.category == category)
        .collect()
}
