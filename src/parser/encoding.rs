use encoding_rs::{Encoding, IBM866, UTF_8, WINDOWS_1251};

/// Поддерживаемые кодировки файлов 1CClientBankExchange
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileEncoding {
    Utf8,
    Windows1251,
    Cp866,
}

impl FileEncoding {
    /// Возвращает encoding_rs::Encoding для данной кодировки
    pub fn to_encoding(&self) -> &'static Encoding {
        match self {
            FileEncoding::Utf8 => UTF_8,
            FileEncoding::Windows1251 => WINDOWS_1251,
            FileEncoding::Cp866 => IBM866,
        }
    }

    /// Парсит кодировку из строки заголовка (поле "Кодировка=...")
    pub fn from_header_value(value: &str) -> Option<Self> {
        let value = value.trim().to_lowercase();

        match value.as_str() {
            "utf-8" | "utf8" | "unicode" => Some(FileEncoding::Utf8),
            "windows-1251" | "windows1251" | "cp1251" | "1251" | "ansi" => {
                Some(FileEncoding::Windows1251)
            }
            "cp866" | "866" | "dos" | "ibm866" => Some(FileEncoding::Cp866),
            // По умолчанию пробуем Windows-1251 (наиболее распространена в 1C)
            _ => None,
        }
    }

    /// Кодировка по умолчанию для файлов 1C
    pub fn default_1c() -> Self {
        FileEncoding::Windows1251
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
    fn test_from_header_value_utf8() {
        assert_eq!(
            FileEncoding::from_header_value("UTF-8"),
            Some(FileEncoding::Utf8)
        );
        assert_eq!(
            FileEncoding::from_header_value("utf8"),
            Some(FileEncoding::Utf8)
        );
        assert_eq!(
            FileEncoding::from_header_value("Unicode"),
            Some(FileEncoding::Utf8)
        );
    }

    #[test]
    fn test_from_header_value_windows1251() {
        assert_eq!(
            FileEncoding::from_header_value("Windows-1251"),
            Some(FileEncoding::Windows1251)
        );
        assert_eq!(
            FileEncoding::from_header_value("cp1251"),
            Some(FileEncoding::Windows1251)
        );
        assert_eq!(
            FileEncoding::from_header_value("ANSI"),
            Some(FileEncoding::Windows1251)
        );
    }

    #[test]
    fn test_from_header_value_cp866() {
        assert_eq!(
            FileEncoding::from_header_value("CP866"),
            Some(FileEncoding::Cp866)
        );
        assert_eq!(
            FileEncoding::from_header_value("866"),
            Some(FileEncoding::Cp866)
        );
        assert_eq!(
            FileEncoding::from_header_value("DOS"),
            Some(FileEncoding::Cp866)
        );
    }

    #[test]
    fn test_from_header_value_unknown() {
        assert_eq!(FileEncoding::from_header_value("Unknown"), None);
    }

    #[test]
    fn test_decode_windows1251() {
        // "Привет" в Windows-1251
        let bytes = [0xCF, 0xF0, 0xE8, 0xB2, 0xE5, 0xF2];
        let text = decode_bytes(&bytes, FileEncoding::Windows1251);
        assert_eq!(text, "Привет");
    }

    #[test]
    fn test_decode_cp866() {
        // "Привет" в CP-866
        let bytes = [0xE2, 0xF0, 0xE8, 0xB2, 0xE5, 0xF2];
        let text = decode_bytes(&bytes, FileEncoding::Cp866);
        assert_eq!(text, "Привет");
    }

    #[test]
    fn test_decode_utf8() {
        let bytes = "Привет".as_bytes();
        let text = decode_bytes(bytes, FileEncoding::Utf8);
        assert_eq!(text, "Привет");
    }
}
