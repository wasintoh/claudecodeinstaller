use crate::commands::InstallEvent;
use crate::utils::download;
use crate::utils::logger::AppLogger;
use crate::utils::process;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{ipc::Channel, State};

/// Install Git for Windows by downloading the latest release from GitHub
/// and running the silent installer.
#[tauri::command]
pub async fn install_git(
    on_event: Channel<InstallEvent>,
    logger: State<'_, Arc<Mutex<AppLogger>>>,
) -> Result<(), String> {
    let logger_arc = logger.inner().clone();
    {
        let log = logger_arc.lock().map_err(|e| e.to_string())?;
        log.info("Starting Git installation...");
    }

    // Step 1: Get the latest Git for Windows download URL
    on_event
        .send(InstallEvent::StepLog {
            step: "git".into(),
            level: "info".into(),
            message: "Fetching latest Git for Windows release...".into(),
        })
        .map_err(|e| e.to_string())?;

    let download_url = get_git_download_url().await?;
    {
        let log = logger_arc.lock().map_err(|e| e.to_string())?;
        log.info(format!("Git download URL: {}", download_url));
    }

    // Step 2: Download the installer
    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.join("Git-Installer.exe");

    on_event
        .send(InstallEvent::StepLog {
            step: "git".into(),
            level: "info".into(),
            message: "Downloading Git for Windows...".into(),
        })
        .map_err(|e| e.to_string())?;

    let event_sender = on_event.clone();
    download::download_with_retry(
        &download_url,
        &installer_path,
        3,
        move |progress| {
            let _ = event_sender.send(InstallEvent::DownloadProgress {
                step: "git".into(),
                downloaded: progress.downloaded,
                total: progress.total,
                speed_bps: progress.speed_bps,
                eta_secs: progress.eta_secs,
            });
        },
        |attempt, max, error| {
            let log_msg = format!(
                "Git download retry {}/{}: {}",
                attempt, max, error
            );
            // Log retry attempt
            eprintln!("{}", log_msg);
        },
    )
    .await?;

    // Step 3: Run the silent installer
    on_event
        .send(InstallEvent::StepLog {
            step: "git".into(),
            level: "info".into(),
            message: "Running Git installer (this may take a moment)...".into(),
        })
        .map_err(|e| e.to_string())?;

    run_git_installer(&installer_path)?;

    // Step 4: Clean up installer
    let _ = std::fs::remove_file(&installer_path);

    // Step 5: Verify installation
    on_event
        .send(InstallEvent::StepLog {
            step: "git".into(),
            level: "info".into(),
            message: "Verifying Git installation...".into(),
        })
        .map_err(|e| e.to_string())?;

    // Git may not be in PATH yet; check common install locations
    let version = verify_git_installed()?;

    {
        let log = logger_arc.lock().map_err(|e| e.to_string())?;
        log.info(format!("Git installed successfully: {}", version));
    }

    on_event
        .send(InstallEvent::StepCompleted {
            step: "git".into(),
            success: true,
            version: Some(version),
            error: None,
        })
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Fetch the latest Git for Windows 64-bit installer URL from GitHub API
async fn get_git_download_url() -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent("Claude-Code-Installer/1.0")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let resp = client
        .get("https://api.github.com/repos/git-for-windows/git/releases/latest")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Git releases: {}", e))?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Git release JSON: {}", e))?;

    // Find the 64-bit installer asset
    let assets = json["assets"]
        .as_array()
        .ok_or("No assets found in Git release")?;

    for asset in assets {
        let name = asset["name"].as_str().unwrap_or("");
        // Look for the 64-bit standalone installer (e.g., "Git-2.47.1-64-bit.exe")
        if name.ends_with("-64-bit.exe") && !name.contains("portable") && !name.contains("busybox") {
            return asset["browser_download_url"]
                .as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| "No download URL for Git installer".to_string());
        }
    }

    Err("Could not find Git for Windows 64-bit installer in latest release".to_string())
}

/// Run the Git installer silently
fn run_git_installer(installer_path: &PathBuf) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let result = process::run_command(
            installer_path.to_str().unwrap_or(""),
            &[
                "/VERYSILENT",
                "/NORESTART",
                "/NOCANCEL",
                "/SP-",
                "/CLOSEAPPLICATIONS",
                "/RESTARTAPPLICATIONS",
                // Add Git to PATH
                "/COMPONENTS=ext,ext\\shellhere,ext\\guihere,gitlfs,assoc,assoc_sh,autoupdate",
            ],
        )?;

        if !result.success {
            return Err(format!(
                "Git installer failed with exit code {:?}: {}",
                result.exit_code, result.stderr
            ));
        }
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        eprintln!("[Dev Mode] Would run Git installer: {:?}", installer_path);
        Ok(())
    }
}

/// Verify Git is installed and return version string
fn verify_git_installed() -> Result<String, String> {
    // First try the PATH
    if let Some(version) = process::get_version("git", "--version") {
        return Ok(version);
    }

    #[cfg(target_os = "windows")]
    {
        let common_paths = [
            "C:\\Program Files\\Git\\cmd\\git.exe",
            "C:\\Program Files (x86)\\Git\\cmd\\git.exe",
        ];
        for path in &common_paths {
            if let Some(version) = process::get_version(path, "--version") {
                return Ok(version);
            }
        }
        return Err("Git installation could not be verified. It may require a terminal restart.".to_string());
    }

    #[cfg(not(target_os = "windows"))]
    Ok("git version 2.47.1 [dev mode]".to_string())
}
