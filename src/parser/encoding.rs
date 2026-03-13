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
                && bytes[i + 1] == b'D'
                && bytes[i + 2] == b'O'
                && bytes[i + 3] == b'S'
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
                && bytes[i + 1] == b'W'
                && bytes[i + 2] == b'i'
                && bytes[i + 3] == b'n'
                && bytes[i + 4] == b'd'
                && bytes[i + 5] == b'o'
                && bytes[i + 6] == b'w'
                && bytes[i + 7] == b's'
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
                && bytes[i + 1] == b'U'
                && bytes[i + 2] == b'T'
                && bytes[i + 3] == b'F'
                && bytes[i + 4] == b'-'
                && bytes[i + 5] == b'8'
            {
                return FileEncoding::Utf8;
            }
            if i + 4 < bytes.len()
                && bytes[i] == b'='
                && bytes[i + 1] == b'u'
                && bytes[i + 2] == b't'
                && bytes[i + 3] == b'f'
                && bytes[i + 4] == b'8'
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_standard_value() {
        assert_eq!(
            FileEncoding::from_standard_value("DOS"),
            Some(FileEncoding::Cp866)
        );
        assert_eq!(
            FileEncoding::from_standard_value("Windows"),
            Some(FileEncoding::Windows1251)
        );
        assert_eq!(
            FileEncoding::from_standard_value("UTF-8"),
            Some(FileEncoding::Utf8)
        );
        assert_eq!(
            FileEncoding::from_standard_value("dos"),
            Some(FileEncoding::Cp866)
        );
        assert_eq!(
            FileEncoding::from_standard_value("WINDOWS"),
            Some(FileEncoding::Windows1251)
        );
        assert_eq!(FileEncoding::from_standard_value("Unknown"), None);
    }

    #[test]
    fn test_detect_from_bytes_with_dos() {
        let mut bytes = b"1CClientBankExchange\n".to_vec();
        bytes.extend_from_slice(&[0xCB, 0xEF, 0xE4, 0xE8, 0xF0, 0xEF, 0xB2, 0xEA, 0xE0]); // Кодировка (CP-866)
        bytes.extend_from_slice(b"=DOS\n");
        let detected = FileEncoding::detect_from_bytes_standard(&bytes);
        assert_eq!(detected, FileEncoding::Cp866);
    }

    #[test]
    fn test_detect_from_bytes_with_windows() {
        let mut bytes = b"1CClientBankExchange\n".to_vec();
        bytes.extend_from_slice(&[0xCA, 0xEE, 0xE4, 0xE8, 0xF0, 0xEE, 0xB2, 0xEA, 0xE0]); // Кодировка (Windows-1251)
        bytes.extend_from_slice(b"=Windows\n");
        let detected = FileEncoding::detect_from_bytes_standard(&bytes);
        assert_eq!(detected, FileEncoding::Windows1251);
    }

    #[test]
    fn test_detect_from_bytes_with_utf8() {
        let bytes = "1CClientBankExchange\nКодировка=UTF-8\nВерсияФормата=1.03".as_bytes();
        let detected = FileEncoding::detect_from_bytes_standard(bytes);
        assert_eq!(detected, FileEncoding::Utf8);
    }

    #[test]
    fn test_detect_from_bytes_default() {
        // Если нет поля Кодировка — возвращаем Windows-1251 по умолчанию
        let bytes =
            b"1CClientBankExchange\n\xC2\xE5\xF0\xF1\xE8\xFF\xD4\xEE\xF0\xEC\xE0\xF2\xE0=1.03\n";
        let detected = FileEncoding::detect_from_bytes_standard(bytes);
        assert_eq!(detected, FileEncoding::Windows1251);
    }

    #[test]
    fn test_decode_windows1251() {
        let bytes = b"\xCF\xF0\xE8\xE2\xE5\xF2";
        let text = decode_bytes(bytes, FileEncoding::Windows1251);
        assert_eq!(text, "Привет");
    }

    #[test]
    fn test_decode_cp866() {
        let bytes = b"\x8F\xE0\xA8\xA2\xA5\xE2";
        let text = decode_bytes(bytes, FileEncoding::Cp866);
        assert_eq!(text, "Привет");
    }

    #[test]
    fn test_decode_utf8() {
        let bytes = "Привет".as_bytes();
        let text = decode_bytes(bytes, FileEncoding::Utf8);
        assert_eq!(text, "Привет");
    }
}
