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
    pending_bytes: Option<Vec<u8>>,
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
            pending_bytes: None,
        }
    }

    pub fn parse(mut self) -> Result<(FileHeader, ParseStats), Box<dyn std::error::Error>> {
        self.detect_encoding_and_buffer_bytes()?;
        self.process_stream()?;
        self.finalize_header();

        // ⭐ Проверка обязательных полей ПОСЛЕ обработки всех данных
        if self.header.version.is_none() && !self.header.raw_content.is_empty() {
            eprintln!("⚠️  Предупреждение: отсутствует поле 'ВерсияФормата'");
        }

        Ok((self.header, self.stats))
    }

    /// Определяет кодировку поиском =DOS, =Windows, =UTF-8 в первых байтах
    fn detect_encoding_and_buffer_bytes(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        const DETECTION_SIZE: usize = 1024;
        let mut detection_buffer = vec![0u8; DETECTION_SIZE];
        let mut total_read = 0;

        while total_read < DETECTION_SIZE {
            let chunk = self.reader.fill_buf()?;
            if chunk.is_empty() {
                break;
            }

            let to_read = std::cmp::min(chunk.len(), DETECTION_SIZE - total_read);
            detection_buffer[total_read..total_read + to_read].copy_from_slice(&chunk[..to_read]);
            total_read += to_read;

            let chunk_len = chunk.len();
            self.reader.consume(to_read);

            if to_read < chunk_len {
                break;
            }
        }

        if total_read > 0 {
            self.encoding =
                FileEncoding::detect_from_bytes_standard(&detection_buffer[..total_read]);
            self.header.detected_encoding = self.encoding;
            self.header.encoding = Some(format!("{:?}", self.encoding));

            // ⭐ Сохраняем байты для последующей обработки
            self.pending_bytes = Some(detection_buffer[..total_read].to_vec());
        }

        Ok(())
    }

    fn process_stream(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // ⭐ Сначала обрабатываем сохранённые байты
        if let Some(bytes) = self.pending_bytes.take() {
            // ⭐ Разбиваем на строки ДО декодирования, чтобы не потерять данные
            let mut start = 0;
            for (i, &byte) in bytes.iter().enumerate() {
                if byte == b'\n' {
                    let line_bytes = bytes[start..=i].to_vec();
                    start = i + 1;
                    self.process_bytes(line_bytes)?;

                    if self.state == ParserState::EndOfFile {
                        return Ok(());
                    }
                }
            }
            // Остаток буфера (если нет завершающего \n)
            if start < bytes.len() {
                let line_bytes = bytes[start..].to_vec();
                self.process_bytes(line_bytes)?;
            }
        }

        // ⭐ Читаем остальной файл построчно
        loop {
            let mut line_bytes = Vec::new();
            let bytes_read = self.reader.read_until(b'\n', &mut line_bytes)?;

            if bytes_read == 0 {
                break;
            }
            self.process_bytes(line_bytes)?;

            if self.state == ParserState::EndOfFile {
                break;
            }
        }

        Ok(())
    }

    fn process_bytes(&mut self, bytes: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        // ⭐ Декодируем с учётом кодировки
        let (cow, _, had_errors) = self.encoding.to_encoding().decode(&bytes);

        if had_errors {
            // Не прерываем обработку, но логируем
            eprintln!("⚠️  Предупреждение: ошибки декодирования в строке {}", cow);
        }

        let line = cow.trim_end_matches(&['\r', '\n'][..]);

        if line.is_empty() {
            return Ok(());
        }

        self.process_line(line)
    }

    fn process_line(&mut self, line: &str) -> Result<(), Box<dyn std::error::Error>> {
        // ⭐ Сначала проверяем ключевые слова
        if let Some(keyword) = Self::extract_keyword(line) {
            let old_state = self.state.clone();
            let (new_state, section_type) = self.state.transition(&keyword);

            // ⭐ Финализируем заголовок при выходе из ReadingHeader
            if matches!(old_state, ParserState::ReadingHeader)
                && !matches!(new_state, ParserState::ReadingHeader)
                && !self.header_finalized
            {
                self.finalize_header();
            }

            // ⭐ Обновляем статистику
            if let Some(section) = &section_type {
                self.stats.total_sections += 1;
                match section {
                    SectionType::AccountStatement => self.stats.account_sections += 1,
                    SectionType::Document(doc_type) => self.stats.add_document(doc_type),
                    SectionType::Header => {}
                }
            }
            self.state = new_state;
        } else {
            // ⭐ Обычная строка данных — парсим поля заголовка
            if matches!(self.state, ParserState::ReadingHeader) {
                self.header_buffer.push_str(line);
                self.header_buffer.push('\n');
                Self::parse_header_field_line(&mut self.header, line);
            }
            // Строки внутри секций можно обрабатывать при расширении функционала
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
        // ⭐ Убрали проверку version отсюда — перенесена в parse()
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
                    if let Some(enc) = FileEncoding::from_standard_value(value) {
                        if enc != header.detected_encoding {
                            eprintln!(
                                "⚠️  Предупреждение: кодировка в файле ({:?}) не совпадает с определённой ({:?})",
                                enc, header.detected_encoding
                            );
                        }
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
    fn test_parse_with_dos_encoding() {
        // Полный файл в CP-866 с РЕАЛЬНЫМИ байтами из hex-дампа
        let sample = b"1CClientBankExchange\n\
    \x82\xA5\xE0\xE1\xA8\xEF\x94\xAE\xE0\xAC\xA0\xE2\xA0=1.03\n\
    \x8A\xAE\xA4\xA8\xE0\xAE\xA2\xAA\xA0=DOS\n\
    \x91\xA5\xAA\xE6\xA8\xEF\x84\xAE\xAA\xE3\xAC\xA5\xAD\xE2=\
    \x8F\xAB\xA0\xE2\xA5\xA6\xAD\xAE\xA5\x20\xAF\xAE\xE0\xE3\xE7\xA5\xAD\xA8\xA5\n\
    \x8A\xAE\xAD\xA5\xE6\x84\xAE\xAA\xE3\xAC\xA5\xAD\xE2\xA0\n\
    \x8A\xAE\xAD\xA5\xE6\x94\xA0\xA9\xAB\xA0\n";

        let cursor = Cursor::new(sample);
        let (header, stats) = StreamParser::new(cursor).parse().unwrap();

        assert_eq!(header.detected_encoding, FileEncoding::Cp866);
        assert_eq!(header.version, Some("1.03".to_string()));
        assert_eq!(stats.document_sections, 1);
    }

    #[test]
    fn test_parse_with_windows_encoding() {
        // Полный файл в Windows-1251
        let sample = b"1CClientBankExchange\n\
    \xC2\xE5\xF0\xF1\xE8\xFF\xD4\xEE\xF0\xEC\xE0\xF2\xE0=1.03\n\
    \xCA\xEE\xE4\xE8\xF0\xEE\xB2\xEA\xE0=Windows\n\
    \xD1\xE5\xEA\xF6\xE8\xFF\xC4\xEE\xEA\xF3\xEC\xE5\xED\xF2=\
    \xCF\xEB\xE0\xF2\xE5\xF6\xED\xEE\xE5\x20\xEF\xEE\xF0\xF3\xF7\xE5\xED\xE8\xE5\n\
    \xCA\xEE\xED\xE5\xF6\xC4\xEE\xEA\xF3\xEC\xE5\xED\xF2\xE0\n\
    \xCA\xEE\xED\xE5\xF6\xD4\xE0\xB9\xEB\xE0\n";

        let cursor = Cursor::new(sample);
        let (header, stats) = StreamParser::new(cursor).parse().unwrap();

        assert_eq!(header.detected_encoding, FileEncoding::Windows1251);
        assert_eq!(header.version, Some("1.03".to_string()));
        assert_eq!(stats.document_sections, 1);
    }

    #[test]
    fn test_parse_utf8_optional() {
        let sample = concat!(
            "1CClientBankExchange\n",
            "ВерсияФормата=1.03\n",
            "Кодировка=UTF-8\n",
            "СекцияДокумент=Платежное\n",
            "КонецДокумента\n",
            "КонецФайла\n"
        );
        let cursor = Cursor::new(sample.as_bytes());
        let (header, stats) = StreamParser::new(cursor).parse().unwrap();

        assert_eq!(header.detected_encoding, FileEncoding::Utf8);
        assert_eq!(stats.document_sections, 1);
    }

    #[test]
    fn test_parse_minimal_file() {
        let mut sample = b"1CClientBankExchange\n".to_vec();
        sample.extend_from_slice(&[
            0xC2, 0xE5, 0xF0, 0xF1, 0xE8, 0xFF, 0xD4, 0xEE, 0xF0, 0xEC, 0xE0, 0xF2, 0xE0, 0x3D,
            0x31, 0x2E, 0x30, 0x33, 0x0A,
        ]); // ВерсияФормата=1.03
        sample.extend_from_slice(&[
            0xCA, 0xEE, 0xE4, 0xE8, 0xF0, 0xEE, 0xB2, 0xEA, 0xE0, 0x3D, 0x57, 0x69, 0x6E, 0x64,
            0x6F, 0x77, 0x73, 0x0A,
        ]); // Кодировка=Windows
        sample.extend_from_slice(&[
            0xCA, 0xEE, 0xED, 0xE5, 0xF6, 0xD4, 0xE0, 0xB9, 0xEB, 0xE0, 0x0A,
        ]); // КонецФайла

        let cursor = Cursor::new(sample);
        let (header, stats) = StreamParser::new(cursor).parse().unwrap();

        assert_eq!(header.version, Some("1.03".to_string()));
        assert_eq!(stats.total_sections, 1);
    }

    #[test]
    fn test_performance_large_file_simulation() {
        let mut data = String::from("1CClientBankExchange\nВерсияФормата=1.03\nКодировка=UTF-8\n");
        for i in 0..10_000 {
            data.push_str(&format!(
                "СекцияДокумент=Платежное\nНомер={}\nСумма=100.00\nКонецДокумента\n",
                i
            ));
        }
        data.push_str("КонецФайла\n");

        let cursor = Cursor::new(data.into_bytes());
        let start = std::time::Instant::now();
        let (_, stats) = StreamParser::new(cursor).parse().unwrap();
        let elapsed = start.elapsed();

        assert_eq!(stats.document_sections, 10_000);
        println!("⏱️  10 000 документов за {:.3} сек", elapsed.as_secs_f64());
        assert!(elapsed.as_secs_f64() < 1.0);
    }
}
