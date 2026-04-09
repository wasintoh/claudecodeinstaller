use crate::commands::{InstallEvent, InstalledComponent};
use crate::utils::logger::AppLogger;
use crate::utils::process;
use std::sync::{Arc, Mutex};
use tauri::{ipc::Channel, State};

/// Detect which components are currently installed on the system.
/// Used to populate the uninstall screen.
#[tauri::command]
pub fn detect_installed(
    logger: State<'_, Arc<Mutex<AppLogger>>>,
) -> Result<Vec<InstalledComponent>, String> {
    {
        let log = logger.lock().map_err(|e| e.to_string())?;
        log.info("Detecting installed components...");
    }

    let mut components = Vec::new();

    // Claude Code
    let claude_version = process::get_version("claude", "--version");
    components.push(InstalledComponent {
        key: "claude".into(),
        label: "Claude Code".into(),
        version: claude_version.clone(),
        installed: claude_version.is_some(),
        warning: None,
    });

    // Node.js
    let node_version = process::get_version("node", "--version");
    components.push(InstalledComponent {
        key: "node".into(),
        label: "Node.js + npm".into(),
        version: node_version.clone(),
        installed: node_version.is_some(),
        warning: None,
    });

    // Git
    let git_version = process::get_version("git", "--version");
    components.push(InstalledComponent {
        key: "git".into(),
        label: "Git for Windows".into(),
        version: git_version.clone(),
        installed: git_version.is_some(),
        warning: Some(
            "Git may be used by other programs (VS Code, GitHub Desktop, etc.)".into(),
        ),
    });

    Ok(components)
}

/// Uninstall selected components.
/// `components` is a list of keys: "claude", "node", "git"
/// `options` contains flags like "include_settings", "include_npm_cache"
#[tauri::command]
pub async fn uninstall_components(
    components: Vec<String>,
    include_settings: bool,
    include_npm_cache: bool,
    on_event: Channel<InstallEvent>,
    logger: State<'_, Arc<Mutex<AppLogger>>>,
) -> Result<(), String> {
    let logger_arc = logger.inner().clone();
    {
        let log = logger_arc.lock().map_err(|e| e.to_string())?;
        log.info(format!("Uninstalling components: {:?}", components));
    }

    let total = components.len() as u32;

    for (i, component) in components.iter().enumerate() {
        on_event
            .send(InstallEvent::StepStarted {
                step: component.clone(),
                total_steps: total,
                current_step: (i + 1) as u32,
            })
            .map_err(|e| e.to_string())?;

        let result = match component.as_str() {
            "claude" => uninstall_claude(include_settings, &on_event).await,
            "node" => uninstall_node(include_npm_cache, &on_event).await,
            "git" => uninstall_git(&on_event).await,
            _ => Err(format!("Unknown component: {}", component)),
        };

        let (success, error) = match result {
            Ok(()) => (true, None),
            Err(e) => {
                let log = logger_arc.lock().map_err(|e2| e2.to_string())?;
                log.error(format!("Failed to uninstall {}: {}", component, e));
                (false, Some(e))
            }
        };

        on_event
            .send(InstallEvent::StepCompleted {
                step: component.clone(),
                success,
                version: None,
                error,
            })
            .map_err(|e| e.to_string())?;
    }

    // Broadcast environment change after all uninstalls
    #[cfg(target_os = "windows")]
    {
        crate::commands::path_manager::broadcast_environment_change();
    }

    Ok(())
}

/// Uninstall Claude Code
#[allow(unused_variables)]
async fn uninstall_claude(
    include_settings: bool,
    on_event: &Channel<InstallEvent>,
) -> Result<(), String> {
    let _ = on_event.send(InstallEvent::StepLog {
        step: "claude".into(),
        level: "info".into(),
        message: "Removing Claude Code...".into(),
    });

    #[cfg(target_os = "windows")]
    {
        let home = std::env::var("USERPROFILE").unwrap_or_default();

        // Try npm uninstall first
        let _ = process::run_command("npm", &["uninstall", "-g", "@anthropic-ai/claude-code"]);

        // Remove binary from ~/.local/bin
        let claude_bin = format!("{}\\.local\\bin\\claude.exe", home);
        let _ = std::fs::remove_file(&claude_bin);

        // Remove version data
        let claude_data = format!("{}\\.local\\share\\claude", home);
        let _ = std::fs::remove_dir_all(&claude_data);

        // Try winget as fallback
        let _ = process::run_command("winget", &["uninstall", "--id", "Anthropic.Claude", "--silent"]);

        // Optionally remove settings
        if include_settings {
            let _ = on_event.send(InstallEvent::StepLog {
                step: "claude".into(),
                level: "info".into(),
                message: "Removing Claude Code settings and session data...".into(),
            });

            let claude_config = format!("{}\\.claude", home);
            let _ = std::fs::remove_dir_all(&claude_config);

            let claude_json = format!("{}\\.claude.json", home);
            let _ = std::fs::remove_file(&claude_json);
        }

        // Clean up PATH
        let local_bin = format!("{}\\.local\\bin", home);
        let _ = crate::commands::path_manager::remove_from_path(&local_bin);
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = on_event.send(InstallEvent::StepLog {
            step: "claude".into(),
            level: "info".into(),
            message: "[Dev Mode] Would remove Claude Code".into(),
        });
    }

    Ok(())
}

/// Uninstall Node.js
#[allow(unused_variables)]
async fn uninstall_node(
    include_npm_cache: bool,
    on_event: &Channel<InstallEvent>,
) -> Result<(), String> {
    let _ = on_event.send(InstallEvent::StepLog {
        step: "node".into(),
        level: "info".into(),
        message: "Removing Node.js...".into(),
    });

    #[cfg(target_os = "windows")]
    {
        let home = std::env::var("USERPROFILE").unwrap_or_default();

        // Try to find the MSI product code from registry
        let uninstalled = try_msi_uninstall_node();

        if !uninstalled {
            // Fallback: try winget
            let _ = process::run_command(
                "winget",
                &["uninstall", "--id", "OpenJS.NodeJS.LTS", "--silent"],
            );
        }

        if include_npm_cache {
            let _ = on_event.send(InstallEvent::StepLog {
                step: "node".into(),
                level: "info".into(),
                message: "Removing npm global packages and cache...".into(),
            });

            // Remove npm directories
            let npm_dirs = [
                format!("{}\\AppData\\Roaming\\npm", home),
                format!("{}\\AppData\\Roaming\\npm-cache", home),
                format!("{}\\.npm-global", home),
            ];

            for dir in &npm_dirs {
                let _ = std::fs::remove_dir_all(dir);
            }
        }

        // Clean PATH
        let _ = crate::commands::path_manager::remove_from_path("C:\\Program Files\\nodejs");
        let npm_path = format!("{}\\AppData\\Roaming\\npm", home);
        let _ = crate::commands::path_manager::remove_from_path(&npm_path);
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = on_event.send(InstallEvent::StepLog {
            step: "node".into(),
            level: "info".into(),
            message: "[Dev Mode] Would remove Node.js".into(),
        });
    }

    Ok(())
}

/// Try to find Node.js MSI product code and uninstall via msiexec
#[cfg(target_os = "windows")]
fn try_msi_uninstall_node() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let uninstall_key = match hklm.open_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall") {
        Ok(key) => key,
        Err(_) => return false,
    };

    // Iterate through subkeys to find Node.js
    for name in uninstall_key.enum_keys().filter_map(|k| k.ok()) {
        if let Ok(subkey) = uninstall_key.open_subkey(&name) {
            let display_name: String = subkey.get_value("DisplayName").unwrap_or_default();
            if display_name.contains("Node.js") || display_name.contains("Node") {
                // Found Node.js, try to uninstall using the product code
                if let Ok(result) = process::run_msi_uninstall(&format!("{{{}}}", name)) {
                    return result.success;
                }
            }
        }
    }

    false
}

/// Uninstall Git for Windows
async fn uninstall_git(on_event: &Channel<InstallEvent>) -> Result<(), String> {
    let _ = on_event.send(InstallEvent::StepLog {
        step: "git".into(),
        level: "info".into(),
        message: "Removing Git for Windows...".into(),
    });

    #[cfg(target_os = "windows")]
    {
        // Try the built-in uninstaller
        let uninstaller_paths = [
            "C:\\Program Files\\Git\\unins000.exe",
            "C:\\Program Files (x86)\\Git\\unins000.exe",
        ];

        let mut uninstalled = false;
        for uninstaller in &uninstaller_paths {
            if std::path::Path::new(uninstaller).exists() {
                let result = process::run_command(uninstaller, &["/VERYSILENT"]);
                if let Ok(r) = result {
                    if r.success {
                        uninstalled = true;
                        break;
                    }
                }
            }
        }

        if !uninstalled {
            // Fallback: try winget
            let _ = process::run_command(
                "winget",
                &["uninstall", "--id", "Git.Git", "--silent"],
            );
        }

        // Clean PATH
        let _ = crate::commands::path_manager::remove_from_path("C:\\Program Files\\Git\\cmd");
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = on_event.send(InstallEvent::StepLog {
            step: "git".into(),
            level: "info".into(),
            message: "[Dev Mode] Would remove Git".into(),
        });
    }

    Ok(())
}
