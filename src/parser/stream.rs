use crate::parser::{models::*, state::*};
use std::io::{BufRead, BufReader, Read};

/// Потоковый парсер файлов 1CClientBankExchange
pub struct StreamParser<R: Read> {
    reader: BufReader<R>,
    state: ParserState,
    header: FileHeader,
    stats: ParseStats,
    header_buffer: String,
    header_finalized: bool,
}

impl<R: Read> StreamParser<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::with_capacity(64 * 1024, reader),
            state: ParserState::WaitingHeader,
            header: FileHeader::default(),
            stats: ParseStats::default(),
            header_buffer: String::new(),
            header_finalized: false,
        }
    }

    pub fn parse(mut self) -> Result<(FileHeader, ParseStats), Box<dyn std::error::Error>> {
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = self.reader.read_line(&mut line)?;

            if bytes_read == 0 {
                self.state = ParserState::EndOfFile;
                break;
            }

            let line = line.trim_end_matches(&['\r', '\n'][..]);

            if line.is_empty() {
                continue;
            }

            self.process_line(line)?;

            if self.state == ParserState::EndOfFile {
                break;
            }
        }

        self.finalize_header();

        Ok((self.header, self.stats))
    }

    fn process_line(&mut self, line: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(keyword) = Self::extract_keyword(line) {
            let old_state = self.state.clone();
            let (new_state, section_type) = self.state.transition(&keyword);

            if matches!(old_state, ParserState::ReadingHeader)
                && !matches!(new_state, ParserState::ReadingHeader)
                && !self.header_finalized
            {
                self.finalize_header();
            }

            if let Some(section) = &section_type {
                self.stats.total_sections += 1;
                match section {
                    SectionType::AccountStatement => {
                        self.stats.account_sections += 1;
                    }
                    SectionType::Document(doc_type) => {
                        self.stats.add_document(doc_type);
                    }
                    SectionType::Header => {}
                }
            }

            self.state = new_state;
        } else {
            match self.state {
                ParserState::ReadingHeader => {
                    self.header_buffer.push_str(line);
                    self.header_buffer.push('\n');
                    Self::parse_header_field_line(&mut self.header, line);
                }
                ParserState::WaitingHeader => {}
                _ => {}
            }
        }

        Ok(())
    }

    fn finalize_header(&mut self) {
        if self.header_finalized {
            return;
        }

        self.header.raw_content = std::mem::take(&mut self.header_buffer);
        self.header_finalized = true;

        if self.header.version.is_none() && !self.header.raw_content.is_empty() {
            eprintln!("⚠️  Предупреждение: отсутствует поле 'ВерсияФормата'");
        }
    }

    /// Извлекает ключевое слово из строки
    fn extract_keyword(line: &str) -> Option<String> {
        if line.starts_with("1CClientBankExchange") {
            return Some("1CClientBankExchange".to_string());
        }
        if line.starts_with("СекцияРасчСчет") {
            return Some("СекцияРасчСчет".to_string());
        }
        if line.starts_with("КонецРасчСчет") {
            return Some("КонецРасчСчет".to_string());
        }
        if line.starts_with("СекцияДокумент") {
            return Some(line.to_string());
        }
        if line.starts_with("КонецДокумента") {
            return Some("КонецДокумента".to_string());
        }
        if line.starts_with("КонецФайла") {
            return Some("КонецФайла".to_string());
        }
        None
    }

    /// Парсит одно поле заголовка вида "Ключ=Значение"
    fn parse_header_field_line(header: &mut FileHeader, line: &str) {
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            match key {
                "ВерсияФормата" => header.version = Some(value.to_string()),
                "Кодировка" => header.encoding = Some(value.to_string()),
                "Отправитель" => header.sender = Some(value.to_string()),
                "Получатель" => header.receiver = Some(value.to_string()),
                "ДатаСоздания" => header.created_date = Some(value.to_string()),
                "ВремяСоздания" => header.created_time = Some(value.to_string()),
                "ДатаНачала" => header.date_from = Some(value.to_string()),
                "ДатаКонца" => header.date_to = Some(value.to_string()),
                "РасчСчет" => header.accounts.push(value.to_string()),
                "Документ" => header.document_types.push(value.to_string()),
                _ => {}
            }
        }
    }

    pub fn into_results(self) -> (FileHeader, ParseStats) {
        (self.header, self.stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_parse_minimal_file() {
        let sample = "1CClientBankExchange\nВерсияФормата=1.03\nКонецФайла\n";
        let cursor = Cursor::new(sample.as_bytes());
        let (header, stats) = StreamParser::new(cursor).parse().unwrap();

        assert_eq!(header.version, Some("1.03".to_string()));
        assert!(header.raw_content.contains("ВерсияФормата=1.03"));
        assert_eq!(stats.total_sections, 1);
    }

    #[test]
    fn test_parse_header_fields() {
        let sample = concat!(
            "1CClientBankExchange\n",
            "ВерсияФормата=1.03\n",
            "Кодировка=Windows\n",
            "РасчСчет=40702810123456789012\n",
            "РасчСчет=40702810987654321098\n",
            "КонецФайла\n"
        );
        let cursor = Cursor::new(sample.as_bytes());
        let (header, _) = StreamParser::new(cursor).parse().unwrap();

        assert_eq!(header.version, Some("1.03".to_string()));
        assert_eq!(header.encoding, Some("Windows".to_string()));
        assert_eq!(header.accounts.len(), 2);
    }

    #[test]
    fn test_parse_document_sections() {
        let sample = concat!(
            "1CClientBankExchange\n",
            "ВерсияФормата=1.03\n",
            "СекцияДокумент=Платежное поручение\n",
            "КонецДокумента\n",
            "СекцияДокумент=Инкассовое поручение\n",
            "КонецДокумента\n",
            "СекцияДокумент=Платежное поручение\n",
            "КонецДокумента\n",
            "КонецФайла\n"
        );
        let cursor = Cursor::new(sample.as_bytes());
        let (_, stats) = StreamParser::new(cursor).parse().unwrap();

        assert_eq!(stats.document_sections, 3);
        assert_eq!(stats.documents_by_type["Платежное поручение"], 2);
        assert_eq!(stats.documents_by_type["Инкассовое поручение"], 1);
    }

    #[test]
    fn test_parse_mixed_sections() {
        let sample = concat!(
            "1CClientBankExchange\n",
            "ВерсияФормата=1.03\n",
            "СекцияРасчСчет\n",
            "РасчСчет=40702810123456789012\n",
            "КонецРасчСчет\n",
            "СекцияДокумент=Платежное поручение\n",
            "КонецДокумента\n",
            "КонецФайла\n"
        );
        let cursor = Cursor::new(sample.as_bytes());
        let (_, stats) = StreamParser::new(cursor).parse().unwrap();

        assert_eq!(stats.account_sections, 1);
        assert_eq!(stats.document_sections, 1);
        assert_eq!(stats.total_sections, 3);
    }

    #[test]
    fn test_document_type_extraction() {
        let sample = concat!(
            "1CClientBankExchange\n",
            "СекцияДокумент=Платежное поручение\n",
            "КонецДокумента\n",
            "КонецФайла\n"
        );
        let cursor = Cursor::new(sample.as_bytes());
        let (_, stats) = StreamParser::new(cursor).parse().unwrap();

        assert_eq!(stats.document_sections, 1);
        assert!(stats.documents_by_type.contains_key("Платежное поручение"));
        assert_eq!(stats.documents_by_type["Платежное поручение"], 1);
    }

    #[test]
    fn test_ignore_unknown_keywords() {
        let sample = concat!(
            "1CClientBankExchange\n",
            "ВерсияФормата=1.03\n",
            "НеизвестноеКлючевоеСлово=Значение\n",
            "СекцияДокумент=Тест\n",
            "КонецДокумента\n",
            "КонецФайла\n"
        );
        let cursor = Cursor::new(sample.as_bytes());
        let (_, stats) = StreamParser::new(cursor).parse().unwrap();

        assert_eq!(stats.document_sections, 1);
        assert_eq!(stats.documents_by_type["Тест"], 1);
    }

    #[test]
    fn test_header_only() {
        let sample = concat!(
            "1CClientBankExchange\n",
            "ВерсияФормата=1.03\n",
            "Кодировка=Windows\n",
            "КонецФайла\n"
        );
        let cursor = Cursor::new(sample.as_bytes());
        let (_, stats) = StreamParser::new(cursor).parse().unwrap();

        assert_eq!(stats.total_sections, 1);
        assert_eq!(stats.account_sections, 0);
        assert_eq!(stats.document_sections, 0);
    }

    #[test]
    fn test_parse_without_end_file_marker() {
        let sample = concat!(
            "1CClientBankExchange\n",
            "ВерсияФормата=1.03\n",
            "СекцияДокумент=Тест\n",
            "КонецДокумента\n"
        );
        let cursor = Cursor::new(sample.as_bytes());
        let (_, stats) = StreamParser::new(cursor).parse().unwrap();

        assert_eq!(stats.document_sections, 1);
    }

    #[test]
    fn test_empty_file() {
        let sample = "";
        let cursor = Cursor::new(sample.as_bytes());
        let (_, stats) = StreamParser::new(cursor).parse().unwrap();

        assert_eq!(stats.total_sections, 0);
    }

    #[test]
    fn test_empty_lines_and_whitespace() {
        let sample = concat!(
            "1CClientBankExchange\n",
            "\n",
            "ВерсияФормата=1.03\n",
            "\n",
            "СекцияДокумент=Тест\n",
            "\n",
            "КонецДокумента\n",
            "\n",
            "КонецФайла\n"
        );
        let cursor = Cursor::new(sample.as_bytes());
        let (_, stats) = StreamParser::new(cursor).parse().unwrap();

        assert_eq!(stats.document_sections, 1);
    }

    #[test]
    fn test_multiple_account_numbers_in_header() {
        let sample = concat!(
            "1CClientBankExchange\n",
            "РасчСчет=40702810123456789012\n",
            "РасчСчет=40702810987654321098\n",
            "РасчСчет=40702810111223344556\n",
            "КонецФайла\n"
        );
        let cursor = Cursor::new(sample.as_bytes());
        let (header, _) = StreamParser::new(cursor).parse().unwrap();

        assert_eq!(header.accounts.len(), 3);
    }

    #[test]
    fn test_performance_large_file_simulation() {
        let mut data = String::from("1CClientBankExchange\nВерсияФормата=1.03\n");
        for i in 0..10_000 {
            data.push_str(&format!(
                "СекцияДокумент=Платежное поручение\nНомер={}\nСумма=100.00\nКонецДокумента\n",
                i
            ));
        }
        data.push_str("КонецФайла\n");

        let cursor = Cursor::new(data.into_bytes());
        let start = std::time::Instant::now();
        let (_, stats) = StreamParser::new(cursor).parse().unwrap();
        let elapsed = start.elapsed();

        assert_eq!(stats.document_sections, 10_000);
        assert_eq!(stats.documents_by_type["Платежное поручение"], 10_000);

        println!("⏱️  10 000 документов за {:.3} сек", elapsed.as_secs_f64());
        assert!(
            elapsed.as_secs_f64() < 1.0,
            "Парсинг должен занимать менее 1 секунды"
        );
    }

    #[test]
    fn test_extract_keyword_header() {
        assert_eq!(
            StreamParser::<Cursor<&[u8]>>::extract_keyword("1CClientBankExchange"),
            Some("1CClientBankExchange".to_string())
        );
    }

    #[test]
    fn test_extract_keyword_account_open() {
        assert_eq!(
            StreamParser::<Cursor<&[u8]>>::extract_keyword("СекцияРасчСчет"),
            Some("СекцияРасчСчет".to_string())
        );
    }

    #[test]
    fn test_extract_keyword_account_close() {
        assert_eq!(
            StreamParser::<Cursor<&[u8]>>::extract_keyword("КонецРасчСчет"),
            Some("КонецРасчСчет".to_string())
        );
    }

    #[test]
    fn test_extract_keyword_document_with_type() {
        assert_eq!(
            StreamParser::<Cursor<&[u8]>>::extract_keyword("СекцияДокумент=Платежное поручение"),
            Some("СекцияДокумент=Платежное поручение".to_string())
        );
    }

    #[test]
    fn test_extract_keyword_document_close() {
        assert_eq!(
            StreamParser::<Cursor<&[u8]>>::extract_keyword("КонецДокумента"),
            Some("КонецДокумента".to_string())
        );
    }

    #[test]
    fn test_extract_keyword_end_file() {
        assert_eq!(
            StreamParser::<Cursor<&[u8]>>::extract_keyword("КонецФайла"),
            Some("КонецФайла".to_string())
        );
    }

    #[test]
    fn test_extract_keyword_unknown() {
        assert_eq!(
            StreamParser::<Cursor<&[u8]>>::extract_keyword("НеизвестноеКлючевоеСлово"),
            None
        );
    }

    #[test]
    fn test_parse_header_field_line() {
        let mut header = FileHeader::default();
        StreamParser::<Cursor<&[u8]>>::parse_header_field_line(&mut header, "ВерсияФормата=1.03");
        assert_eq!(header.version, Some("1.03".to_string()));

        StreamParser::<Cursor<&[u8]>>::parse_header_field_line(
            &mut header,
            "РасчСчет=40702810123456789012",
        );
        assert_eq!(header.accounts.len(), 1);
    }
}
