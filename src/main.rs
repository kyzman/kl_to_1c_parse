use load1c::config::{CliArgs, ConfigLoader};
use load1c::parser::stream::StreamParser;
use load1c::stats::print_results; // ⭐ Импортируем функцию вывода
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Парсим CLI аргументы
    let args = CliArgs::parse_args();

    // Загружаем конфигурацию
    let config_paths = if args.config_files.is_empty() {
        vec!["config/default.toml"]
    } else {
        args.config_files.iter().map(|s| s.as_str()).collect()
    };

    let base_path = PathBuf::from(".");
    let loader = ConfigLoader::new(base_path);
    let mut config = loader.load(&config_paths)?;

    // Применяем CLI аргументы поверх конфига
    config.apply_cli_args(&args);

    // Открываем файл
    let file = File::open(&args.file)?;
    let file_size = std::fs::metadata(&args.file)?.len();

    println!("🔍 Обработка файла: {}", args.file);
    println!(
        "📦 Размер файла: {:.2} MB",
        file_size as f64 / (1024.0 * 1024.0)
    );
    println!();

    // ⭐ Создаём парсер с конфигурацией и размером файла
    #[cfg(feature = "progress-bar")]
    let parser =
        StreamParser::with_config_and_size(BufReader::new(file), &config.parser, file_size);

    #[cfg(not(feature = "progress-bar"))]
    let parser = StreamParser::with_config(BufReader::new(file), &config.parser);

    // Запускаем парсинг
    let start = Instant::now();
    let (header, stats) = parser.parse()?;
    let elapsed = start.elapsed();

    // ⭐ ВЫВОДИМ статистику через stats.rs
    print_results(&header, &stats);

    println!("⏱️  Время обработки: {:.3} сек", elapsed.as_secs_f64());
    println!(
        "🚀 Скорость: {:.2} MB/сек",
        (file_size as f64 / (1024.0 * 1024.0)) / elapsed.as_secs_f64()
    );

    Ok(())
}
