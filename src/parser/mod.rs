pub mod encoding;
pub mod models;
pub mod state;
pub mod stream;

// ⭐ Константы удалены - теперь загружаются из конфигурации!
// Используйте config.parser.buffer_size и т.д.

// ⭐ Максимальная длина строки в байтах (нужна исключительно для тестов, в коде не используется)
pub const MAX_LINE_LENGTH: usize = 16 * 1024; // 16 KB
