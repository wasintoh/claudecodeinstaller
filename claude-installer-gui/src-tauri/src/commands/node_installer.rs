use crate::commands::InstallEvent;
use crate::utils::download;
use crate::utils::logger::AppLogger;
use crate::utils::process;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{ipc::Channel, State};

/// The Node.js LTS version to install. Update this when a new LTS is released.
const NODE_LTS_VERSION: &str = "22.14.0";

/// Install Node.js LTS by downloading the MSI and running msiexec silently.
#[tauri::command]
pub async fn install_node(
    on_event: Channel<InstallEvent>,
    logger: State<'_, Arc<Mutex<AppLogger>>>,
) -> Result<(), String> {
    let logger_arc = logger.inner().clone();
    {
        let log = logger_arc.lock().map_err(|e| e.to_string())?;
        log.info("Starting Node.js installation...");
    }

    // Step 1: Download Node.js MSI
    let download_url = format!(
        "https://nodejs.org/dist/v{}/node-v{}-x64.msi",
        NODE_LTS_VERSION, NODE_LTS_VERSION
    );

    on_event
        .send(InstallEvent::StepLog {
            step: "node".into(),
            level: "info".into(),
            message: format!("Downloading Node.js v{}...", NODE_LTS_VERSION),
        })
        .map_err(|e| e.to_string())?;

    let temp_dir = std::env::temp_dir();
    let msi_path = temp_dir.join(format!("node-v{}-x64.msi", NODE_LTS_VERSION));

    let event_sender = on_event.clone();
    download::download_with_retry(
        &download_url,
        &msi_path,
        3,
        move |progress| {
            let _ = event_sender.send(InstallEvent::DownloadProgress {
                step: "node".into(),
                downloaded: progress.downloaded,
                total: progress.total,
                speed_bps: progress.speed_bps,
                eta_secs: progress.eta_secs,
            });
        },
        |attempt, max, error| {
            eprintln!("Node.js download retry {}/{}: {}", attempt, max, error);
        },
    )
    .await?;

    // Step 2: Run MSI installer silently
    on_event
        .send(InstallEvent::StepLog {
            step: "node".into(),
            level: "info".into(),
            message: "Installing Node.js (this may take a moment)...".into(),
        })
        .map_err(|e| e.to_string())?;

    run_node_msi(&msi_path)?;

    // Step 3: Clean up
    let _ = std::fs::remove_file(&msi_path);

    // Step 4: Configure npm global prefix to a user-writable location
    on_event
        .send(InstallEvent::StepLog {
            step: "node".into(),
            level: "info".into(),
            message: "Configuring npm...".into(),
        })
        .map_err(|e| e.to_string())?;

    configure_npm()?;

    // Step 5: Verify installation
    on_event
        .send(InstallEvent::StepLog {
            step: "node".into(),
            level: "info".into(),
            message: "Verifying Node.js installation...".into(),
        })
        .map_err(|e| e.to_string())?;

    let version = verify_node_installed()?;

    {
        let log = logger_arc.lock().map_err(|e| e.to_string())?;
        log.info(format!("Node.js installed successfully: {}", version));
    }

    on_event
        .send(InstallEvent::StepCompleted {
            step: "node".into(),
            success: true,
            version: Some(version),
            error: None,
        })
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Run the Node.js MSI installer silently via msiexec
fn run_node_msi(msi_path: &PathBuf) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let msi_str = msi_path
            .to_str()
            .ok_or("Invalid MSI path")?;

        let result = process::run_command(
            "msiexec",
            &["/i", msi_str, "/qn", "/norestart"],
        )?;

        if !result.success {
            // msiexec may need elevation; try with PowerShell Start-Process
            let result = process::run_elevated(
                "msiexec",
                &format!("/i \"{}\" /qn /norestart", msi_str),
            )?;

            if !result.success {
                return Err(format!(
                    "Node.js MSI installation failed (exit code {:?}): {}",
                    result.exit_code, result.stderr
                ));
            }
        }
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        eprintln!("[Dev Mode] Would install Node.js MSI: {:?}", msi_path);
        Ok(())
    }
}

/// Configure npm to use a user-writable global prefix so packages
/// don't require admin rights to install.
fn configure_npm() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let home = std::env::var("USERPROFILE")
            .map_err(|_| "USERPROFILE environment variable not set".to_string())?;
        let npm_prefix = format!("{}\\AppData\\Roaming\\npm", home);

        // Create the directory if it doesn't exist
        let _ = std::fs::create_dir_all(&npm_prefix);

        // Try to find npm in common locations
        let npm_paths = [
            "npm",
            "C:\\Program Files\\nodejs\\npm.cmd",
        ];

        for npm in &npm_paths {
            let result = process::run_command(npm, &["config", "set", "prefix", &npm_prefix]);
            if let Ok(r) = result {
                if r.success {
                    return Ok(());
                }
            }
        }

        // Not critical if this fails - npm usually works with default config
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        eprintln!("[Dev Mode] Would configure npm prefix");
        Ok(())
    }
}

/// Verify Node.js is installed and return version string
fn verify_node_installed() -> Result<String, String> {
    // First try PATH
    if let Some(version) = process::get_version("node", "--version") {
        return Ok(version);
    }

    #[cfg(target_os = "windows")]
    {
        let common_paths = ["C:\\Program Files\\nodejs\\node.exe"];
        for path in &common_paths {
            if let Some(version) = process::get_version(path, "--version") {
                return Ok(version);
            }
        }
        return Err("Node.js installation could not be verified. It may require a terminal restart.".to_string());
    }

    #[cfg(not(target_os = "windows"))]
    Ok(format!("v{} [dev mode]", NODE_LTS_VERSION))
}
