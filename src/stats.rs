use crate::parser::models::{FileHeader, ParseStats};

pub fn print_results(header: &FileHeader, stats: &ParseStats) {
    println!("\n📊 ════════════════════════════════════════════════════");
    println!("📄 ЗАГОЛОВОК ФАЙЛА");
    println!("════════════════════════════════════════════════════");
    println!("   Версия формата:     {:?}", header.version);
    println!("   Кодировка (файл):   {:?}", header.encoding);
    println!("   Кодировка (определена): {:?}", header.detected_encoding);
    println!("   Отправитель:        {:?}", header.sender);
    println!("   Получатель:         {:?}", header.receiver);
    println!(
        "   Дата создания:      {:?} {:?}",
        header.created_date, header.created_time
    );
    println!(
        "   Период:             {:?} — {:?}",
        header.date_from, header.date_to
    );
    println!("   Счета:              {:?}", header.accounts);
    println!("   Фильтр документов:  {:?}", header.document_types);

    println!("\n📈 ════════════════════════════════════════════════════");
    println!("📈 СТАТИСТИКА ОБРАБОТКИ");
    println!("════════════════════════════════════════════════════");
    println!(
        "   Всего строк:        {}",
        format_number(stats.total_lines)
    );
    println!("   Всего байт:         {}", format_bytes(stats.total_bytes));
    println!(
        "   Всего секций:       {}",
        format_number(stats.total_sections)
    );
    println!(
        "   ├─ РасчСчет:        {}",
        format_number(stats.account_sections)
    );
    println!(
        "   └─ Документы:       {}",
        format_number(stats.document_sections)
    );

    if !stats.documents_by_type.is_empty() {
        println!("\n📋 ════════════════════════════════════════════════════");
        println!("📋 ДОКУМЕНТЫ ПО ТИПАМ");
        println!("════════════════════════════════════════════════════");
        let mut types: Vec<_> = stats.documents_by_type.iter().collect();
        types.sort_by(|a, b| b.1.cmp(a.1));
        for (doc_type, count) in types {
            println!("   • {:<35} {}", doc_type, format_number(*count));
        }
    }

    println!("\n✅ ════════════════════════════════════════════════════\n");
}

/// ⭐ Форматирование чисел с разделителями (1 000 000)
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

/// ⭐ Форматирование размера в байтах (1.5 MB, 70.2 MB, и т.д.)
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
