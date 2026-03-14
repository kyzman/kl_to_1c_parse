use load1c::parser::encoding::FileEncoding;
use load1c::parser::stream::StreamParser;
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
        "СекцияДокумент=Платежное поручение\n",
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
        0xC2, 0xE5, 0xF0, 0xF1, 0xE8, 0xFF, 0xD4, 0xEE, 0xF0, 0xEC, 0xE0, 0xF2, 0xE0, 0x3D, 0x31,
        0x2E, 0x30, 0x33, 0x0A,
    ]);
    sample.extend_from_slice(&[
        0xCA, 0xEE, 0xE4, 0xE8, 0xF0, 0xEE, 0xB2, 0xEA, 0xE0, 0x3D, 0x57, 0x69, 0x6E, 0x64, 0x6F,
        0x77, 0x73, 0x0A,
    ]);
    sample.extend_from_slice(&[
        0xCA, 0xEE, 0xED, 0xE5, 0xF6, 0xD4, 0xE0, 0xB9, 0xEB, 0xE0, 0x0A,
    ]);

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
