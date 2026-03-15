pub mod config;
pub mod file_discovery;
pub mod parser;
pub mod stats; // ⭐ НОВОЕ

pub use config::{CliArgs, Config, ConfigLoader, StatsMode};
pub use file_discovery::discover_files;
