use crate::commands::InstallEvent;
use crate::utils::logger::AppLogger;
use crate::utils::process;
use std::sync::{Arc, Mutex};
use tauri::{ipc::Channel, State};

/// Install Claude Code using the official install script.
/// Requires Node.js/npm to be already installed.
#[tauri::command]
pub async fn install_claude(
    on_event: Channel<InstallEvent>,
    logger: State<'_, Arc<Mutex<AppLogger>>>,
) -> Result<(), String> {
    let logger_arc = logger.inner().clone();
    {
        let log = logger_arc.lock().map_err(|e| e.to_string())?;
        log.info("Starting Claude Code installation...");
    }

    // Step 1: Verify npm is available
    on_event
        .send(InstallEvent::StepLog {
            step: "claude".into(),
            level: "info".into(),
            message: "Checking npm availability...".into(),
        })
        .map_err(|e| e.to_string())?;

    verify_npm_available()?;

    // Step 2: Install Claude Code via npm
    on_event
        .send(InstallEvent::StepLog {
            step: "claude".into(),
            level: "info".into(),
            message: "Installing Claude Code via npm (this may take a few minutes)...".into(),
        })
        .map_err(|e| e.to_string())?;

    run_claude_install(&on_event)?;

    // Step 3: Ensure Claude is in PATH
    on_event
        .send(InstallEvent::StepLog {
            step: "claude".into(),
            level: "info".into(),
            message: "Configuring PATH...".into(),
        })
        .map_err(|e| e.to_string())?;

    ensure_claude_in_path()?;

    // Step 4: Verify installation
    on_event
        .send(InstallEvent::StepLog {
            step: "claude".into(),
            level: "info".into(),
            message: "Verifying Claude Code installation...".into(),
        })
        .map_err(|e| e.to_string())?;

    let version = verify_claude_installed()?;

    {
        let log = logger_arc.lock().map_err(|e| e.to_string())?;
        log.info(format!("Claude Code installed successfully: {}", version));
    }

    on_event
        .send(InstallEvent::StepCompleted {
            step: "claude".into(),
            success: true,
            version: Some(version),
            error: None,
        })
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Verify that npm is available on the system
fn verify_npm_available() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let npm_locations = [
            "npm",
            "npm.cmd",
            "C:\\Program Files\\nodejs\\npm.cmd",
        ];

        for npm in &npm_locations {
            if let Ok(result) = process::run_command(npm, &["--version"]) {
                if result.success {
                    return Ok(());
                }
            }
        }

        Err("npm is not available. Please install Node.js first.".to_string())
    }

    #[cfg(not(target_os = "windows"))]
    {
        // On dev machine, check if npm exists
        if process::get_version("npm", "--version").is_some() {
            Ok(())
        } else {
            Ok(()) // Allow dev mode to continue
        }
    }
}

/// Run the Claude Code installation
fn run_claude_install(on_event: &Channel<InstallEvent>) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        // Use the official install script via PowerShell
        let result = process::run_powershell(
            "irm https://claude.ai/install.ps1 | iex"
        );

        match result {
            Ok(r) if r.success => {
                let _ = on_event.send(InstallEvent::StepLog {
                    step: "claude".into(),
                    level: "info".into(),
                    message: "Claude Code install script completed.".into(),
                });
                Ok(())
            }
            Ok(r) => {
                // Fallback: try npm install
                let _ = on_event.send(InstallEvent::StepLog {
                    step: "claude".into(),
                    level: "warn".into(),
                    message: format!(
                        "Install script returned error, trying npm fallback: {}",
                        r.stderr
                    ),
                });
                install_via_npm(on_event)
            }
            Err(e) => {
                // Fallback: try npm install
                let _ = on_event.send(InstallEvent::StepLog {
                    step: "claude".into(),
                    level: "warn".into(),
                    message: format!(
                        "Install script failed, trying npm fallback: {}",
                        e
                    ),
                });
                install_via_npm(on_event)
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = on_event.send(InstallEvent::StepLog {
            step: "claude".into(),
            level: "info".into(),
            message: "[Dev Mode] Would run Claude install script".into(),
        });
        Ok(())
    }
}

/// Fallback: install Claude Code via npm global install
fn install_via_npm(on_event: &Channel<InstallEvent>) -> Result<(), String> {
    let _ = on_event.send(InstallEvent::StepLog {
        step: "claude".into(),
        level: "info".into(),
        message: "Installing via npm install -g @anthropic-ai/claude-code...".into(),
    });

    #[cfg(target_os = "windows")]
    {
        let npm_locations = [
            "npm",
            "npm.cmd",
            "C:\\Program Files\\nodejs\\npm.cmd",
        ];

        for npm in &npm_locations {
            let result = process::run_command(
                npm,
                &["install", "-g", "@anthropic-ai/claude-code"],
            );

            if let Ok(r) = result {
                if r.success {
                    return Ok(());
                }
            }
        }

        Err("Failed to install Claude Code via npm. Please check your internet connection and try again.".to_string())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(())
    }
}

/// Ensure the Claude Code binary location is in the user's PATH
fn ensure_claude_in_path() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use crate::commands::path_manager;

        let home = std::env::var("USERPROFILE")
            .map_err(|_| "USERPROFILE not set".to_string())?;

        // Claude Code is typically installed to ~/.local/bin or AppData/Roaming/npm
        let claude_paths = vec![
            format!("{}\\.local\\bin", home),
            format!("{}\\AppData\\Roaming\\npm", home),
        ];

        for path in claude_paths {
            if std::path::Path::new(&path).exists() {
                let _ = path_manager::add_to_path(&path);
            }
        }

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(())
    }
}

/// Verify Claude Code is installed and return version string
fn verify_claude_installed() -> Result<String, String> {
    // Try PATH first
    if let Some(version) = process::get_version("claude", "--version") {
        return Ok(version);
    }

    #[cfg(target_os = "windows")]
    {
        let home = std::env::var("USERPROFILE").unwrap_or_default();
        let locations = [
            format!("{}\\.local\\bin\\claude.exe", home),
            format!("{}\\AppData\\Roaming\\npm\\claude.cmd", home),
        ];
        for loc in &locations {
            if let Some(version) = process::get_version(loc, "--version") {
                return Ok(version);
            }
        }
        return Err("Claude Code installation could not be verified. Try restarting your terminal.".to_string());
    }

    #[cfg(not(target_os = "windows"))]
    Ok("claude v1.0.0 [dev mode]".to_string())
}
