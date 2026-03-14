use load1c::parser::stream::StreamParser;
use load1c::stats::print_results;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Использование: {} <путь_к_файлу>", args[0]);
        eprintln!();
        eprintln!("Опции компиляции:");
        eprintln!("  --features progress-bar  # Включить прогресс-бар");
        eprintln!("  --features line-limit-error   # Завершать при превышении длины строки");
        eprintln!("  --features line-limit-truncate # Обрезать строку при превышении");
        std::process::exit(1);
    }

    let file_path = &args[1];

    if !Path::new(file_path).exists() {
        eprintln!("❌ Ошибка: файл '{}' не найден", file_path);
        std::process::exit(1);
    }

    // Получаем размер файла для прогресс-бара
    let file_size = std::fs::metadata(file_path)?.len();

    println!("🔍 Обработка файла: {}", file_path);
    println!(
        "📦 Размер файла: {:.2} MB",
        file_size as f64 / (1024.0 * 1024.0)
    );
    println!();

    let file = File::open(file_path)?;

    #[cfg(feature = "progress-bar")]
    let parser = StreamParser::with_file_size(BufReader::new(file), file_size);

    #[cfg(not(feature = "progress-bar"))]
    let parser = StreamParser::new(BufReader::new(file));

    let start = Instant::now();
    let (header, stats) = parser.parse()?;
    let elapsed = start.elapsed();

    print_results(&header, &stats);

    println!("⏱️  Время обработки: {:.3} сек", elapsed.as_secs_f64());
    println!(
        "🚀 Скорость: {:.2} MB/сек",
        (file_size as f64 / (1024.0 * 1024.0)) / elapsed.as_secs_f64()
    );

    Ok(())
}
