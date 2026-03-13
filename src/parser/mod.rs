pub mod encoding;
pub mod models;
pub mod state;
pub mod stream;

/// Размер буфера для чтения файла (можно изменить при компиляции)
/// Оптимально: 32-256 КБ для баланса между памятью и производительностью
pub const BUFFER_SIZE: usize = 64 * 1024;
