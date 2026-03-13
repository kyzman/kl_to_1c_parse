/// Типы секций в файле обмена
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SectionType {
    Header,           // Заголовок файла (1CClientBankExchange)
    AccountStatement, // СекцияРасчСчет
    Document(String), // СекцияДокумент=<вид_документа>
}

/// Служебная информация из заголовка файла
#[derive(Debug, Clone, Default)]
pub struct FileHeader {
    pub version: Option<String>,      // ВерсияФормата
    pub encoding: Option<String>,     // Кодировка
    pub sender: Option<String>,       // Отправитель
    pub receiver: Option<String>,     // Получатель
    pub created_date: Option<String>, // ДатаСоздания
    pub created_time: Option<String>, // ВремяСоздания
    pub date_from: Option<String>,    // ДатаНачала
    pub date_to: Option<String>,      // ДатаКонца
    pub accounts: Vec<String>,        // РасчСчет (может быть несколько)
    pub document_types: Vec<String>,  // Документ (фильтр типов)
    /// Сырое содержимое заголовка для отладки/расширения
    pub raw_content: String,
}

/// Статистика по секциям файла
#[derive(Debug, Clone, Default)]
pub struct ParseStats {
    pub total_sections: u64,
    pub account_sections: u64,
    pub document_sections: u64,
    /// Подсчёт документов по видам: "Платежное поручение" -> 42
    pub documents_by_type: std::collections::HashMap<String, u64>,
}

impl ParseStats {
    pub fn add_document(&mut self, doc_type: &str) {
        self.document_sections += 1;
        *self
            .documents_by_type
            .entry(doc_type.to_string())
            .or_insert(0) += 1;
    }
}
