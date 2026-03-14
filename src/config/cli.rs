use clap::Parser;

/// ⭐ CLI аргументы командной строки
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    /// Путь к файлу для обработки
    #[arg(required = true)]
    pub file: String,

    /// Пути к файлам конфигурации (переопределяют друг друга по порядку)
    #[arg(short = 'c', long = "config", value_name = "FILE")]
    pub config_files: Vec<String>,

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
    /// ⭐ Парсит аргументы из командной строки
    pub fn parse_args() -> Self {
        Self::parse()
    }
}
