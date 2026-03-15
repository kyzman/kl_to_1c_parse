use super::models::{Config, ConfigError};
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value;
use toml::map::Map;

/// ⭐ Загрузчик конфигурации с поддержкой includes
pub struct ConfigLoader {
    base_path: PathBuf,
}

impl ConfigLoader {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    /// ⭐ Загружает и объединяет несколько конфигов
    pub fn load(&self, paths: &[&str]) -> Result<Config, ConfigError> {
        let mut merged = Map::new();

        // Загружаем каждый файл и сливаем
        for path in paths {
            let full_path = self.base_path.join(path);
            let content = fs::read_to_string(&full_path)
                .map_err(|e| ConfigError::IoError(path.to_string(), e))?;

            let toml_value: Value = toml::from_str(&content)
                .map_err(|e| ConfigError::ParseError(path.to_string(), e))?;

            // Рекурсивно обрабатываем includes
            let processed = self.process_includes(toml_value, &full_path)?;

            // Сливаем с предыдущими
            merged = Self::merge_maps(merged, processed);
        }

        // Конвертируем в Config через serde
        let config: Config = Value::Table(merged)
            .try_into()
            .map_err(ConfigError::DeserializationError)?;

        // Валидация
        config.validate()?;

        Ok(config)
    }

    /// ⭐ Обрабатывает директиву [includes]
    fn process_includes(
        &self,
        mut value: Value,
        current_path: &Path,
    ) -> Result<Map<String, Value>, ConfigError> {
        // Извлекаем includes
        let includes = if let Value::Table(ref table) = value {
            table.get("includes").cloned()
        } else {
            None
        };

        // Удаляем includes из текущего конфига
        if let Value::Table(ref mut table) = value {
            table.remove("includes");
        }

        let mut result = Self::value_to_map(value)?;

        // Загружаем включённые файлы
        if let Some(Value::Table(includes_table)) = includes {
            if let Some(Value::Array(files)) = includes_table.get("files") {
                for file in files {
                    if let Value::String(file_path) = file {
                        let include_path = current_path
                            .parent()
                            .unwrap_or(&self.base_path)
                            .join(file_path);

                        let content = fs::read_to_string(&include_path)
                            .map_err(|e| ConfigError::IoError(file_path.clone(), e))?;

                        let included_value: Value = toml::from_str(&content)
                            .map_err(|e| ConfigError::ParseError(file_path.clone(), e))?;

                        // Рекурсивно обрабатываем includes во вложенных файлах
                        let processed = self.process_includes(included_value, &include_path)?;

                        // Сливаем
                        result = Self::merge_maps(result, processed);
                    }
                }
            }
        }

        Ok(result)
    }

    /// ⭐ Сливает два Map (второй переопределяет первый)
    fn merge_maps(
        mut base: Map<String, Value>,
        override_map: Map<String, Value>,
    ) -> Map<String, Value> {
        for (key, value) in override_map {
            if let Some(existing) = base.get_mut(&key) {
                // ⭐ Используем as_table_mut() вместо паттерн-матчинга
                // Это не двигает `existing`, поэтому можно использовать его позже
                if let (Some(base_table), Some(override_table)) =
                    (existing.as_table_mut(), value.as_table())
                {
                    // Рекурсивно сливаем таблицы
                    for (k, v) in override_table.clone() {
                        base_table.insert(k, v);
                    }
                } else {
                    // Иначе просто переопределяем
                    *existing = value;
                }
            } else {
                // Ключа нет в base — просто добавляем
                base.insert(key, value);
            }
        }
        base
    }

    /// ⭐ Конвертирует Value в Map
    fn value_to_map(value: Value) -> Result<Map<String, Value>, ConfigError> {
        match value {
            Value::Table(table) => Ok(table),
            _ => Err(ConfigError::InvalidStructure("Expected table".to_string())),
        }
    }
}
