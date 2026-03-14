use load1c::parser::encoding::{FileEncoding, decode_bytes};

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
    let bytes = b"1CClientBankExchange\n\xCB\xEF\xE4\xE8\xF0\xEF\xB2\xEA\xE0=DOS\n";
    let detected = FileEncoding::detect_from_bytes_standard(bytes);
    assert_eq!(detected, FileEncoding::Cp866);
}

#[test]
fn test_detect_from_bytes_with_windows() {
    let bytes = b"1CClientBankExchange\n\xC2\xE5\xF0\xF1\xE8\xFF\xD4\xEE\xF0\xEC\xE0\xF2\xE0=1.03\n\xCA\xEE\xE4\xE8\xF0\xEE\xB2\xEA\xE0=Windows\n";
    let detected = FileEncoding::detect_from_bytes_standard(bytes);
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
