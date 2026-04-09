// Post-install test, auto-repair, and launch commands.
//
// This module mirrors the PowerShell script's Phase 5 and Phase 6:
//   - Test `claude --version`
//   - Classify any failure (COMMAND_NOT_FOUND, GIT_BASH_MISSING, BLOCKED, EXEC_FAILED)
//   - Auto-repair based on error kind (up to 3 attempts)
//   - Launch Claude Code in a new PowerShell window
//
// See claude-installer-ps/install-claude-code.ps1 lines 1062-1342 for the reference impl.

use crate::commands::InstallEvent;
use crate::utils::logger::AppLogger;
use crate::utils::process;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::{ipc::Channel, State};

/// Result of testing the Claude Code runtime.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    pub success: bool,
    pub version: Option<String>,
    /// One of: "ok", "commandNotFound", "gitBashMissing", "blocked", "execFailed"
    pub error_kind: String,
    pub binary_path: Option<String>,
    pub raw_output: String,
    /// Repair attempts made before giving up (0 if first attempt succeeded)
    pub repair_attempts: u32,
}

/// Test that `claude --version` works and auto-repair common failures.
///
/// This is a single command that does test + repair + re-test, up to 3 repair
/// attempts. It emits `InstallEvent::StepLog` messages as it goes so the UI
/// can show progress.
#[tauri::command]
pub async fn test_claude_code(
    on_event: Channel<InstallEvent>,
    logger: State<'_, Arc<Mutex<AppLogger>>>,
) -> Result<TestResult, String> {
    let logger_arc = logger.inner().clone();
    {
        let log = logger_arc.lock().map_err(|e| e.to_string())?;
        log.info("Phase 5: Testing Claude Code runtime...");
    }

    // StepStarted event for the testing phase
    let _ = on_event.send(InstallEvent::StepStarted {
        step: "test".into(),
        total_steps: 1,
        current_step: 1,
    });

    let mut result = run_test(&on_event);

    // Auto-repair up to 3 times
    let max_attempts = 3u32;
    let mut attempts = 0u32;

    while !result.success && attempts < max_attempts {
        attempts += 1;

        // Don't retry for unknown/exec_failed errors
        if result.error_kind == "execFailed" || result.error_kind == "unknown" {
            let _ = on_event.send(InstallEvent::StepLog {
                step: "test".into(),
                level: "error".into(),
                message: format!(
                    "Cannot auto-repair error type: {}. Raw: {}",
                    result.error_kind, result.raw_output
                ),
            });
            break;
        }

        {
            let log = logger_arc.lock().map_err(|e| e.to_string())?;
            log.warn(format!(
                "Auto-repair attempt {}/{} for error: {}",
                attempts, max_attempts, result.error_kind
            ));
        }

        let _ = on_event.send(InstallEvent::RetryAttempt {
            step: "test".into(),
            attempt: attempts,
            max_attempts,
            error: result.error_kind.clone(),
        });

        match result.error_kind.as_str() {
            "commandNotFound" => repair_command_not_found(&on_event)?,
            "gitBashMissing" => repair_git_bash_missing(&on_event)?,
            "blocked" => repair_blocked(&on_event, &result.binary_path)?,
            _ => break,
        }

        // Re-test
        result = run_test(&on_event);
    }

    result.repair_attempts = attempts;

    {
        let log = logger_arc.lock().map_err(|e| e.to_string())?;
        if result.success {
            log.info(format!(
                "Claude Code runtime test PASSED: v{} (after {} repair attempts)",
                result.version.clone().unwrap_or_default(),
                attempts
            ));
        } else {
            log.error(format!(
                "Claude Code runtime test FAILED after {} repair attempts. Error: {}",
                attempts, result.error_kind
            ));
        }
    }

    // StepCompleted event
    let _ = on_event.send(InstallEvent::StepCompleted {
        step: "test".into(),
        success: result.success,
        version: result.version.clone(),
        error: if result.success {
            None
        } else {
            Some(result.error_kind.clone())
        },
    });

    Ok(result)
}

/// Run `claude --version` once and classify the result.
fn run_test(on_event: &Channel<InstallEvent>) -> TestResult {
    let _ = on_event.send(InstallEvent::StepLog {
        step: "test".into(),
        level: "info".into(),
        message: "Running 'claude --version'...".into(),
    });

    // Step 1: Locate the binary
    let binary_path = locate_claude_binary();

    #[cfg(target_os = "windows")]
    {
        if binary_path.is_none() {
            return TestResult {
                success: false,
                version: None,
                error_kind: "commandNotFound".into(),
                binary_path: None,
                raw_output: "claude binary not found in PATH or candidate locations".into(),
                repair_attempts: 0,
            };
        }

        let path = binary_path.unwrap();

        // Step 2: Try to run it
        match process::run_command(&path, &["--version"]) {
            Ok(proc_result) => {
                let combined = format!("{}\n{}", proc_result.stdout, proc_result.stderr)
                    .trim()
                    .to_string();

                if proc_result.success {
                    // Extract version number
                    let version = extract_version(&combined);
                    return TestResult {
                        success: true,
                        version: Some(version),
                        error_kind: "ok".into(),
                        binary_path: Some(path),
                        raw_output: combined,
                        repair_attempts: 0,
                    };
                }

                // Failed — classify by output
                let error_kind = classify_error(&combined);
                TestResult {
                    success: false,
                    version: None,
                    error_kind,
                    binary_path: Some(path),
                    raw_output: combined,
                    repair_attempts: 0,
                }
            }
            Err(e) => {
                // Command failed to execute at all
                let error_kind = classify_error(&e);
                TestResult {
                    success: false,
                    version: None,
                    error_kind,
                    binary_path: Some(path),
                    raw_output: e,
                    repair_attempts: 0,
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = binary_path;
        // Dev mode on macOS — pretend success
        let _ = on_event.send(InstallEvent::StepLog {
            step: "test".into(),
            level: "info".into(),
            message: "[Dev Mode] Skipping real test on non-Windows".into(),
        });
        TestResult {
            success: true,
            version: Some("1.0.0-dev".into()),
            error_kind: "ok".into(),
            binary_path: None,
            raw_output: "[dev mode] test skipped".into(),
            repair_attempts: 0,
        }
    }
}

/// Find a claude binary on disk. Returns path if found, None otherwise.
fn locate_claude_binary() -> Option<String> {
    // First, try PATH resolution via the OS
    if process::get_version("claude", "--version").is_some() {
        return Some("claude".into());
    }

    #[cfg(target_os = "windows")]
    {
        let home = std::env::var("USERPROFILE").unwrap_or_default();
        let app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();

        let candidates = vec![
            format!("{}\\.local\\bin\\claude.exe", home),
            format!("{}\\.local\\bin\\claude.cmd", home),
            format!("{}\\.local\\bin\\claude", home),
            format!("{}\\AppData\\Roaming\\npm\\claude.cmd", home),
            format!("{}\\Programs\\claude\\claude.exe", app_data),
        ];

        for candidate in candidates {
            if std::path::Path::new(&candidate).exists() {
                return Some(candidate);
            }
        }
    }

    None
}

/// Classify an error message/output into one of our known error kinds.
fn classify_error(text: &str) -> String {
    let lower = text.to_lowercase();

    if lower.contains("requires git-bash") || lower.contains("git-bash") {
        return "gitBashMissing".into();
    }
    if lower.contains("not recognized")
        || lower.contains("commandnotfound")
        || lower.contains("cannot find")
    {
        return "commandNotFound".into();
    }
    if lower.contains("access")
        && (lower.contains("denied") || lower.contains("unauthorized"))
    {
        return "blocked".into();
    }
    if lower.contains("win32exception") || lower.contains("blocked") || lower.contains("virus")
    {
        return "blocked".into();
    }

    // Empty output but non-success often means SmartScreen or silent block
    if text.trim().is_empty() {
        return "blocked".into();
    }

    "execFailed".into()
}

/// Extract a semver-like version string from output text.
fn extract_version(text: &str) -> String {
    for line in text.lines() {
        for word in line.split_whitespace() {
            let trimmed = word.trim_start_matches('v');
            let parts: Vec<&str> = trimmed.split('.').collect();
            if parts.len() >= 2 && parts.iter().all(|p| p.chars().next().map_or(false, |c| c.is_ascii_digit())) {
                return trimmed.to_string();
            }
        }
    }
    text.trim().to_string()
}

// ─── Repair functions ─────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn repair_command_not_found(on_event: &Channel<InstallEvent>) -> Result<(), String> {
    use crate::commands::path_manager;

    let _ = on_event.send(InstallEvent::StepLog {
        step: "test".into(),
        level: "info".into(),
        message: "Repair: adding ~/.local/bin to PATH...".into(),
    });

    let home = std::env::var("USERPROFILE").map_err(|e| e.to_string())?;
    let local_bin = format!("{}\\.local\\bin", home);

    if std::path::Path::new(&local_bin).exists() {
        let _ = path_manager::add_to_path(&local_bin);
        path_manager::broadcast_environment_change();
        return Ok(());
    }

    // Binary dir doesn't exist — re-run the bootstrap
    let _ = on_event.send(InstallEvent::StepLog {
        step: "test".into(),
        level: "warn".into(),
        message: "Binary directory missing — re-running Claude Code bootstrap...".into(),
    });

    let _ = process::run_powershell("irm https://claude.ai/install.ps1 | iex");

    if std::path::Path::new(&local_bin).exists() {
        let _ = path_manager::add_to_path(&local_bin);
        path_manager::broadcast_environment_change();
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn repair_command_not_found(_on_event: &Channel<InstallEvent>) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn repair_git_bash_missing(on_event: &Channel<InstallEvent>) -> Result<(), String> {
    let _ = on_event.send(InstallEvent::StepLog {
        step: "test".into(),
        level: "info".into(),
        message: "Repair: locating bash.exe and setting CLAUDE_CODE_GIT_BASH_PATH...".into(),
    });

    let candidates = vec![
        format!(
            "{}\\Programs\\Git\\bin\\bash.exe",
            std::env::var("LOCALAPPDATA").unwrap_or_default()
        ),
        format!(
            "{}\\Git\\bin\\bash.exe",
            std::env::var("ProgramFiles").unwrap_or_default()
        ),
        format!(
            "{}\\Git\\bin\\bash.exe",
            std::env::var("ProgramFiles(x86)").unwrap_or_default()
        ),
    ];

    for candidate in candidates {
        if std::path::Path::new(&candidate).exists() {
            // Set user env var via PowerShell
            let script = format!(
                "[Environment]::SetEnvironmentVariable('CLAUDE_CODE_GIT_BASH_PATH', '{}', 'User')",
                candidate.replace('\\', "\\\\")
            );
            let _ = process::run_powershell(&script);

            let _ = on_event.send(InstallEvent::StepLog {
                step: "test".into(),
                level: "info".into(),
                message: format!("Set CLAUDE_CODE_GIT_BASH_PATH={}", candidate),
            });
            return Ok(());
        }
    }

    let _ = on_event.send(InstallEvent::StepLog {
        step: "test".into(),
        level: "warn".into(),
        message: "bash.exe not found — Git may not be installed. Cannot auto-repair.".into(),
    });
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn repair_git_bash_missing(_on_event: &Channel<InstallEvent>) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn repair_blocked(
    on_event: &Channel<InstallEvent>,
    binary_path: &Option<String>,
) -> Result<(), String> {
    let _ = on_event.send(InstallEvent::StepLog {
        step: "test".into(),
        level: "info".into(),
        message: "Repair: unblocking Claude Code files (SmartScreen)...".into(),
    });

    // Unblock the specific binary
    if let Some(path) = binary_path {
        let script = format!("Unblock-File -Path '{}' -ErrorAction SilentlyContinue", path);
        let _ = process::run_powershell(&script);
    }

    // Unblock everything under ~/.local/bin and ~/.local/share/claude
    let home = std::env::var("USERPROFILE").unwrap_or_default();
    let unblock_script = format!(
        "Get-ChildItem -Path '{}\\.local\\bin' -File -ErrorAction SilentlyContinue | ForEach-Object {{ Unblock-File -Path $_.FullName -ErrorAction SilentlyContinue }}; Get-ChildItem -Path '{}\\.local\\share\\claude' -File -Recurse -ErrorAction SilentlyContinue | ForEach-Object {{ Unblock-File -Path $_.FullName -ErrorAction SilentlyContinue }}",
        home, home
    );
    let _ = process::run_powershell(&unblock_script);

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn repair_blocked(
    _on_event: &Channel<InstallEvent>,
    _binary_path: &Option<String>,
) -> Result<(), String> {
    Ok(())
}

// ─── Launch Command ───────────────────────────────────────────────────

/// Launch Claude Code in a new PowerShell window.
///
/// Writes a wrapper script to %TEMP%, then spawns `powershell.exe -NoExit -File <wrapper>`.
/// The wrapper sets the window title, prints a friendly banner, and runs `claude` inside a
/// try/catch so errors stay visible.
#[tauri::command]
pub fn launch_claude_in_new_terminal(
    logger: State<'_, Arc<Mutex<AppLogger>>>,
) -> Result<(), String> {
    {
        let log = logger.lock().map_err(|e| e.to_string())?;
        log.info("Phase 6: Launching Claude Code in a new terminal window...");
    }

    #[cfg(target_os = "windows")]
    {
        use std::io::Write;

        let wrapper_script = r#"
$host.UI.RawUI.WindowTitle = 'Claude Code'
Write-Host ''
Write-Host '  ============================================================' -ForegroundColor Cyan
Write-Host '    Claude Code is ready!' -ForegroundColor Green
Write-Host '    Starting now... (you can close this window anytime)' -ForegroundColor Cyan
Write-Host '  ============================================================' -ForegroundColor Cyan
Write-Host ''
Start-Sleep -Seconds 1
try {
    claude
}
catch {
    Write-Host ''
    Write-Host '  [ERROR] Failed to start Claude Code:' -ForegroundColor Red
    Write-Host "  $($_.Exception.Message)" -ForegroundColor Yellow
    Write-Host ''
    Write-Host '  Try running this command manually:  claude' -ForegroundColor White
    Write-Host ''
}
"#;

        let temp_dir = std::env::var("TEMP").unwrap_or_else(|_| "C:\\Temp".into());
        let wrapper_path = format!("{}\\claude-installer-launch.ps1", temp_dir);

        let mut file = std::fs::File::create(&wrapper_path)
            .map_err(|e| format!("Failed to create launch wrapper: {}", e))?;
        file.write_all(wrapper_script.as_bytes())
            .map_err(|e| format!("Failed to write launch wrapper: {}", e))?;

        let home = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".into());

        std::process::Command::new("powershell.exe")
            .args(&[
                "-NoExit",
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                &wrapper_path,
            ])
            .current_dir(&home)
            .spawn()
            .map_err(|e| format!("Failed to launch Claude Code: {}", e))?;

        {
            let log = logger.lock().map_err(|e| e.to_string())?;
            log.info("Launched Claude Code in new PowerShell window");
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let log = logger.lock().map_err(|e| e.to_string())?;
        log.info("[Dev Mode] Would launch Claude Code in new terminal");
    }

    Ok(())
}
