use crate::parser::encoding::FileEncoding;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SectionType {
    Header,
    AccountStatement,
    Document(String),
}

#[derive(Debug, Clone, Default)]
pub struct FileHeader {
    pub version: Option<String>,
    pub encoding: Option<String>,
    pub detected_encoding: FileEncoding,
    pub sender: Option<String>,
    pub receiver: Option<String>,
    pub created_date: Option<String>,
    pub created_time: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub accounts: Vec<String>,
    pub document_types: Vec<String>,
    pub raw_content: String,
}

impl FileHeader {
    pub fn new() -> Self {
        Self {
            detected_encoding: FileEncoding::default_1c(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ParseStats {
    pub total_sections: u64,
    pub account_sections: u64,
    pub document_sections: u64,
    pub documents_by_type: HashMap<String, u64>,
    pub total_lines: u64,
    pub total_bytes: u64, // ⭐ НОВОЕ: обработанные байты
}

impl ParseStats {
    pub fn add_document(&mut self, doc_type: &str) {
        self.document_sections += 1;
        *self
            .documents_by_type
            .entry(doc_type.to_string())
            .or_insert(0) += 1;
    }

    pub fn add_line(&mut self) {
        self.total_lines += 1;
    }

    pub fn add_bytes(&mut self, bytes: u64) {
        self.total_bytes += bytes;
    }
}
