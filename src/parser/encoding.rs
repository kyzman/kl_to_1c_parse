use encoding_rs::{Decoder, Encoding, IBM866, UTF_8, WINDOWS_1251};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileEncoding {
    Utf8,
    Windows1251, // Значение "Windows" в поле Кодировка
    Cp866,       // Значение "DOS" в поле Кодировка
}

impl FileEncoding {
    pub fn to_encoding(&self) -> &'static Encoding {
        match self {
            FileEncoding::Utf8 => UTF_8,
            FileEncoding::Windows1251 => WINDOWS_1251,
            FileEncoding::Cp866 => IBM866,
        }
    }

    /// Парсит значение поля "Кодировка=..." из стандарта 1C
    pub fn from_standard_value(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "dos" => Some(FileEncoding::Cp866),
            "windows" => Some(FileEncoding::Windows1251),
            "utf-8" | "utf8" => Some(FileEncoding::Utf8),
            _ => None,
        }
    }

    pub fn default_1c() -> Self {
        FileEncoding::Windows1251
    }

    pub fn new_decoder(&self) -> Decoder {
        self.to_encoding().new_decoder()
    }

    /// ⭐ Ищет ТОЛЬКО ASCII-паттерны =DOS, =Windows, =UTF-8 в байтах
    /// Без эвристики! Если не найдено — возвращаем Windows-1251 по умолчанию
    pub fn detect_from_bytes_standard(bytes: &[u8]) -> Self {
        if bytes.is_empty() {
            return Self::default_1c();
        }

        // Ищем "=DOS" в байтах
        for i in 0..bytes.len().saturating_sub(3) {
            if bytes[i] == b'='
                && i + 3 < bytes.len()
                && bytes[i + 1..i + 4].eq_ignore_ascii_case(b"dos")
            {
                let after = i + 4;
                if after >= bytes.len()
                    || bytes[after] == b'\n'
                    || bytes[after] == b'\r'
                    || bytes[after] == b' '
                    || bytes[after] == b'\t'
                {
                    return FileEncoding::Cp866;
                }
            }
        }

        // Ищем "=Windows" в байтах
        for i in 0..bytes.len().saturating_sub(7) {
            if bytes[i] == b'='
                && i + 7 < bytes.len()
                && bytes[i + 1..i + 8].eq_ignore_ascii_case(b"windows")
            {
                let after = i + 8;
                if after >= bytes.len()
                    || bytes[after] == b'\n'
                    || bytes[after] == b'\r'
                    || bytes[after] == b' '
                    || bytes[after] == b'\t'
                {
                    return FileEncoding::Windows1251;
                }
            }
        }

        // Ищем "=UTF-8" или "=utf8"
        for i in 0..bytes.len().saturating_sub(5) {
            if bytes[i] == b'='
                && i + 5 < bytes.len()
                && bytes[i + 1..i + 6].eq_ignore_ascii_case(b"utf-8")
            {
                return FileEncoding::Utf8;
            }
            if i + 4 < bytes.len()
                && bytes[i] == b'='
                && bytes[i + 1..i + 5].eq_ignore_ascii_case(b"utf8")
            {
                return FileEncoding::Utf8;
            }
        }

        // ⭐ По умолчанию Windows-1251 (если поле Кодировка не найдено)
        FileEncoding::Windows1251
    }
}

impl Default for FileEncoding {
    fn default() -> Self {
        Self::default_1c()
    }
}

pub fn decode_bytes(bytes: &[u8], encoding: FileEncoding) -> String {
    let (cow, _, _) = encoding.to_encoding().decode(bytes);
    cow.into_owned()
}
