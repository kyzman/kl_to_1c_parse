use load1c::parser::MAX_LINE_LENGTH;
use load1c::parser::stream::StreamParser;
use std::io::Cursor;

#[test]
fn test_max_line_length_constant() {
    // Проверяем, что константа имеет разумное значение
    assert!(MAX_LINE_LENGTH >= 1024);
    assert!(MAX_LINE_LENGTH <= 1024 * 1024); // Не больше 1 MB
}

#[test]
fn test_normal_line_length() {
    // Нормальная строка (меньше лимита)
    let sample = concat!(
        "1CClientBankExchange\n",
        "ВерсияФормата=1.03\n",
        "Кодировка=UTF-8\n",
        "КонецФайла\n"
    );
    let cursor = Cursor::new(sample.as_bytes());
    let result = StreamParser::new(cursor).parse();
    assert!(result.is_ok());
}

#[test]
fn test_very_long_line_truncate_mode() {
    // Тест работает только в режиме truncate
    #[cfg(feature = "line-limit-truncate")]
    {
        let mut data = String::from("1CClientBankExchange\nВерсияФормата=1.03\nКодировка=UTF-8\n");
        // Создаём строку длиной 2 MB (больше лимита 1 MB)
        data.push_str(&"x".repeat(2 * 1024 * 1024));
        data.push_str("\nКонецФайла\n");

        let cursor = Cursor::new(data.into_bytes());
        let result = StreamParser::new(cursor).parse();

        // В режиме truncate должно успешно завершиться
        assert!(result.is_ok());
    }
}

#[test]
fn test_very_long_line_error_mode() {
    // Тест работает только в режиме error
    #[cfg(feature = "line-limit-error")]
    {
        let mut data = String::from("1CClientBankExchange\nВерсияФормата=1.03\nКодировка=UTF-8\n");
        // Создаём строку длиной 2 MB (больше лимита 1 MB)
        data.push_str(&"x".repeat(2 * 1024 * 1024));
        data.push_str("\nКонецФайла\n");

        let cursor = Cursor::new(data.into_bytes());
        let result = StreamParser::new(cursor).parse();

        // В режиме error должно завершиться с ошибкой
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Превышена максимальная длина строки"));
    }
}
