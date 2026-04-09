use crate::commands::{CheckItem, SystemCheckResult};
use crate::utils::logger::AppLogger;
use crate::utils::process;
use std::sync::{Arc, Mutex};
use tauri::State;

/// Runs all system checks and returns results.
/// Each check is independent and failures in one don't prevent others from running.
#[tauri::command]
pub async fn system_check(
    logger: State<'_, Arc<Mutex<AppLogger>>>,
) -> Result<SystemCheckResult, String> {
    // Clone the logger Arc so we don't hold the State across awaits
    let logger_arc = logger.inner().clone();

    {
        let log = logger_arc.lock().map_err(|e| e.to_string())?;
        log.info("Starting system check...");
    }

    let mut items = Vec::new();
    let mut install_count = 0u32;
    let mut approx_mb = 0u32;

    // 1. Windows Version
    items.push(check_windows_version());

    // 2. Internet Connection
    items.push(check_internet().await);

    // 3. Disk Space
    items.push(check_disk_space());

    // 4. RAM
    items.push(check_ram());

    // 5. Git
    let git = check_git();
    if git.status == "fail" {
        install_count += 1;
        approx_mb += 55;
    }
    items.push(git);

    // 6. Node.js
    let node = check_node();
    if node.status == "fail" || node.status == "warn" {
        install_count += 1;
        approx_mb += 30;
    }
    items.push(node);

    // 7. Claude Code
    let claude = check_claude();
    if claude.status == "fail" {
        install_count += 1;
        approx_mb += 20;
    }
    items.push(claude);

    {
        let log = logger_arc.lock().map_err(|e| e.to_string())?;
        log.info(format!(
            "System check complete: {} items to install (~{}MB)",
            install_count, approx_mb
        ));
    }

    Ok(SystemCheckResult {
        items,
        install_count,
        approx_download_mb: approx_mb,
    })
}

fn check_windows_version() -> CheckItem {

    #[cfg(target_os = "windows")]
    {
        match process::run_command("cmd", &["/c", "ver"]) {
            Ok(result) if result.success => {
                let version = result.stdout.trim().to_string();
                // Check for Windows 10+ (version 10.x)
                let is_win10_plus = version.contains("10.0") || version.contains("11.");
                CheckItem {
                    key: "windows".into(),
                    label: "Windows Version".into(),
                    status: if is_win10_plus { "pass" } else { "warn" }.into(),
                    detail: version.clone(),
                    version: Some(version),
                }
            }
            Ok(result) => CheckItem {
                key: "windows".into(),
                label: "Windows Version".into(),
                status: "warn".into(),
                detail: format!("Could not determine version: {}", result.stderr),
                version: None,
            },
            Err(e) => CheckItem {
                key: "windows".into(),
                label: "Windows Version".into(),
                status: "fail".into(),
                detail: e,
                version: None,
            },
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        CheckItem {
            key: "windows".into(),
            label: "Windows Version".into(),
            status: "pass".into(),
            detail: "[Dev Mode] Windows 11 23H2".into(),
            version: Some("Windows 11 23H2".into()),
        }
    }
}

async fn check_internet() -> CheckItem {

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build();

    match client {
        Ok(client) => match client.head("https://nodejs.org").send().await {
            Ok(resp) if resp.status().is_success() || resp.status().is_redirection() => CheckItem {
                key: "internet".into(),
                label: "Internet Connection".into(),
                status: "pass".into(),
                detail: "Connected".into(),
                version: None,
            },
            Ok(resp) => CheckItem {
                key: "internet".into(),
                label: "Internet Connection".into(),
                status: "warn".into(),
                detail: format!("Unexpected status: {}", resp.status()),
                version: None,
            },
            Err(e) => CheckItem {
                key: "internet".into(),
                label: "Internet Connection".into(),
                status: "fail".into(),
                detail: format!("No internet connection: {}", e),
                version: None,
            },
        },
        Err(e) => CheckItem {
            key: "internet".into(),
            label: "Internet Connection".into(),
            status: "fail".into(),
            detail: format!("HTTP client error: {}", e),
            version: None,
        },
    }
}

fn check_disk_space() -> CheckItem {

    #[cfg(target_os = "windows")]
    {
        match process::run_powershell(
            "(Get-PSDrive C).Free / 1GB | ForEach-Object { [math]::Round($_, 1) }",
        ) {
            Ok(result) if result.success => {
                let free_gb: f64 = result.stdout.trim().parse().unwrap_or(0.0);
                let enough = free_gb >= 2.0;
                CheckItem {
                    key: "disk".into(),
                    label: "Disk Space (≥ 2GB)".into(),
                    status: if enough { "pass" } else { "fail" }.into(),
                    detail: format!("{:.1} GB free", free_gb),
                    version: None,
                }
            }
            _ => CheckItem {
                key: "disk".into(),
                label: "Disk Space (≥ 2GB)".into(),
                status: "warn".into(),
                detail: "Could not determine free space".into(),
                version: None,
            },
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        CheckItem {
            key: "disk".into(),
            label: "Disk Space (≥ 2GB)".into(),
            status: "pass".into(),
            detail: "[Dev Mode] 120.5 GB free".into(),
            version: None,
        }
    }
}

fn check_ram() -> CheckItem {

    #[cfg(target_os = "windows")]
    {
        match process::run_powershell(
            "[math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1GB, 1)",
        ) {
            Ok(result) if result.success => {
                let ram_gb: f64 = result.stdout.trim().parse().unwrap_or(0.0);
                let enough = ram_gb >= 4.0;
                CheckItem {
                    key: "ram".into(),
                    label: "RAM (≥ 4GB)".into(),
                    status: if enough { "pass" } else { "fail" }.into(),
                    detail: format!("{:.1} GB", ram_gb),
                    version: None,
                }
            }
            _ => CheckItem {
                key: "ram".into(),
                label: "RAM (≥ 4GB)".into(),
                status: "warn".into(),
                detail: "Could not determine RAM".into(),
                version: None,
            },
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        CheckItem {
            key: "ram".into(),
            label: "RAM (≥ 4GB)".into(),
            status: "pass".into(),
            detail: "[Dev Mode] 16.0 GB".into(),
            version: None,
        }
    }
}

fn check_git() -> CheckItem {
    match process::get_version("git", "--version") {
        Some(version) => {
            CheckItem {
                key: "git".into(),
                label: "Git for Windows".into(),
                status: "pass".into(),
                detail: version.clone(),
                version: Some(version),
            }
        }
        None => CheckItem {
            key: "git".into(),
            label: "Git for Windows".into(),
            status: "fail".into(),
            detail: "Not installed — will install".into(),
            version: None,
        },
    }
}

fn check_node() -> CheckItem {
    match process::get_version("node", "--version") {
        Some(version) => {
            // Parse version number (e.g., "v18.20.1" -> 18)
            let major: u32 = version
                .trim_start_matches('v')
                .split('.')
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);

            if major >= 18 {
                CheckItem {
                    key: "node".into(),
                    label: "Node.js".into(),
                    status: "pass".into(),
                    detail: version.clone(),
                    version: Some(version),
                }
            } else {
                CheckItem {
                    key: "node".into(),
                    label: "Node.js".into(),
                    status: "warn".into(),
                    detail: format!("{} → need v18+", version),
                    version: Some(version),
                }
            }
        }
        None => CheckItem {
            key: "node".into(),
            label: "Node.js".into(),
            status: "fail".into(),
            detail: "Not installed — will install".into(),
            version: None,
        },
    }
}

fn check_claude() -> CheckItem {
    match process::get_version("claude", "--version") {
        Some(version) => {
            CheckItem {
                key: "claude".into(),
                label: "Claude Code".into(),
                status: "pass".into(),
                detail: version.clone(),
                version: Some(version),
            }
        }
        None => CheckItem {
            key: "claude".into(),
            label: "Claude Code".into(),
            status: "fail".into(),
            detail: "Not installed — will install".into(),
            version: None,
        },
    }
}
