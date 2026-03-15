//! ⭐ Модуль обнаружения файлов для пакетной обработки

use glob::glob;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::InputConfig;

/// ⭐ Результат обнаружения файлов
#[derive(Debug, Clone)]
pub struct FileDiscoveryResult {
    pub files: Vec<PathBuf>,
    pub errors: Vec<(PathBuf, String)>,
}

/// ⭐ Обнаруживает файлы на основе конфигурации
pub fn discover_files(config: &InputConfig) -> Result<FileDiscoveryResult, String> {
    let mut files = Vec::new();
    let mut errors = Vec::new();

    // ⭐ ПРИОРИТЕТ ИСТОЧНИКОВ:
    // 1. files (список конкретных файлов)
    // 2. file (одиночный файл)
    // 3. glob (шаблон)
    // 4. directory (каталог)

    // 1. Список файлов из конфига
    if !config.files.is_empty() {
        for file_path in &config.files {
            let path = PathBuf::from(file_path);
            if path.is_file() {
                files.push(path);
            } else {
                errors.push((path, "Файл не найден".to_string()));
            }
        }
    }
    // 2. Одиночный файл (если files пуст)
    else if let Some(file_path) = &config.file {
        let path = PathBuf::from(file_path);
        if path.is_file() {
            files.push(path);
        } else {
            errors.push((path, "Файл не найден".to_string()));
        }
    }
    // 3. Glob-паттерн
    else if let Some(glob_pattern) = &config.glob {
        match glob(glob_pattern) {
            Ok(paths) => {
                for entry in paths {
                    match entry {
                        Ok(path) => files.push(path),
                        Err(e) => errors.push((PathBuf::new(), format!("Ошибка glob: {}", e))),
                    }
                }
            }
            Err(e) => return Err(format!("Неверный glob-паттерн '{}': {}", glob_pattern, e)),
        }
    }
    // 4. Каталог
    else if let Some(dir_path) = &config.directory {
        let dir = PathBuf::from(dir_path);
        if !dir.is_dir() {
            return Err(format!("Каталог не найден: {}", dir_path));
        }

        if config.recursive {
            // Рекурсивный поиск
            for entry in WalkDir::new(&dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();
                if has_extension(path, &config.extensions) {
                    files.push(path.to_path_buf());
                }
            }
        } else {
            // Только текущий каталог
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_file() && has_extension(&path, &config.extensions) {
                        files.push(path);
                    }
                }
            }
        }
    }

    // Сортируем для предсказуемого порядка
    files.sort();
    files.dedup(); // ⭐ Удаляем дубликаты

    Ok(FileDiscoveryResult { files, errors })
}

/// ⭐ Проверяет, имеет ли файл одно из указанных расширений
fn has_extension(path: &Path, extensions: &[String]) -> bool {
    if let Some(ext) = path.extension() {
        if let Some(ext_str) = ext.to_str() {
            let ext_with_dot = format!(".{}", ext_str.to_lowercase());
            return extensions.iter().any(|e| {
                e.to_lowercase() == ext_with_dot || e.to_lowercase() == ext_str.to_lowercase()
            });
        }
    }
    false
}

/// ⭐ Форматирует результат для вывода
impl FileDiscoveryResult {
    pub fn summary(&self) -> String {
        format!(
            "Найдено файлов: {} | Ошибок: {}",
            self.files.len(),
            self.errors.len()
        )
    }
}
