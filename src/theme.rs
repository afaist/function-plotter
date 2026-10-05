use egui::Color32;

/// Тема оформления приложения.
pub struct Theme {
    pub name: &'static str,
    pub canvas_bg: Color32,
    pub frame_fill: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub grid_color: Color32,
    pub axis_color: Color32,
    pub error_color: Color32,
    pub error_bg: Color32,
    pub status_info: Color32,
    pub status_error: Color32,
    pub selection_fill: Color32,
    pub selection_stroke: Color32,
    pub intersection_dot: Color32,
    pub intersection_label_bg: Color32,
    pub vertex_color: Color32,
    pub root_color: Color32,
    pub fill_alpha: u8,
    // egui Style
    pub egui_bg: Color32,
    pub egui_text: Color32,
    pub egui_button: Color32,
    pub egui_button_hover: Color32,
    pub egui_selected: Color32,
    pub egui_accent: Color32,
}

/// Доступные темы оформления.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeKind {
    Dark,
    Light,
    System,
    HighContrast,
    Ocean,
    Monokai,
    SolarizedLight,
    GitHubLight,
}

impl ThemeKind {
    /// Все доступные темы.
    pub fn all() -> &'static [ThemeKind] {
        &[
            ThemeKind::Dark,
            ThemeKind::Light,
            ThemeKind::System,
            ThemeKind::HighContrast,
            ThemeKind::Ocean,
            ThemeKind::Monokai,
            ThemeKind::SolarizedLight,
            ThemeKind::GitHubLight,
        ]
    }

    /// Преобразовать тему в строку.
    pub fn to_string(&self) -> &'static str {
        match self {
            ThemeKind::Dark => "Dark",
            ThemeKind::Light => "Light",
            ThemeKind::System => "System",
            ThemeKind::HighContrast => "HighContrast",
            ThemeKind::Ocean => "Ocean",
            ThemeKind::Monokai => "Monokai",
            ThemeKind::SolarizedLight => "SolarizedLight",
            ThemeKind::GitHubLight => "GitHubLight",
        }
    }

    /// Преобразовать строку в тему.
    pub fn from_string(s: &str) -> Option<ThemeKind> {
        match s.to_lowercase().as_str() {
            "dark" => Some(ThemeKind::Dark),
            "light" => Some(ThemeKind::Light),
            "system" => Some(ThemeKind::System),
            "highcontrast" | "high_contrast" => Some(ThemeKind::HighContrast),
            "ocean" => Some(ThemeKind::Ocean),
            "monokai" => Some(ThemeKind::Monokai),
            "solarizedlight" | "solarized_light" => Some(ThemeKind::SolarizedLight),
            "githublight" | "github_light" => Some(ThemeKind::GitHubLight),
            _ => None,
        }
    }

    /// Создать тему по умолчанию для данного типа.
    pub fn default_theme(&self) -> Theme {
        match self {
            ThemeKind::Dark => Theme {
                name: "Dark",
                canvas_bg: Color32::from_rgb(30, 30, 35),
                frame_fill: Color32::from_rgb(40, 40, 50),
                text_primary: Color32::from_rgb(224, 224, 224),
                text_secondary: Color32::from_rgb(180, 180, 180),
                grid_color: Color32::from_rgba_unmultiplied(100, 100, 100, 80),
                axis_color: Color32::from_rgb(120, 120, 120),
                error_color: Color32::from_rgb(255, 80, 80),
                error_bg: Color32::from_rgb(255, 100, 100),
                status_info: Color32::from_rgb(100, 255, 100),
                status_error: Color32::from_rgb(255, 100, 100),
                selection_fill: Color32::from_rgb(40, 40, 50),
                selection_stroke: Color32::from_rgb(100, 150, 255),
                intersection_dot: Color32::WHITE,
                intersection_label_bg: Color32::from_rgb(255, 255, 100),
                vertex_color: Color32::from_rgb(255, 255, 100),
                root_color: Color32::from_rgb(100, 255, 100),
                fill_alpha: 38,
                egui_bg: Color32::from_rgb(25, 25, 30),
                egui_text: Color32::from_rgb(224, 224, 224),
                egui_button: Color32::from_rgb(60, 60, 70),
                egui_button_hover: Color32::from_rgb(70, 70, 80),
                egui_selected: Color32::from_rgb(50, 50, 70),
                egui_accent: Color32::from_rgb(100, 150, 255),
            },
            ThemeKind::Light => Theme {
                name: "Light",
                canvas_bg: Color32::from_rgb(245, 245, 245),
                frame_fill: Color32::from_rgb(235, 235, 235),
                text_primary: Color32::from_rgb(26, 26, 26),
                text_secondary: Color32::from_rgb(80, 80, 80),
                grid_color: Color32::from_rgba_unmultiplied(180, 180, 180, 100),
                axis_color: Color32::from_rgb(100, 100, 100),
                error_color: Color32::from_rgb(220, 50, 50),
                error_bg: Color32::from_rgb(255, 200, 200),
                status_info: Color32::from_rgb(0, 150, 0),
                status_error: Color32::from_rgb(220, 50, 50),
                selection_fill: Color32::from_rgb(220, 220, 230),
                selection_stroke: Color32::from_rgb(50, 100, 200),
                intersection_dot: Color32::from_rgb(0, 0, 0),
                intersection_label_bg: Color32::from_rgb(255, 255, 200),
                vertex_color: Color32::from_rgb(200, 180, 0),
                root_color: Color32::from_rgb(0, 150, 0),
                fill_alpha: 30,
                egui_bg: Color32::from_rgb(240, 240, 240),
                egui_text: Color32::from_rgb(26, 26, 26),
                egui_button: Color32::from_rgb(200, 200, 210),
                egui_button_hover: Color32::from_rgb(180, 180, 190),
                egui_selected: Color32::from_rgb(180, 190, 220),
                egui_accent: Color32::from_rgb(50, 100, 200),
            },
            ThemeKind::System => {
                // Будет разрешено через resolve()
                Theme {
                    name: "System",
                    canvas_bg: Color32::from_rgb(30, 30, 35),
                    frame_fill: Color32::from_rgb(40, 40, 50),
                    text_primary: Color32::from_rgb(224, 224, 224),
                    text_secondary: Color32::from_rgb(180, 180, 180),
                    grid_color: Color32::from_rgba_unmultiplied(100, 100, 100, 80),
                    axis_color: Color32::from_rgb(120, 120, 120),
                    error_color: Color32::from_rgb(255, 80, 80),
                    error_bg: Color32::from_rgb(255, 100, 100),
                    status_info: Color32::from_rgb(100, 255, 100),
                    status_error: Color32::from_rgb(255, 100, 100),
                    selection_fill: Color32::from_rgb(40, 40, 50),
                    selection_stroke: Color32::from_rgb(100, 150, 255),
                    intersection_dot: Color32::WHITE,
                    intersection_label_bg: Color32::from_rgb(255, 255, 100),
                    vertex_color: Color32::from_rgb(255, 255, 100),
                    root_color: Color32::from_rgb(100, 255, 100),
                    fill_alpha: 38,
                    egui_bg: Color32::from_rgb(25, 25, 30),
                    egui_text: Color32::from_rgb(224, 224, 224),
                    egui_button: Color32::from_rgb(60, 60, 70),
                    egui_button_hover: Color32::from_rgb(70, 70, 80),
                    egui_selected: Color32::from_rgb(50, 50, 70),
                    egui_accent: Color32::from_rgb(100, 150, 255),
                }
            }
            ThemeKind::HighContrast => Theme {
                name: "HighContrast",
                canvas_bg: Color32::from_rgb(0, 0, 0),
                frame_fill: Color32::from_rgb(20, 20, 20),
                text_primary: Color32::from_rgb(255, 255, 255),
                text_secondary: Color32::from_rgb(220, 220, 220),
                grid_color: Color32::from_rgba_unmultiplied(200, 200, 200, 150),
                axis_color: Color32::from_rgb(255, 255, 255),
                error_color: Color32::from_rgb(255, 0, 0),
                error_bg: Color32::from_rgb(100, 0, 0),
                status_info: Color32::from_rgb(0, 255, 0),
                status_error: Color32::from_rgb(255, 0, 0),
                selection_fill: Color32::from_rgb(50, 50, 50),
                selection_stroke: Color32::from_rgb(255, 255, 0),
                intersection_dot: Color32::WHITE,
                intersection_label_bg: Color32::from_rgb(0, 0, 0),
                vertex_color: Color32::from_rgb(255, 255, 0),
                root_color: Color32::from_rgb(0, 255, 0),
                fill_alpha: 50,
                egui_bg: Color32::from_rgb(0, 0, 0),
                egui_text: Color32::from_rgb(255, 255, 255),
                egui_button: Color32::from_rgb(40, 40, 40),
                egui_button_hover: Color32::from_rgb(60, 60, 60),
                egui_selected: Color32::from_rgb(80, 80, 0),
                egui_accent: Color32::from_rgb(255, 255, 0),
            },
            ThemeKind::Ocean => Theme {
                name: "Ocean",
                canvas_bg: Color32::from_rgb(13, 27, 42),
                frame_fill: Color32::from_rgb(22, 40, 58),
                text_primary: Color32::from_rgb(230, 237, 243),
                text_secondary: Color32::from_rgb(170, 190, 210),
                grid_color: Color32::from_rgba_unmultiplied(60, 90, 120, 80),
                axis_color: Color32::from_rgb(80, 120, 160),
                error_color: Color32::from_rgb(255, 100, 100),
                error_bg: Color32::from_rgb(100, 40, 40),
                status_info: Color32::from_rgb(100, 220, 150),
                status_error: Color32::from_rgb(255, 120, 100),
                selection_fill: Color32::from_rgb(30, 50, 70),
                selection_stroke: Color32::from_rgb(100, 180, 255),
                intersection_dot: Color32::from_rgb(200, 220, 255),
                intersection_label_bg: Color32::from_rgb(40, 60, 90),
                vertex_color: Color32::from_rgb(255, 220, 100),
                root_color: Color32::from_rgb(100, 220, 150),
                fill_alpha: 35,
                egui_bg: Color32::from_rgb(10, 20, 35),
                egui_text: Color32::from_rgb(230, 237, 243),
                egui_button: Color32::from_rgb(30, 50, 70),
                egui_button_hover: Color32::from_rgb(40, 60, 80),
                egui_selected: Color32::from_rgb(50, 80, 120),
                egui_accent: Color32::from_rgb(100, 180, 255),
            },
            ThemeKind::Monokai => Theme {
                name: "Monokai",
                canvas_bg: Color32::from_rgb(39, 40, 34),
                frame_fill: Color32::from_rgb(49, 50, 44),
                text_primary: Color32::from_rgb(248, 248, 242),
                text_secondary: Color32::from_rgb(180, 180, 170),
                grid_color: Color32::from_rgba_unmultiplied(100, 100, 80, 80),
                axis_color: Color32::from_rgb(140, 140, 120),
                error_color: Color32::from_rgb(255, 90, 90),
                error_bg: Color32::from_rgb(100, 50, 50),
                status_info: Color32::from_rgb(120, 220, 120),
                status_error: Color32::from_rgb(255, 100, 100),
                selection_fill: Color32::from_rgb(60, 60, 50),
                selection_stroke: Color32::from_rgb(255, 180, 100),
                intersection_dot: Color32::from_rgb(255, 220, 150),
                intersection_label_bg: Color32::from_rgb(60, 60, 40),
                vertex_color: Color32::from_rgb(255, 200, 50),
                root_color: Color32::from_rgb(100, 200, 100),
                fill_alpha: 35,
                egui_bg: Color32::from_rgb(35, 36, 30),
                egui_text: Color32::from_rgb(248, 248, 242),
                egui_button: Color32::from_rgb(55, 56, 50),
                egui_button_hover: Color32::from_rgb(65, 66, 60),
                egui_selected: Color32::from_rgb(70, 70, 55),
                egui_accent: Color32::from_rgb(255, 180, 100),
            },
            ThemeKind::SolarizedLight => Theme {
                name: "SolarizedLight",
                // Тёплый кремовый фон Solarized
                canvas_bg: Color32::from_rgb(253, 246, 227),
                frame_fill: Color32::from_rgb(242, 235, 217),
                // Тёмно-синий текст (не чёрный — меньше утомляет)
                text_primary: Color32::from_rgb(7, 54, 66),
                text_secondary: Color32::from_rgb(88, 110, 117),
                // Приглушённый серо-голубой для сетки
                grid_color: Color32::from_rgba_unmultiplied(147, 161, 161, 100),
                axis_color: Color32::from_rgb(110, 131, 150),
                error_color: Color32::from_rgb(198, 68, 52),
                error_bg: Color32::from_rgb(255, 220, 215),
                status_info: Color32::from_rgb(0, 147, 100),
                status_error: Color32::from_rgb(198, 68, 52),
                selection_fill: Color32::from_rgb(220, 230, 235),
                selection_stroke: Color32::from_rgb(59, 131, 181),
                intersection_dot: Color32::from_rgb(7, 54, 66),
                intersection_label_bg: Color32::from_rgb(255, 255, 230),
                vertex_color: Color32::from_rgb(198, 140, 50),
                root_color: Color32::from_rgb(0, 147, 100),
                fill_alpha: 30,
                egui_bg: Color32::from_rgb(253, 246, 227),
                egui_text: Color32::from_rgb(7, 54, 66),
                egui_button: Color32::from_rgb(232, 225, 207),
                egui_button_hover: Color32::from_rgb(220, 213, 195),
                egui_selected: Color32::from_rgb(200, 220, 230),
                egui_accent: Color32::from_rgb(38, 139, 210),
            },
            ThemeKind::GitHubLight => Theme {
                name: "GitHubLight",
                // Чистый белый фон GitHub
                canvas_bg: Color32::from_rgb(255, 255, 255),
                frame_fill: Color32::from_rgb(246, 248, 250),
                // Почти чёрный текст для максимальной контрастности
                text_primary: Color32::from_rgb(31, 35, 40),
                text_secondary: Color32::from_rgb(84, 95, 109),
                // Светло-серая сетка
                grid_color: Color32::from_rgba_unmultiplied(208, 215, 222, 120),
                axis_color: Color32::from_rgb(140, 152, 165),
                error_color: Color32::from_rgb(218, 54, 51),
                error_bg: Color32::from_rgb(255, 235, 235),
                status_info: Color32::from_rgb(22, 142, 78),
                status_error: Color32::from_rgb(218, 54, 51),
                selection_fill: Color32::from_rgb(224, 232, 255),
                selection_stroke: Color32::from_rgb(9, 105, 218),
                intersection_dot: Color32::from_rgb(31, 35, 40),
                intersection_label_bg: Color32::from_rgb(255, 255, 255),
                vertex_color: Color32::from_rgb(180, 140, 0),
                root_color: Color32::from_rgb(22, 142, 78),
                fill_alpha: 25,
                egui_bg: Color32::from_rgb(255, 255, 255),
                egui_text: Color32::from_rgb(31, 35, 40),
                egui_button: Color32::from_rgb(224, 228, 232),
                egui_button_hover: Color32::from_rgb(210, 215, 220),
                egui_selected: Color32::from_rgb(224, 232, 255),
                egui_accent: Color32::from_rgb(9, 105, 218),
            },
        }
    }

    /// Разрешить тему с учётом системной настройки (для ThemeKind::System).
    pub fn resolve(&self, ctx: &egui::Context) -> Theme {
        match self {
            ThemeKind::System => {
                let preferred = ctx.options(|opts| opts.theme_preference);
                match preferred {
                    egui::ThemePreference::Light => ThemeKind::Light.default_theme(),
                    _ => ThemeKind::Dark.default_theme(),
                }
            }
            _ => self.default_theme(),
        }
    }
}
    

                