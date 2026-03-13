use crate::parser::{encoding::*, models::*, state::*};
use std::io::{BufRead, BufReader, Read};

/// Потоковый парсер файлов 1CClientBankExchange
pub struct StreamParser<R: Read> {
    reader: BufReader<R>,
    state: ParserState,
    header: FileHeader,
    stats: ParseStats,
    header_buffer: String,
    header_finalized: bool,
    encoding: FileEncoding,
    raw_bytes: Vec<u8>, // Накопление байтов для перекодировки
}

impl<R: Read> StreamParser<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::with_capacity(64 * 1024, reader),
            state: ParserState::WaitingHeader,
            header: FileHeader::new(),
            stats: ParseStats::default(),
            header_buffer: String::new(),
            header_finalized: false,
            encoding: FileEncoding::default_1c(),
            raw_bytes: Vec::new(),
        }
    }

    /// Запускает парсинг с автоопределением кодировки
    pub fn parse(mut self) -> Result<(FileHeader, ParseStats), Box<dyn std::error::Error>> {
        // Сначала читаем весь файл в байты для определения кодировки
        // (для потоковой обработки больших файлов можно оптимизировать)
        self.reader.read_to_end(&mut self.raw_bytes)?;

        // Пытаемся определить кодировку из первых строк заголовка
        self.detect_encoding_from_header()?;

        // Конвертируем в UTF-8 для внутренней обработки
        let content = decode_bytes(&self.raw_bytes, self.encoding);
        let lines: Vec<&str> = content.lines().collect();

        // Парсим строки
        for line in lines {
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

    /// Определяет кодировку из поля "Кодировка=" в заголовке
    fn detect_encoding_from_header(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Пробуем распарсить первые строки в разных кодировках для поиска поля Кодировка=
        let test_encodings = [
            FileEncoding::Utf8,
            FileEncoding::Windows1251,
            FileEncoding::Cp866,
        ];

        for encoding in test_encodings {
            let (cow, _, _) = encoding.to_encoding().decode(&self.raw_bytes);

            for line in cow.lines() {
                if let Some((key, value)) = line.split_once('=') {
                    if key.trim() == "Кодировка" || key.trim() == "Encoding" {
                        if let Some(detected) = FileEncoding::from_header_value(value) {
                            self.encoding = detected;
                            self.header.encoding = Some(value.trim().to_string());
                            return Ok(());
                        }
                    }
                }

                // Останавливаемся после первой секции
                if line.starts_with("Секция") || line.starts_with("1CClientBankExchange") {
                    break;
                }
            }
        }

        // Если не нашли - используем кодировку по умолчанию
        self.header.encoding = Some("Windows-1251 (по умолчанию)".to_string());
        Ok(())
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
        self.header.detected_encoding = self.encoding;
        self.header_finalized = true;

        if self.header.version.is_none() && !self.header.raw_content.is_empty() {
            eprintln!("⚠️  Предупреждение: отсутствует поле 'ВерсияФормата'");
        }
    }

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

    fn parse_header_field_line(header: &mut FileHeader, line: &str) {
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            match key {
                "ВерсияФормата" => header.version = Some(value.to_string()),
                "Кодировка" => {
                    header.encoding = Some(value.to_string());
                    if let Some(enc) = FileEncoding::from_header_value(value) {
                        header.detected_encoding = enc;
                    }
                }
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
    fn test_parse_utf8_file() {
        let sample = concat!(
            "1CClientBankExchange\n",
            "ВерсияФормата=1.03\n",
            "Кодировка=UTF-8\n",
            "СекцияДокумент=Платежное поручение\n",
            "КонецДокумента\n",
            "КонецФайла\n"
        );
        let cursor = Cursor::new(sample.as_bytes());
        let (header, stats) = StreamParser::new(cursor).parse().unwrap();

        assert_eq!(header.detected_encoding, FileEncoding::Utf8);
        assert_eq!(stats.document_sections, 1);
    }

    #[test]
    fn test_parse_windows1251_file() {
        // "1CClientBankExchange\nВерсияФормата=1.03\nКодировка=Windows-1251\n" в Windows-1251
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"1CClientBankExchange\n");
        bytes.extend_from_slice(&[
            0xC2, 0xE5, 0xF0, 0xF1, 0xE8, 0xFF, 0xD4, 0xEE, 0xF0, 0xEC, 0xE0, 0xF2, 0xE0, 0x3D,
            0x31, 0x2E, 0x30, 0x33, 0x0A,
        ]); // ВерсияФормата=1.03
        bytes.extend_from_slice(&[
            0xCA, 0xEE, 0xE4, 0xE8, 0xF0, 0xEE, 0xB2, 0xEA, 0xE0, 0x3D, 0x57, 0x69, 0x6E, 0x64,
            0x6F, 0x77, 0x73, 0x2D, 0x31, 0x32, 0x35, 0x31, 0x0A,
        ]); // Кодировка=Windows-1251
        bytes.extend_from_slice(&[
            0xD1, 0xE5, 0xEA, 0xF6, 0xE8, 0xFF, 0xC4, 0xEE, 0xEA, 0xF3, 0xEC, 0xE5, 0xED, 0xF2,
            0x3D, 0xCF, 0xEB, 0xE0, 0xF2, 0xE5, 0xF6, 0xED, 0xEE, 0xE5, 0x20, 0xEF, 0xEE, 0xF0,
            0xF3, 0xF7, 0xE5, 0xED, 0xE8, 0xE5, 0x0A,
        ]); // СекцияДокумент=Платежное поручение
        bytes.extend_from_slice(b"\xCA, 0xEE, 0xED, 0xE5, 0xF6, 0xC4, 0xEE, 0xEA, 0xF3, 0xEC, 0xE5, 0xED, 0xF2, 0xE0, 0x0A"); // КонецДокумента
        bytes
            .extend_from_slice(b"\xCA, 0xEE, 0xED, 0xE5, 0xF6, 0xD4, 0xE0, 0xB9, 0xEB, 0xE0, 0x0A"); // КонецФайла

        let cursor = Cursor::new(bytes);
        let (header, stats) = StreamParser::new(cursor).parse().unwrap();

        assert_eq!(header.detected_encoding, FileEncoding::Windows1251);
        assert_eq!(stats.document_sections, 1);
    }

    #[test]
    fn test_parse_no_encoding_specified() {
        let sample = concat!(
            "1CClientBankExchange\n",
            "ВерсияФормата=1.03\n",
            "СекцияДокумент=Тест\n",
            "КонецДокумента\n",
            "КонецФайла\n"
        );
        let cursor = Cursor::new(sample.as_bytes());
        let (header, _) = StreamParser::new(cursor).parse().unwrap();

        // По умолчанию должна быть Windows-1251
        assert_eq!(header.detected_encoding, FileEncoding::Windows1251);
    }

    // ... остальные тесты из предыдущей версии
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
