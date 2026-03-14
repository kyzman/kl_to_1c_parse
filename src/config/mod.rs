//! ⭐ Модуль конфигурации с поддержкой TOML, includes и CLI
//!
//! Поддерживает:
//! - Несколько файлов конфигурации
//! - Включение дочерних файлов через [includes]
//! - Переопределение через CLI аргументы
//! - Обязательные и опциональные настройки

mod cli;
mod loader;
mod models;

pub use cli::CliArgs;
pub use loader::ConfigLoader;
pub use models::*;

/// Путь к конфигурации по умолчанию
pub const DEFAULT_CONFIG_PATH: &str = "config/default.toml";
