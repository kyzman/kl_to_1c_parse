use crate::parser::models::{FileHeader, ParseStats};

pub fn print_results(header: &FileHeader, stats: &ParseStats) {
    println!("📄 Заголовок файла:");
    println!("   Версия формата: {:?}", header.version);
    println!("   Кодировка (из файла): {:?}", header.encoding);
    println!("   Кодировка (определена): {:?}", header.detected_encoding);
    println!("   Отправитель: {:?}", header.sender);
    println!("   Получатель: {:?}", header.receiver);
    println!(
        "   Дата создания: {:?} {:?}",
        header.created_date, header.created_time
    );
    println!("   Период: {:?} — {:?}", header.date_from, header.date_to);
    println!("   Счета: {:?}", header.accounts);
    println!("   Фильтр документов: {:?}", header.document_types);
    println!();

    println!("📈 Статистика секций:");
    println!("   Всего секций: {}", stats.total_sections);
    println!("   Секций РасчСчет: {}", stats.account_sections);
    println!("   Секций Документ: {}", stats.document_sections);

    if !stats.documents_by_type.is_empty() {
        println!("\n📋 Документы по типам:");
        let mut types: Vec<_> = stats.documents_by_type.iter().collect();
        types.sort_by(|a, b| b.1.cmp(a.1));
        for (doc_type, count) in types {
            println!("   • {}: {}", doc_type, count);
        }
    }
    println!();
}
