use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Константа версии формата конфигурации.
pub const CONFIG_VERSION: u32 = 2;

/// Дефолтные значения конфигурации.
pub const DEFAULT_WINDOW_WIDTH: f64 = 1000.0;
pub const DEFAULT_WINDOW_HEIGHT: f64 = 700.0;
pub const DEFAULT_AUTO_LOAD: bool = false;

/// Настройки приложения.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    pub version: u32,
    pub window_width: f64,
    pub window_height: f64,
    pub auto_load_last_session: bool,
    pub last_session_path: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            window_width: DEFAULT_WINDOW_WIDTH,
            window_height: DEFAULT_WINDOW_HEIGHT,
            auto_load_last_session: DEFAULT_AUTO_LOAD,
            last_session_path: None,
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
                        // Сохраняем обновлённую версию
                        let _ = config.save();
                    }
                    config
                }
                Err(e) => {
                    eprintln!("Ошибка десериализации конфигурации: {e}. Используются значения по умолчанию.");
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
}
