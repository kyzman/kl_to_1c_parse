use load1c::config::{CliArgs, ConfigLoader};
use load1c::file_discovery::discover_files;
use load1c::parser::stream::StreamParser;
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

    let loader = ConfigLoader::new(PathBuf::from("."));
    let mut config = loader.load(&config_paths)?;

    // Применяем CLI аргументы поверх конфига
    config.apply_cli_args(&args);

    // ⭐ Обнаруживаем файлы для обработки
    let discovery =
        discover_files(&config.input).map_err(|e| format!("Ошибка обнаружения файлов: {}", e))?;

    if discovery.files.is_empty() {
        eprintln!("❌ Не найдено файлов для обработки");
        if !discovery.errors.is_empty() {
            eprintln!("Ошибки:");
            for (path, error) in &discovery.errors {
                eprintln!("  • {:?}: {}", path, error);
            }
        }
        return Ok(());
    }

    println!("🔍 {}", discovery.summary());
    if config.output.verbose {
        for file in &discovery.files {
            println!("   📄 {:?}", file);
        }
    }
    println!();

    // ⭐ Обрабатываем каждый файл
    let mut total_stats = load1c::parser::models::ParseStats::default();
    let global_start = Instant::now();
    let mut file_errors = Vec::new();

    for (index, file_path) in discovery.files.iter().enumerate() {
        println!(
            "📦 [{}/{}] Обработка: {:?}",
            index + 1,
            discovery.files.len(),
            file_path
        );

        let file = match File::open(file_path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("   ❌ Ошибка открытия файла: {}", e);
                file_errors.push((file_path.clone(), e.to_string()));
                continue;
            }
        };

        let file_size = std::fs::metadata(file_path)?.len();

        // Создаём парсер
        #[cfg(feature = "progress-bar")]
        let parser = if config.parser.features.progress_bar && file_size > 0 {
            StreamParser::with_config_and_size(BufReader::new(file), &config.parser, file_size)
        } else {
            StreamParser::with_config(BufReader::new(file), &config.parser)
        };

        #[cfg(not(feature = "progress-bar"))]
        let parser = StreamParser::with_config(BufReader::new(file), &config.parser);

        // Парсим
        let start = Instant::now();
        match parser.parse() {
            Ok((header, stats)) => {
                let elapsed = start.elapsed();

                // ⭐ Вывод статистики по файлу (если включено)
                if config.output.stats_mode.show_per_file() {
                    println!("\n   📊 Статистика файла:");
                    println!("   ────────────────────────────────");
                    println!("   Строк:  {}", format_number(stats.total_lines));
                    println!("   Байт:   {}", format_bytes(stats.total_bytes));
                    println!("   Секций: {}", format_number(stats.total_sections));
                    if stats.document_sections > 0 {
                        println!("   Документы: {}", format_number(stats.document_sections));
                        for (doc_type, count) in &stats.documents_by_type {
                            println!("     • {}: {}", doc_type, count);
                        }
                    }
                    println!(
                        "   ⏱️  {:.3} сек | 🚀 {:.2} MB/сек",
                        elapsed.as_secs_f64(),
                        (file_size as f64 / (1024.0 * 1024.0)) / elapsed.as_secs_f64()
                    );
                    println!();
                } else {
                    // Краткий вывод если не show_per_file
                    println!(
                        "   ✅ {:.3} сек | {} строк | {} байт",
                        elapsed.as_secs_f64(),
                        format_number(stats.total_lines),
                        format_bytes(stats.total_bytes)
                    );
                    println!();
                }

                // Суммируем статистику
                total_stats.total_sections += stats.total_sections;
                total_stats.account_sections += stats.account_sections;
                total_stats.document_sections += stats.document_sections;
                total_stats.total_lines += stats.total_lines;
                total_stats.total_bytes += stats.total_bytes;

                for (doc_type, count) in stats.documents_by_type {
                    *total_stats.documents_by_type.entry(doc_type).or_insert(0) += count;
                }
            }
            Err(e) => {
                eprintln!("   ❌ Ошибка парсинга: {}", e);
                file_errors.push((file_path.clone(), e.to_string()));
            }
        }
    }

    // ⭐ Итоговая статистика по всем файлам (если включено)
    if discovery.files.len() > 1 && config.output.stats_mode.show_aggregated() {
        print_aggregated_stats(&total_stats, discovery.files.len(), file_errors.len());
    } else if discovery.files.len() == 1 && config.output.stats_mode.show_aggregated() {
        // Для одного файла всегда показываем полную статистику
        print_aggregated_stats(&total_stats, 1, file_errors.len());
    }

    let global_elapsed = global_start.elapsed();
    println!("\n⏱️  Общее время: {:.3} сек", global_elapsed.as_secs_f64());

    // Ошибки
    if !file_errors.is_empty() {
        eprintln!("\n❌ Ошибки обработки ({} файлов):", file_errors.len());
        for (path, error) in &file_errors {
            eprintln!("   • {:?}: {}", path, error);
        }
    }

    Ok(())
}

/// ⭐ Вывод итоговой статистики
fn print_aggregated_stats(
    stats: &load1c::parser::models::ParseStats,
    file_count: usize,
    error_count: usize,
) {
    println!("\n📊 ════════════════════════════════════════════════════");
    println!("📊 ИТОГОВАЯ СТАТИСТИКА ({} файлов)", file_count);
    println!("════════════════════════════════════════════════════");
    println!("   Всего строк:  {}", format_number(stats.total_lines));
    println!("   Всего байт:   {}", format_bytes(stats.total_bytes));
    println!("   Всего секций: {}", format_number(stats.total_sections));
    println!("   ├─ РасчСчет:  {}", format_number(stats.account_sections));
    println!(
        "   └─ Документы: {}",
        format_number(stats.document_sections)
    );

    if !stats.documents_by_type.is_empty() {
        println!("\n📋 Документы по типам:");
        let mut types: Vec<_> = stats.documents_by_type.iter().collect();
        types.sort_by(|a, b| b.1.cmp(a.1));
        for (doc_type, count) in types {
            println!("   • {:<40} {}", doc_type, format_number(*count));
        }
    }

    if error_count > 0 {
        println!("\n⚠️  Ошибки: {} файлов не обработано", error_count);
    }

    println!("════════════════════════════════════════════════════");
}

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(' ');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
