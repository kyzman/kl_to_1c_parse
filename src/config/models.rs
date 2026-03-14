use crate::CliArgs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// ⭐ Корневая структура конфигурации
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    /// ⭐ Директива включения других файлов
    #[serde(default)]
    pub includes: IncludesConfig,

    /// ⭐ Настройки парсера (обязательные)
    pub parser: ParserConfig,

    /// ⭐ Настройки вывода (опциональные)
    #[serde(default)]
    pub output: OutputConfig,

    /// ⭐ Настройки логирования (опциональные)
    #[serde(default)]
    pub logging: LoggingConfig,
}

/// ⭐ Конфигурация includes
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct IncludesConfig {
    #[serde(default)]
    pub files: Vec<String>,
}

/// ⭐ Настройки парсера
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ParserConfig {
    /// Размер буфера для чтения файла (байты)
    pub buffer_size: usize,

    /// ⭐ Максимальная длина строки (байты)
    pub max_line_length: usize,

    /// Размер буфера для определения кодировки (байты)
    #[serde(default = "default_encoding_detection_size")]
    pub encoding_detection_size: usize,

    /// Количество строк для детектирования кодировки
    #[serde(default = "default_detection_lines")]
    pub detection_lines: usize,

    /// ⭐ Фичи парсера
    #[serde(default)]
    pub features: ParserFeatures,
}

fn default_encoding_detection_size() -> usize {
    1024
}
fn default_detection_lines() -> usize {
    20
}

/// ⭐ Фичи парсера
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ParserFeatures {
    /// Показывать прогресс-бар
    #[serde(default)]
    pub progress_bar: bool,

    /// Завершать программу при превышении длины строки
    #[serde(default = "default_true")]
    pub line_limit_error: bool,

    /// Валидировать контрольные суммы
    #[serde(default)]
    pub validate_checksums: bool,
}

fn default_true() -> bool {
    true
}

/// ⭐ Настройки вывода
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct OutputConfig {
    /// Формат вывода (table, json, csv)
    #[serde(default = "default_format")]
    pub format: String,

    /// Подробный вывод
    #[serde(default)]
    pub verbose: bool,

    /// Путь к файлу вывода (stdout если None)
    #[serde(default)]
    pub file: Option<PathBuf>,
}

fn default_format() -> String {
    "table".to_string()
}

/// ⭐ Настройки логирования
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct LoggingConfig {
    /// Уровень логирования (debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Путь к файлу лога
    #[serde(default)]
    pub file: Option<PathBuf>,
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Config {
    /// ⭐ Создаёт конфигурацию по умолчанию
    pub fn default_config() -> Self {
        Self {
            includes: IncludesConfig::default(),
            parser: ParserConfig {
                buffer_size: 64 * 1024,        // 64 KB
                max_line_length: 1024 * 1024,  // 1 MB
                encoding_detection_size: 1024, // 1 KB
                detection_lines: 20,
                features: ParserFeatures {
                    progress_bar: false,
                    line_limit_error: true,
                    validate_checksums: false,
                },
            },
            output: OutputConfig::default(),
            logging: LoggingConfig::default(),
        }
    }

    /// ⭐ Применяет CLI аргументы поверх конфигурации
    pub fn apply_cli_args(&mut self, args: &CliArgs) {
        if let Some(buffer_size) = args.buffer_size {
            self.parser.buffer_size = buffer_size;
        }

        if let Some(max_line_length) = args.max_line_length {
            self.parser.max_line_length = max_line_length;
        }

        if args.progress_bar {
            self.parser.features.progress_bar = true;
        }

        if args.line_limit_truncate {
            self.parser.features.line_limit_error = false;
        }

        if args.verbose {
            self.output.verbose = true;
        }

        if let Some(format) = &args.output_format {
            self.output.format = format.clone();
        }
    }
}

/// ⭐ Валидация конфигурации
impl Config {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.parser.buffer_size < 1024 {
            return Err(ConfigError::ValidationError(
                "buffer_size должен быть >= 1024".to_string(),
            ));
        }

        if self.parser.buffer_size > 100 * 1024 * 1024 {
            return Err(ConfigError::ValidationError(
                "buffer_size должен быть <= 100 MB".to_string(),
            ));
        }

        if self.parser.max_line_length < 1024 {
            return Err(ConfigError::ValidationError(
                "max_line_length должен быть >= 1024".to_string(),
            ));
        }

        Ok(())
    }
}

/// ⭐ Ошибки конфигурации
#[derive(Debug)]
pub enum ConfigError {
    IoError(String, std::io::Error),
    ParseError(String, toml::de::Error),
    DeserializationError(toml::de::Error),
    ValidationError(String),
    IncludeError(String),
    InvalidStructure(String), // ⭐ ДОБАВЛЕНО
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::IoError(path, e) => write!(f, "Ошибка чтения {}: {}", path, e),
            ConfigError::ParseError(path, e) => write!(f, "Ошибка парсинга {}: {}", path, e),
            ConfigError::DeserializationError(e) => write!(f, "Ошибка десериализации: {}", e),
            ConfigError::ValidationError(msg) => write!(f, "Ошибка валидации: {}", msg),
            ConfigError::IncludeError(msg) => write!(f, "Ошибка include: {}", msg),
            ConfigError::InvalidStructure(msg) => write!(f, "Ошибка структуры: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        ConfigError::IoError("unknown".to_string(), e)
    }
}
