use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::app::{GraphEntry, PlotApp};

/// Сериализуемое представление одного графика.
#[derive(Serialize, Deserialize, Clone)]
pub struct GraphEntryDto {
    pub label: String,
    pub formula: String,
    pub color: [u8; 3],
    pub visible: bool,
}

/// Сериализуемое представление всего состояния.
#[derive(Serialize, Deserialize)]
pub struct SessionData {
    pub version: u32,
    pub graphs: Vec<GraphEntryDto>,
    pub x_min: f64,
    pub x_max: f64,
    pub n_points: usize,
    pub auto_y: bool,
    pub adaptive: bool,
    pub adaptive_tolerance: f64,
    pub polar_mode: bool,
    pub viewport_x_min: f64,
    pub viewport_x_max: f64,
    pub viewport_y_min: f64,
    pub viewport_y_max: f64,
}

impl SessionData {
    pub const VERSION: u32 = 3;

    /// Собрать DTO из состояния приложения.
    pub fn from_app(app: &PlotApp) -> Self {
        let graphs = app
            .graphs
            .iter()
            .map(|g| GraphEntryDto {
                label: g.style.label.clone(),
                formula: g.formula_text.clone(),
                color: [g.style.color.r(), g.style.color.g(), g.style.color.b()],
                visible: g.style.visible,
            })
            .collect();

        SessionData {
            version: Self::VERSION,
            graphs,
            x_min: app.x_min,
            x_max: app.x_max,
            n_points: app.n_points,
            auto_y: app.auto_y,
            adaptive: app.adaptive,
            adaptive_tolerance: app.adaptive_tolerance,
            polar_mode: app.polar_mode,
            viewport_x_min: app.viewport.x_min,
            viewport_x_max: app.viewport.x_max,
            viewport_y_min: app.viewport.y_min,
            viewport_y_max: app.viewport.y_max,
        }
    }

    /// Применить сохранённое состояние к приложению.
    pub fn apply_to_app(self, app: &mut PlotApp) {
        app.graphs.clear();

        for dto in self.graphs {
            let color = egui::Color32::from_rgb(dto.color[0], dto.color[1], dto.color[2]);
            let mut entry = GraphEntry::new(&dto.label, color, &dto.formula);
            entry.style.visible = dto.visible;
            app.graphs.push(entry);
        }

        if app.graphs.is_empty() {
            app.graphs.push(GraphEntry::new(
                "f1",
                egui::Color32::from_rgb(100, 200, 255),
                "x",
            ));
        }

        app.x_min = self.x_min;
        app.x_max = self.x_max;
        app.n_points = self.n_points.max(10);
        app.auto_y = self.auto_y;
        app.adaptive = self.adaptive;
        app.adaptive_tolerance = self.adaptive_tolerance;
        app.polar_mode = self.polar_mode;
        app.viewport.x_min = self.viewport_x_min;
        app.viewport.x_max = self.viewport_x_max;
        if !self.auto_y {
            app.viewport.y_min = self.viewport_y_min;
            app.viewport.y_max = self.viewport_y_max;
        }
        app.recompute_all();
    }

    /// Сохранить сессию в JSON-файл.
    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| format!("Сериализация: {e}"))?;
        fs::write(path, json).map_err(|e| format!("Запись файла: {e}"))
    }

    /// Загрузить сессию из JSON-файла.
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path).map_err(|e| format!("Чтение файла: {e}"))?;
        let data: SessionData =
            serde_json::from_str(&content).map_err(|e| format!("Десериализация: {e}"))?;
        if data.version != Self::VERSION {
            return Err(format!(
                "Версия файла {} не поддерживается (ожидается {})",
                data.version,
                Self::VERSION
            ));
        }
        Ok(data)
    }
}
