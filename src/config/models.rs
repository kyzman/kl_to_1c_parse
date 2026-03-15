use crate::CliArgs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// ⭐ Корневая структура конфигурации
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub includes: IncludesConfig,

    /// ⭐ НОВОЕ: Настройки входных файлов
    #[serde(default)]
    pub input: InputConfig,

    pub parser: ParserConfig,

    #[serde(default)]
    pub output: OutputConfig,

    #[serde(default)]
    pub logging: LoggingConfig,
}

/// ⭐ Конфигурация includes
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct IncludesConfig {
    #[serde(default)]
    pub files: Vec<String>,
}

/// ⭐ НОВОЕ: Настройки входных файлов
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct InputConfig {
    /// Одиночный файл для обработки
    #[serde(default)]
    pub file: Option<String>,

    /// ⭐ НОВОЕ: Список конкретных файлов для обработки
    #[serde(default)]
    pub files: Vec<String>,

    /// Каталог для рекурсивного поиска файлов
    #[serde(default)]
    pub directory: Option<String>,

    /// Glob-паттерн для поиска файлов (например, "data/*.1c")
    #[serde(default)]
    pub glob: Option<String>,

    /// Расширения файлов для поиска в каталоге (по умолчанию: .txt, .1c)
    #[serde(default = "default_extensions")]
    pub extensions: Vec<String>,

    /// Рекурсивный поиск в подкаталогах
    #[serde(default = "default_true")]
    pub recursive: bool,
}

fn default_extensions() -> Vec<String> {
    vec![".txt".to_string(), ".1c".to_string()]
}

fn default_true() -> bool {
    true
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

/// ⭐ Настройки вывода
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct OutputConfig {
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default)]
    pub verbose: bool,
    #[serde(default)]
    pub file: Option<PathBuf>,

    /// ⭐ НОВОЕ: Режим вывода статистики
    #[serde(default)]
    pub stats_mode: StatsMode,
}

/// ⭐ Режим вывода статистики
#[derive(Debug, Deserialize, Serialize, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StatsMode {
    /// Только общая статистика (по умолчанию)
    #[default]
    Aggregated,
    /// Только по каждому файлу
    PerFile,
    /// И общая, и по каждому файлу
    Both,
}

impl StatsMode {
    /// Проверяет, нужно ли выводить статистику по файлу
    pub fn show_per_file(&self) -> bool {
        matches!(self, StatsMode::PerFile | StatsMode::Both)
    }

    /// Проверяет, нужно ли выводить общую статистику
    pub fn show_aggregated(&self) -> bool {
        matches!(self, StatsMode::Aggregated | StatsMode::Both)
    }
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
            input: InputConfig::default(),
            parser: ParserConfig {
                buffer_size: 64 * 1024,
                max_line_length: 1024 * 1024,
                encoding_detection_size: 1024,
                detection_lines: 20,
                features: ParserFeatures::default(),
            },
            output: OutputConfig::default(),
            logging: LoggingConfig::default(),
        }
    }

    /// ⭐ Применяет CLI аргументы поверх конфигурации
    pub fn apply_cli_args(&mut self, args: &CliArgs) {
        // ⭐ ИСПРАВЛЕНО: Приоритет источников файлов (CLI > config.files > config.file)
        if !args.files.is_empty() {
            // CLI файлы переопределяют всё
            self.input.files = args.files.clone();
            self.input.file = None;
            self.input.directory = None;
            self.input.glob = None;
        }

        if args.directory.is_some() {
            self.input.directory = args.directory.clone();
            self.input.file = None;
            self.input.glob = None;
        }

        if args.glob.is_some() {
            self.input.glob = args.glob.clone();
            self.input.file = None;
            self.input.directory = None;
        }

        if !args.extensions.is_empty() {
            self.input.extensions = args.extensions.clone();
        }

        if args.recursive.is_some() {
            self.input.recursive = args.recursive.unwrap();
        }

        // ⭐ НОВОЕ: Применяем режим статистики
        if args.stats_mode.is_some() == false || args.per_file_stats {
            self.output.stats_mode = args.get_stats_mode();
        }

        // Остальные аргументы...
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
        // ⭐ ИСПРАВЛЕНО: Проверяем после применения CLI или наличие в конфиге
        // Валидация вызывается ПОСЛЕ apply_cli_args() в main.rs

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

    /// ⭐ Проверяет, что указан хотя бы один источник файлов
    pub fn validate_file_source(&self) -> Result<(), ConfigError> {
        if self.input.file.is_none()
            && self.input.files.is_empty()  // ⭐ ПРОВЕРЯЕМ список файлов
            && self.input.directory.is_none()
            && self.input.glob.is_none()
        {
            return Err(ConfigError::ValidationError(
                "Не указан источник файлов: file, files, directory или glob".to_string(),
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
