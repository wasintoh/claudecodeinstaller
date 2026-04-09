use chrono::Local;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Log level for entries
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
}

/// A single log entry with timestamp
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
}

/// Thread-safe logger that accumulates entries in memory and can export to file
#[derive(Debug, Clone)]
pub struct AppLogger {
    entries: Arc<Mutex<Vec<LogEntry>>>,
}

impl AppLogger {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Add a log entry
    pub fn log(&self, level: LogLevel, message: impl Into<String>) {
        let entry = LogEntry {
            timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            level,
            message: message.into(),
        };
        if let Ok(mut entries) = self.entries.lock() {
            entries.push(entry);
        }
    }

    /// Convenience methods
    pub fn info(&self, message: impl Into<String>) {
        self.log(LogLevel::Info, message);
    }

    pub fn warn(&self, message: impl Into<String>) {
        self.log(LogLevel::Warn, message);
    }

    pub fn error(&self, message: impl Into<String>) {
        self.log(LogLevel::Error, message);
    }

    /// Get all log entries
    pub fn get_entries(&self) -> Vec<LogEntry> {
        self.entries.lock().map(|e| e.clone()).unwrap_or_default()
    }

    /// Export all log entries to a file, returns the file path
    pub fn export_to_file(&self) -> Result<PathBuf, String> {
        let entries = self.get_entries();
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("claude-installer-log-{}.txt", timestamp);

        // Write to user's Desktop or temp dir
        let desktop = dirs_next_desktop().unwrap_or_else(std::env::temp_dir);
        let path = desktop.join(&filename);

        let mut content = String::new();
        content.push_str("=== Claude Code Installer Log ===\n");
        content.push_str(&format!("Exported: {}\n", Local::now().format("%Y-%m-%d %H:%M:%S")));
        content.push_str("=================================\n\n");

        for entry in &entries {
            let level_str = match entry.level {
                LogLevel::Info => "INFO ",
                LogLevel::Warn => "WARN ",
                LogLevel::Error => "ERROR",
                LogLevel::Debug => "DEBUG",
            };
            content.push_str(&format!("[{}] {} {}\n", entry.timestamp, level_str, entry.message));
        }

        fs::write(&path, &content).map_err(|e| format!("Failed to write log file: {}", e))?;
        Ok(path)
    }

    /// Clear all entries
    pub fn clear(&self) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.clear();
        }
    }
}

/// Get the user's Desktop path
fn dirs_next_desktop() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE")
            .ok()
            .map(|p| PathBuf::from(p).join("Desktop"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME")
            .ok()
            .map(|p| PathBuf::from(p).join("Desktop"))
    }
}
