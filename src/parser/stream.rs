use crate::parser::{
    BUFFER_SIZE, LINE_LENGTH_STRICT, MAX_LINE_LENGTH, encoding::*, models::*, state::*,
};
use std::io::{BufRead, BufReader, Read};

/// ⭐ Ошибка превышения длины строки
#[derive(Debug)]
pub struct LineLengthError {
    pub line_number: u64,
    pub actual_length: usize,
    pub max_length: usize,
}

impl std::fmt::Display for LineLengthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Превышена максимальная длина строки #{}: {} байт (максимум {})",
            self.line_number, self.actual_length, self.max_length
        )
    }
}

impl std::error::Error for LineLengthError {}

pub struct StreamParser<R: Read> {
    reader: BufReader<R>,
    state: ParserState,
    header: FileHeader,
    stats: ParseStats,
    header_buffer: String,
    header_finalized: bool,
    encoding: FileEncoding,
    pending_bytes: Vec<Vec<u8>>,
    line_number: u64,
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
            line_number: 0,
        }
    }

    pub fn parse(mut self) -> Result<(FileHeader, ParseStats), Box<dyn std::error::Error>> {
        self.detect_encoding_and_buffer_bytes()?;
        self.process_stream()?;
        self.finalize_header();

        Ok((self.header, self.stats))
    }

    /// ⭐ Читает ПОЛНЫЕ строки как БАЙТЫ, определяет кодировку
    fn detect_encoding_and_buffer_bytes(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        const DETECTION_LINES: usize = 20;
        let mut detection_bytes: Vec<u8> = Vec::new();
        let mut lines_count = 0; // ⭐ Только для ограничения цикла, не для статистики!

        while lines_count < DETECTION_LINES {
            let mut line_bytes = Vec::new();
            let bytes_read = self.reader.read_until(b'\n', &mut line_bytes)?;

            if bytes_read == 0 || line_bytes.is_empty() {
                break;
            }

            if line_bytes.len() > MAX_LINE_LENGTH {
                self.handle_line_length_exceeded(line_bytes.len())?;
                // Если не паниковали - обрезаем
                line_bytes.truncate(MAX_LINE_LENGTH);
            }

            detection_bytes.extend_from_slice(&line_bytes);
            self.pending_bytes.push(line_bytes.clone());
            lines_count += 1; // ⭐ Только для выхода из цикла (не для stats!)
        }

        // ⭐ Определяем кодировку по собранным байтам
        self.encoding = FileEncoding::detect_from_bytes_standard(&detection_bytes);
        self.header.detected_encoding = self.encoding;
        self.header.encoding = Some(format!("{:?}", self.encoding));
        Ok(())
    }

    /// ⭐ Декодируем и обрабатываем все строки с ПРАВИЛЬНОЙ кодировкой
    fn process_stream(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // ⭐ Один буфер на все строки
        let mut line_bytes = Vec::with_capacity(1024);

        // Обрабатываем pending_bytes
        let pending = std::mem::take(&mut self.pending_bytes);
        for pending_line in pending {
            self.process_line_bytes(&pending_line)?;
            if self.state == ParserState::EndOfFile {
                return Ok(());
            }
        }

        // Читаем остальной файл
        loop {
            line_bytes.clear(); // ⭐ Переиспользуем буфер!
            let bytes_read = self.reader.read_until(b'\n', &mut line_bytes)?;

            if bytes_read == 0 {
                break;
            }

            // ⭐ Проверка длины строки
            if line_bytes.len() > MAX_LINE_LENGTH {
                self.handle_line_length_exceeded(line_bytes.len())?;
                // Если не паниковали - обрезаем
                line_bytes.truncate(MAX_LINE_LENGTH);
            }

            self.process_line_bytes(&line_bytes)?;

            if self.state == ParserState::EndOfFile {
                break;
            }
        }

        // ⭐ Сохраняем общее количество строк в статистику
        self.stats.total_lines = self.line_number;

        Ok(())
    }

    /// ⭐ Обработка превышения длины строки
    fn handle_line_length_exceeded(
        &self,
        actual_length: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let error = LineLengthError {
            line_number: self.line_number,
            actual_length,
            max_length: MAX_LINE_LENGTH,
        };

        if LINE_LENGTH_STRICT {
            // ⭐ Завершаем программу с ошибкой
            eprintln!("❌ Ошибка: {}", error);
            eprintln!(
                "   Строка #{}: {} байт (максимум {})",
                self.line_number, actual_length, MAX_LINE_LENGTH
            );
            return Err(Box::new(error));
        } else {
            // ⭐ Предупреждение и обрезка
            eprintln!("⚠️  Предупреждение: {}", error);
            eprintln!("   Строка будет обрезана до {} байт", MAX_LINE_LENGTH);
        }

        Ok(())
    }

    fn process_line_bytes(&mut self, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let (cow, _, had_errors) = self.encoding.to_encoding().decode(bytes);
        self.line_number += 1;
        if had_errors {
            eprintln!(
                "⚠️  Предупреждение: ошибки декодирования в строке #{}",
                self.line_number
            );
        }

        let line = cow.trim_end_matches(&['\r', '\n'][..]);

        if line.trim().is_empty() {
            return Ok(());
        }

        self.process_line(line)
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
