use encoding_rs::{Decoder, Encoding, IBM866, UTF_8, WINDOWS_1251};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileEncoding {
    Utf8,
    Windows1251,
    Cp866,
}

impl FileEncoding {
    pub fn to_encoding(&self) -> &'static Encoding {
        match self {
            FileEncoding::Utf8 => UTF_8,
            FileEncoding::Windows1251 => WINDOWS_1251,
            FileEncoding::Cp866 => IBM866,
        }
    }

    pub fn from_header_value(value: &str) -> Option<Self> {
        let value = value.trim().to_lowercase();
        match value.as_str() {
            "utf-8" | "utf8" | "unicode" => Some(FileEncoding::Utf8),
            "windows-1251" | "windows1251" | "cp1251" | "1251" | "ansi" => {
                Some(FileEncoding::Windows1251)
            }
            "cp866" | "866" | "dos" | "ibm866" => Some(FileEncoding::Cp866),
            _ => None,
        }
    }

    pub fn default_1c() -> Self {
        FileEncoding::Windows1251
    }

    pub fn new_decoder(&self) -> Decoder {
        self.to_encoding().new_decoder()
    }

    /// Эвристическое определение кодировки по байтам
    /// Анализирует паттерны байт характерные для разных кодировок
    pub fn detect_from_bytes(bytes: &[u8]) -> Self {
        if bytes.is_empty() {
            return FileEncoding::default_1c();
        }

        let mut utf8_score = 0;
        let mut cp1251_score = 0;
        let mut cp866_score = 0;
        let mut invalid_utf8 = false;

        let mut i = 0;
        while i < bytes.len() {
            let byte = bytes[i];

            if byte < 0x80 {
                // ASCII - валиден для всех кодировок
                i += 1;
                continue;
            }

            // Проверяем UTF-8 последовательности
            if byte >= 0xC0 && byte <= 0xDF {
                // 2-байтная последовательность UTF-8
                if i + 1 < bytes.len() && bytes[i + 1] >= 0x80 && bytes[i + 1] <= 0xBF {
                    utf8_score += 2;
                    i += 2;
                    continue;
                }
                invalid_utf8 = true;
            } else if byte >= 0xE0 && byte <= 0xEF {
                // 3-байтная последовательность UTF-8
                if i + 2 < bytes.len()
                    && bytes[i + 1] >= 0x80
                    && bytes[i + 1] <= 0xBF
                    && bytes[i + 2] >= 0x80
                    && bytes[i + 2] <= 0xBF
                {
                    utf8_score += 3;
                    i += 3;
                    continue;
                }
                invalid_utf8 = true;
            } else if byte >= 0x80 && byte <= 0xBF {
                // Продолжение UTF-8 без начала - невалидно
                invalid_utf8 = true;
            }

            // Для single-byte кодировок считаем байты в диапазонах кириллицы
            if byte >= 0x80 {
                cp1251_score += 1;
                cp866_score += 1;

                // Специфичные диапазоны
                if byte >= 0x80 && byte <= 0x9F {
                    cp1251_score += 1; // Чаще в Windows-1251
                }
                if byte >= 0xF0 {
                    cp866_score += 1; // Чаще в CP-866
                }
            }

            i += 1;
        }

        // Если UTF-8 валиден и есть кириллические символы - это UTF-8
        if !invalid_utf8 && utf8_score > cp1251_score && utf8_score > cp866_score {
            return FileEncoding::Utf8;
        }

        // Иначе выбираем между Windows-1251 и CP-866
        if cp1251_score >= cp866_score {
            FileEncoding::Windows1251
        } else {
            FileEncoding::Cp866
        }
    }
}

impl Default for FileEncoding {
    fn default() -> Self {
        Self::default_1c()
    }
}

/// Конвертирует байты в строку с учётом кодировки
pub fn decode_bytes(bytes: &[u8], encoding: FileEncoding) -> String {
    let (cow, _, _) = encoding.to_encoding().decode(bytes);
    cow.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_header_value() {
        assert_eq!(
            FileEncoding::from_header_value("UTF-8"),
            Some(FileEncoding::Utf8)
        );
        assert_eq!(
            FileEncoding::from_header_value("windows-1251"),
            Some(FileEncoding::Windows1251)
        );
        assert_eq!(
            FileEncoding::from_header_value("CP866"),
            Some(FileEncoding::Cp866)
        );
        assert_eq!(FileEncoding::from_header_value("Unknown"), None);
    }

    #[test]
    fn test_new_decoder() {
        let decoder = FileEncoding::Windows1251.new_decoder();
        assert_eq!(decoder.encoding(), WINDOWS_1251);
    }

    #[test]
    fn test_decode_windows1251() {
        // "Привет" в Windows-1251
        let bytes: Vec<u8> = vec![0xCF, 0xF0, 0xE8, 0xB2, 0xE5, 0xF2];
        let text = decode_bytes(&bytes, FileEncoding::Windows1251);
        assert_eq!(text, "Привет");
    }

    #[test]
    fn test_decode_cp866() {
        // "Привет" в CP-866
        let bytes: Vec<u8> = vec![0xE2, 0xF0, 0xE8, 0xB2, 0xE5, 0xF2];
        let text = decode_bytes(&bytes, FileEncoding::Cp866);
        assert_eq!(text, "Привет");
    }

    #[test]
    fn test_decode_utf8() {
        let bytes = "Привет".as_bytes();
        let text = decode_bytes(bytes, FileEncoding::Utf8);
        assert_eq!(text, "Привет");
    }

    #[test]
    fn test_detect_utf8() {
        let bytes = "1CClientBankExchange\nВерсияФормата=1.03".as_bytes();
        let detected = FileEncoding::detect_from_bytes(bytes);
        assert_eq!(detected, FileEncoding::Utf8);
    }

    #[test]
    fn test_detect_windows1251() {
        // "1CClientBankExchange\nВерсия" в Windows-1251
        let mut bytes = b"1CClientBankExchange\n".to_vec();
        bytes.extend_from_slice(&[0xC2, 0xE5, 0xF0, 0xF1, 0xE8, 0xFF]); // Версия
        let detected = FileEncoding::detect_from_bytes(&bytes);
        assert_eq!(detected, FileEncoding::Windows1251);
    }

    #[test]
    fn test_detect_cp866() {
        // "1CClientBankExchange\nВерсия" в CP-866
        let mut bytes = b"1CClientBankExchange\n".to_vec();
        bytes.extend_from_slice(&[0xE2, 0xF0, 0xE8, 0xF1, 0xE8, 0xFF]); // Версия
        let detected = FileEncoding::detect_from_bytes(&bytes);
        assert_eq!(detected, FileEncoding::Cp866);
    }
}
