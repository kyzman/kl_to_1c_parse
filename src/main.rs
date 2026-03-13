// src/main.rs
use load1c::parser::stream::StreamParser;
use load1c::stats::print_results;

use std::env;
use std::fs::File;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Использование: {} <путь_к_файлу>", args[0]);
        std::process::exit(1);
    }

    let file_path = &args[1];
    println!("🔍 Парсинг файла: {}", file_path);

    let file = File::open(file_path)?;
    let parser = StreamParser::new(file);

    let start = std::time::Instant::now();
    let (header, stats) = parser.parse()?;
    let elapsed = start.elapsed();

    print_results(&header, &stats);
    println!("⏱️  Время обработки: {:.3} сек", elapsed.as_secs_f64());

    Ok(())
}
