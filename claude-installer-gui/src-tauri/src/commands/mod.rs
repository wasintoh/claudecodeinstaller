pub mod claude_installer;
pub mod git_installer;
pub mod node_installer;
pub mod path_manager;
pub mod system_check;
pub mod uninstaller;

use serde::Serialize;

/// Status of a single check item in the system check
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckItem {
    /// Unique key for this check (e.g., "git", "node", "windows")
    pub key: String,
    /// Human-readable label
    pub label: String,
    /// "pass", "fail", "warn", "checking", "skipped"
    pub status: String,
    /// Detail text (e.g., version number or error message)
    pub detail: String,
    /// Detected version if applicable
    pub version: Option<String>,
}

/// Events emitted during installation for real-time progress
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum InstallEvent {
    /// A new installation step has started
    StepStarted {
        step: String,
        total_steps: u32,
        current_step: u32,
    },
    /// Download progress update
    DownloadProgress {
        step: String,
        downloaded: u64,
        total: u64,
        speed_bps: u64,
        eta_secs: u64,
    },
    /// A log message for the current step
    StepLog {
        step: String,
        level: String,
        message: String,
    },
    /// Retry attempt
    RetryAttempt {
        step: String,
        attempt: u32,
        max_attempts: u32,
        error: String,
    },
    /// A step has completed
    StepCompleted {
        step: String,
        success: bool,
        version: Option<String>,
        error: Option<String>,
    },
    /// Overall progress percentage
    OverallProgress {
        percent: f32,
        message: String,
    },
}

/// Result of the system check
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemCheckResult {
    pub items: Vec<CheckItem>,
    /// Number of components that need to be installed
    pub install_count: u32,
    /// Approximate total download size in MB
    pub approx_download_mb: u32,
}

/// Information about an installed component (for uninstall screen)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledComponent {
    pub key: String,
    pub label: String,
    pub version: Option<String>,
    pub installed: bool,
    /// Warning text if removing this could affect other software
    pub warning: Option<String>,
}
