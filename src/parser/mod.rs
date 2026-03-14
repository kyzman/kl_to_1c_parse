pub mod encoding;
pub mod models;
pub mod state;
pub mod stream;

/// ⭐ Максимальная длина строки в байтах (настраивается при компиляции)
/// По умолчанию 16 KБ — достаточно для любых реальных файлов 1C (Максимально рекомендуемый 1 MB)
pub const MAX_LINE_LENGTH: usize = 16 * 1024; // 16 KB

/// ⭐ Политика при превышении длины строки
/// true = завершить программу с ошибкой
/// false = обрезать строку до MAX_LINE_LENGTH
#[cfg(feature = "line-limit-error")]
pub const LINE_LENGTH_STRICT: bool = true;

#[cfg(feature = "line-limit-truncate")]
pub const LINE_LENGTH_STRICT: bool = false;

#[cfg(not(any(feature = "line-limit-error", feature = "line-limit-truncate")))]
pub const LINE_LENGTH_STRICT: bool = false; // По умолчанию обрезаем

/// Размер буфера для чтения файла (64 KB по умолчанию)
pub const BUFFER_SIZE: usize = 64 * 1024;
