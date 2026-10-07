use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Константа версии формата конфигурации.
pub const CONFIG_VERSION: u32 = 4;

/// Дефолтные значения конфигурации.
pub const DEFAULT_WINDOW_WIDTH: f64 = 1000.0;
pub const DEFAULT_WINDOW_HEIGHT: f64 = 700.0;
pub const DEFAULT_AUTO_LOAD: bool = false;
pub const DEFAULT_FONT_FAMILY: &str = "proportional";
pub const DEFAULT_FONT_SIZE: f32 = 12.0;

/// Настройки приложения.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    pub version: u32,
    pub window_width: f64,
    pub window_height: f64,
    pub auto_load_last_session: bool,
    pub last_session_path: Option<String>,
    pub theme: String,
    // Настройки шрифта
    pub font_family: String,     // "monospace" | "proportional" | "custom"
    pub font_size: f32,          // 12.0 по умолчанию
    pub custom_font_path: Option<String>, // путь к .ttf файлу
    // Настройки отображения
    pub school_grid: bool,
    pub polar_mode: bool,
    pub adaptive_density: bool,
    pub n_points: usize,
    pub auto_y: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            window_width: DEFAULT_WINDOW_WIDTH,
            window_height: DEFAULT_WINDOW_HEIGHT,
            auto_load_last_session: DEFAULT_AUTO_LOAD,
            last_session_path: None,
            theme: "dark".to_string(),
            font_family: DEFAULT_FONT_FAMILY.to_string(),
            font_size: DEFAULT_FONT_SIZE,
            custom_font_path: None,
            school_grid: false,
            polar_mode: false,
            adaptive_density: false,
            n_points: 500,
            auto_y: true,
        }
    }
}

impl AppConfig {
    /// Путь к пользовательской data directory.
    pub fn data_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("function-plotter")
    }

    /// Путь к файлу конфигурации.
    pub fn config_path() -> PathBuf {
        Self::data_dir().join("config.json")
    }

    /// Загрузить конфигурацию из файла.
    /// Если файл не существует или повреждён — возвращает конфигурацию по умолчанию.
    pub fn load() -> Self {
        let path = Self::config_path();
        if !path.exists() {
            return Self::default();
        }

        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<Self>(&content) {
                Ok(mut config) => {
                    // Forward compatibility: если версия старая, обновляем
                    if config.version < CONFIG_VERSION {
                        config.version = CONFIG_VERSION;
                        // Миграция v2 -> v3: добавляем theme по умолчанию
                        if config.theme.is_empty() {
                            config.theme = "dark".to_string();
                        }
                        // Миграция v3 -> v4: добавляем новые поля по умолчанию
                        if config.font_family.is_empty() {
                            config.font_family = DEFAULT_FONT_FAMILY.to_string();
                        }
                        if config.font_size <= 0.0 {
                            config.font_size = DEFAULT_FONT_SIZE;
                        }
                        // Сохраняем обновлённую версию
                        let _ = config.save();
                    }
                    config
                }
                Err(e) => {
                    eprintln!(
                        "Ошибка десериализации конфигурации: {e}. Используются значения по умолчанию."
                    );
                    Self::default()
                }
            },
            Err(e) => {
                eprintln!("Ошибка чтения конфигурации: {e}. Используются значения по умолчанию.");
                Self::default()
            }
        }
    }

    /// Сохранить конфигурацию в файл.
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();

        // Создаём директорию если не существует
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Создание директории: {e}"))?;
        }

        let json = serde_json::to_string_pretty(self).map_err(|e| format!("Сериализация: {e}"))?;
        fs::write(&path, json).map_err(|e| format!("Запись файла: {e}"))
    }

    /// Загрузить конфигурацию из указанного пути.
    #[allow(dead_code)]
    pub fn load_from(path: &PathBuf) -> Self {
        if !path.exists() {
            return Self::default();
        }

        match fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str::<Self>(&content) {
                Ok(mut config) => {
                    if config.version < CONFIG_VERSION {
                        config.version = CONFIG_VERSION;
                        if config.theme.is_empty() {
                            config.theme = "dark".to_string();
                        }
                        if config.font_family.is_empty() {
                            config.font_family = DEFAULT_FONT_FAMILY.to_string();
                        }
                        if config.font_size <= 0.0 {
                            config.font_size = DEFAULT_FONT_SIZE;
                        }
                    }
                    config
                }
                Err(_) => Self::default(),
            },
            Err(_) => Self::default(),
        }
    }

    /// Сохранить конфигурацию в указанный путь.
    #[allow(dead_code)]
    pub fn save_to(&self, path: &PathBuf) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Создание директории: {e}"))?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| format!("Сериализация: {e}"))?;
        fs::write(path, json).map_err(|e| format!("Запись файла: {e}"))
    }
}
