use chrono::{NaiveDateTime, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::{Arc, RwLock};

use crate::utils;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogMode {
    Full,
    FileOnly,
    Disabled,
}

impl LogMode {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "file_only" => LogMode::FileOnly,
            "disabled" => LogMode::Disabled,
            _ => LogMode::Full,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warning => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LogCategory {
    Core,
    P2P,
    RPC,
}

impl fmt::Display for LogCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogCategory::Core => write!(f, "Core"),
            LogCategory::P2P => write!(f, "P2P"),
            LogCategory::RPC => write!(f, "RPC"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: NaiveDateTime,
    pub level: LogLevel,
    pub category: LogCategory,
    pub message: String,
}

impl fmt::Display for LogEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] [{}] [{}] {}",
            self.timestamp.format("%Y-%m-%d %H:%M:%S"),
            self.level,
            self.category,
            self.message
        )
    }
}

struct LoggerInner {
    entries: Vec<LogEntry>,
    log_file: Option<File>,
    mode: LogMode,
}

static LOGGER: Lazy<Arc<RwLock<LoggerInner>>> = Lazy::new(|| {
    Arc::new(RwLock::new(LoggerInner {
        entries: Vec::new(),
        log_file: None,
        mode: LogMode::Full,
    }))
});

pub fn init_logger(file_path: &str, mode: &str) {
    let log_mode = LogMode::from_str(mode);

    let mut logger = LOGGER.write().unwrap();
    logger.mode = log_mode;

    if log_mode == LogMode::Disabled {
        return;
    }

    utils::assert_parent_dir_exists(file_path)
        .expect("Failed to create parent directories for log file");

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .expect("Failed to open log file");

    logger.log_file = Some(file);
}

fn log(level: LogLevel, category: LogCategory, message: &str) {
    let entry = LogEntry {
        timestamp: Utc::now().naive_utc(),
        level,
        category,
        message: message.to_string(),
    };

    let mut logger = LOGGER.write().unwrap();

    if logger.mode == LogMode::Disabled {
        return;
    }

    let formatted = entry.to_string();

    // Print to stdout/stderr only in Full mode
    if logger.mode == LogMode::Full {
        match level {
            LogLevel::Error => eprintln!("{}", formatted),
            _ => println!("{}", formatted),
        }
    }

    // Write to file if initialized
    if let Some(ref mut file) = logger.log_file {
        let _ = writeln!(file, "{}", formatted);
    }

    // Store in memory
    logger.entries.push(entry);
}

pub fn log_info(category: LogCategory, message: &str) {
    log(LogLevel::Info, category, message);
}

pub fn log_warning(category: LogCategory, message: &str) {
    log(LogLevel::Warning, category, message);
}

pub fn log_error(category: LogCategory, message: &str) {
    log(LogLevel::Error, category, message);
}

pub fn get_logs(
    category: Option<LogCategory>,
    level: Option<LogLevel>,
    limit: Option<usize>,
) -> Vec<LogEntry> {
    let logger = LOGGER.read().unwrap();

    let mut filtered: Vec<LogEntry> = logger
        .entries
        .iter()
        .filter(|e| category.map_or(true, |c| e.category == c))
        .filter(|e| level.map_or(true, |l| e.level == l))
        .cloned()
        .collect();

    filtered.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    match limit {
        Some(n) => filtered.into_iter().rev().take(n).rev().collect(),
        None => filtered,
    }
}
