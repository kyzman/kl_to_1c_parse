use crate::parser::{BUFFER_SIZE, encoding::*, models::*, state::*};
use std::io::{BufRead, BufReader, Read};

pub struct StreamParser<R: Read> {
    reader: BufReader<R>,
    state: ParserState,
    header: FileHeader,
    stats: ParseStats,
    header_buffer: String,
    header_finalized: bool,
    encoding: FileEncoding,
    encoding_detected: bool,
}

impl<R: Read> StreamParser<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::with_capacity(BUFFER_SIZE, reader),
            state: ParserState::WaitingHeader,
            header: FileHeader::new(),
            stats: ParseStats::default(),
            header_buffer: String::new(),
            header_finalized: false,
            encoding: FileEncoding::default_1c(),
            encoding_detected: false,
        }
    }

    pub fn parse(mut self) -> Result<(FileHeader, ParseStats), Box<dyn std::error::Error>> {
        self.detect_encoding_and_process_header()?;
        self.process_stream()?;
        self.finalize_header();

        Ok((self.header, self.stats))
    }

    /// Определяет кодировку и сразу обрабатывает строки заголовка
    fn detect_encoding_and_process_header(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut raw_lines: Vec<Vec<u8>> = Vec::new();
        let mut lines_count = 0;

        // Читаем первые строки для определения кодировки
        loop {
            let mut line_bytes = Vec::new();
            let bytes_read = self.reader.read_until(b'\n', &mut line_bytes)?;

            if bytes_read == 0 {
                break;
            }

            raw_lines.push(line_bytes);
            lines_count += 1;

            // Проверяем последнюю строку на наличие поля Кодировка=
            if let Ok(line) = std::str::from_utf8(raw_lines.last().unwrap()) {
                if let Some((key, value)) = line.split_once('=') {
                    if key.trim() == "Кодировка" || key.trim() == "Encoding" {
                        if let Some(detected) = FileEncoding::from_header_value(value) {
                            self.encoding = detected;
                            self.header.encoding = Some(value.trim().to_string());
                            self.encoding_detected = true;
                        }
                    }
                }

                // Останавливаемся после первой секции или 15 строк
                if line.starts_with("Секция") && !line.starts_with("1CClientBankExchange") {
                    break;
                }
                if lines_count >= 15 {
                    break;
                }
            }
        }

        // Кодировка по умолчанию
        if !self.encoding_detected {
            self.encoding = FileEncoding::Windows1251;
            self.header.encoding = Some("Windows-1251 (по умолчанию)".to_string());
        }

        // Декодируем и обрабатываем все прочитанные строки
        for line_bytes in raw_lines {
            let (cow, _, _) = self.encoding.to_encoding().decode(&line_bytes);
            let line = cow.trim_end_matches(&['\r', '\n'][..]);

            if line.is_empty() {
                continue;
            }

            // Сразу обрабатываем строку через машину состояний
            self.process_line(line)?;

            if self.state == ParserState::EndOfFile {
                return Ok(());
            }
        }

        Ok(())
    }

    /// Потоковая обработка остального файла
    fn process_stream(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let mut line_bytes = Vec::new();
            let bytes_read = self.reader.read_until(b'\n', &mut line_bytes)?;

            if bytes_read == 0 {
                break;
            }

            let (cow, _, _) = self.encoding.to_encoding().decode(&line_bytes);
            let line = cow.trim_end_matches(&['\r', '\n'][..]);

            if line.is_empty() {
                continue;
            }

            self.process_line(line)?;

            if self.state == ParserState::EndOfFile {
                break;
            }
        }

        Ok(())
    }

    fn process_line(&mut self, line: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(keyword) = Self::extract_keyword(line) {
            let old_state = self.state.clone();
            let (new_state, section_type) = self.state.transition(&keyword);

            // Финализируем заголовок при выходе из ReadingHeader
            if matches!(old_state, ParserState::ReadingHeader)
                && !matches!(new_state, ParserState::ReadingHeader)
                && !self.header_finalized
            {
                self.finalize_header();
            }

            // Обновляем статистику
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
            // Обычная строка данных
            if matches!(self.state, ParserState::ReadingHeader) {
                self.header_buffer.push_str(line);
                self.header_buffer.push('\n');
                Self::parse_header_field_line(&mut self.header, line);
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
    fn test_buffer_size_constant() {
        assert!(BUFFER_SIZE >= 1024);
        assert!(BUFFER_SIZE <= 1024 * 1024);
    }

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
        assert_eq!(header.version, Some("1.03".to_string()));
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

        assert_eq!(header.detected_encoding, FileEncoding::Windows1251);
        assert_eq!(header.version, Some("1.03".to_string()));
    }

    #[test]
    fn test_parse_minimal_file() {
        let sample = "1CClientBankExchange\nВерсияФормата=1.03\nКонецФайла\n";
        let cursor = Cursor::new(sample.as_bytes());
        let (header, stats) = StreamParser::new(cursor).parse().unwrap();

        assert_eq!(header.version, Some("1.03".to_string()));
        assert_eq!(stats.total_sections, 1);
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
        assert_eq!(header.accounts.len(), 2);
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
    fn test_empty_file() {
        let sample = "";
        let cursor = Cursor::new(sample.as_bytes());
        let (_, stats) = StreamParser::new(cursor).parse().unwrap();

        assert_eq!(stats.total_sections, 0);
    }

    #[test]
    fn test_performance_large_file_simulation() {
        let mut data = String::from("1CClientBankExchange\nВерсияФормата=1.03\nКодировка=UTF-8\n");
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
        assert!(elapsed.as_secs_f64() < 1.0);
    }

    #[test]
    fn test_extract_keyword_functions() {
        assert_eq!(
            StreamParser::<Cursor<&[u8]>>::extract_keyword("1CClientBankExchange"),
            Some("1CClientBankExchange".to_string())
        );
        assert_eq!(
            StreamParser::<Cursor<&[u8]>>::extract_keyword("СекцияДокумент=Платежное поручение"),
            Some("СекцияДокумент=Платежное поручение".to_string())
        );
        assert_eq!(
            StreamParser::<Cursor<&[u8]>>::extract_keyword("НеизвестноеКлючевоеСлово"),
            None
        );
    }

    #[test]
    fn test_parse_header_field_line() {
        let mut header = FileHeader::new();
        StreamParser::<Cursor<&[u8]>>::parse_header_field_line(&mut header, "ВерсияФормата=1.03");
        assert_eq!(header.version, Some("1.03".to_string()));

        StreamParser::<Cursor<&[u8]>>::parse_header_field_line(
            &mut header,
            "РасчСчет=40702810123456789012",
        );
        assert_eq!(header.accounts.len(), 1);
    }
}
