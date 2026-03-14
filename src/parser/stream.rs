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
    pending_bytes: Vec<Vec<u8>>, // ⭐ Храним СЫРЫЕ БАЙТЫ строк, не декодированные строки
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
            pending_bytes: Vec::new(),
        }
    }

    pub fn parse(mut self) -> Result<(FileHeader, ParseStats), Box<dyn std::error::Error>> {
        self.detect_encoding_and_buffer_bytes()?;
        self.process_stream()?;
        self.finalize_header();

        Ok((self.header, self.stats))
    }

    /// ⭐ Читает ПОЛНЫЕ строки как БАЙТЫ, определяет кодировку, не декодируя заранее
    fn detect_encoding_and_buffer_bytes(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        const DETECTION_LINES: usize = 20;
        let mut detection_bytes: Vec<u8> = Vec::new();
        let mut lines_count = 0;

        // ⭐ Читаем ПОЛНЫЕ строки как БАЙТЫ (не декодируем!)
        while lines_count < DETECTION_LINES {
            let mut line_bytes = Vec::new();
            let bytes_read = self.reader.read_until(b'\n', &mut line_bytes)?;

            if bytes_read == 0 {
                break;
            }

            // Сохраняем сырые байты для последующего декодирования
            detection_bytes.extend_from_slice(&line_bytes);
            self.pending_bytes.push(line_bytes.clone());
            lines_count += 1;

            // ⭐ Ищем "=DOS", "=Windows", "=UTF-8" в сырых байтах (ASCII-паттерны)
            if let Ok(line) = std::str::from_utf8(&line_bytes) {
                let line = line.trim_end_matches(&['\r', '\n'][..]);

                if line.starts_with("Секция") && !line.starts_with("1CClientBankExchange") {
                    break;
                }
            }
        }

        // ⭐ Определяем кодировку по собранным байтам (если не нашли в поле)
        if self.header.encoding.is_none() {
            self.encoding = FileEncoding::detect_from_bytes_standard(&detection_bytes);
            self.header.encoding = Some(format!("{:?}", self.encoding));
        }

        self.header.detected_encoding = self.encoding;

        Ok(())
    }

    /// ⭐ Декодируем и обрабатываем все строки с ПРАВИЛЬНОЙ кодировкой
    fn process_stream(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // ⭐ Сначала обрабатываем сохранённые байты с правильной кодировкой
        let pending = std::mem::take(&mut self.pending_bytes);
        for line_bytes in pending {
            let (cow, _, had_errors) = self.encoding.to_encoding().decode(&line_bytes);

            if had_errors {
                eprintln!("⚠️  Предупреждение: ошибки декодирования в строке");
            }

            let line = cow.trim_end_matches(&['\r', '\n'][..]);

            if line.trim().is_empty() {
                continue;
            }

            self.process_line(line)?;

            if self.state == ParserState::EndOfFile {
                return Ok(());
            }
        }

        // ⭐ Читаем остальной файл ПОЛНЫМИ строками
        loop {
            let mut line_bytes = Vec::new();
            let bytes_read = self.reader.read_until(b'\n', &mut line_bytes)?;

            if bytes_read == 0 {
                break;
            }

            let (cow, _, had_errors) = self.encoding.to_encoding().decode(&line_bytes);

            if had_errors {
                eprintln!("⚠️  Предупреждение: ошибки декодирования в строке");
            }

            let line = cow.trim_end_matches(&['\r', '\n'][..]);

            if line.trim().is_empty() {
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

            if matches!(old_state, ParserState::ReadingHeader)
                && !matches!(new_state, ParserState::ReadingHeader)
                && !self.header_finalized
            {
                self.finalize_header();
            }

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
                "Кодировка" => header.encoding = Some(value.to_string()), // кодировка в любом случае уже должна быть определена, поэтому без вариантов
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
