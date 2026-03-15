use crate::config::StatsMode;
use clap::Parser;

/// ⭐ CLI аргументы командной строки
#[derive(Parser, Debug, Clone)]
#[command(author, version, about = "Парсер файлов 1CClientBankExchange", long_about = None)]
pub struct CliArgs {
    /// Файлы для обработки (можно указать несколько, переопределяет config.files)
    #[arg(short = 'f', long = "file", value_name = "PATH", num_args = 1..)]
    pub files: Vec<String>, // ⭐ ИСПРАВЛЕНО: Vec вместо Option

    /// Каталог для рекурсивного поиска файлов
    #[arg(short = 'd', long = "directory", value_name = "DIR")]
    pub directory: Option<String>,

    /// Glob-паттерн для поиска файлов (например, "data/*.1c")
    #[arg(short = 'g', long = "glob", value_name = "PATTERN")]
    pub glob: Option<String>,

    /// Расширения файлов для поиска (по умолчанию: .txt, .1c)
    #[arg(long = "ext", value_name = "EXT", num_args = 1.., value_delimiter = ',')]
    pub extensions: Vec<String>,

    /// Рекурсивный поиск в подкаталогах
    #[arg(long = "recursive", action = clap::ArgAction::Set)]
    pub recursive: Option<bool>,

    /// Пути к файлам конфигурации
    #[arg(short = 'c', long = "config", value_name = "FILE")]
    pub config_files: Vec<String>,

    /// Режим вывода статистики (aggregated, per-file, both)
    #[arg(long = "stats-mode", value_name = "MODE", default_value = "aggregated")]
    pub stats_mode: Option<String>,

    /// Выводить статистику по каждому файлу (краткая форма)
    #[arg(long = "per-file-stats", conflicts_with = "stats_mode")]
    pub per_file_stats: bool,

    /// Размер буфера чтения (байты)
    #[arg(long = "buffer-size", value_name = "BYTES")]
    pub buffer_size: Option<usize>,

    /// Максимальная длина строки (байты)
    #[arg(long = "max-line-length", value_name = "BYTES")]
    pub max_line_length: Option<usize>,

    /// Показывать прогресс-бар
    #[arg(long = "progress")]
    pub progress_bar: bool,

    /// Обрезать строки вместо ошибки при превышении длины
    #[arg(long = "line-limit-truncate")]
    pub line_limit_truncate: bool,

    /// Подробный вывод
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Формат вывода (table, json, csv)
    #[arg(long = "output-format", value_name = "FORMAT")]
    pub output_format: Option<String>,
}

impl CliArgs {
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// ⭐ Преобразует CLI аргументы в StatsMode
    pub fn get_stats_mode(&self) -> StatsMode {
        if self.per_file_stats {
            StatsMode::PerFile
        } else if let Some(mode) = &self.stats_mode {
            // ⭐ ИСПРАВЛЕНО: проверяем Option
            match mode.as_str() {
                "per-file" | "per_file" => StatsMode::PerFile,
                "both" => StatsMode::Both,
                "aggregated" => StatsMode::Aggregated,
                _ => StatsMode::Aggregated,
            }
        } else {
            StatsMode::Aggregated // ⭐ По умолчанию
        }
    }

    /// ⭐ Проверяет, указаны ли источники файлов через CLI
    pub fn has_file_source(&self) -> bool {
        !self.files.is_empty() || self.directory.is_some() || self.glob.is_some()
    }
}
