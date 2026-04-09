// Tauri v2 requires a lib.rs for the library crate.
// The actual application logic is in main.rs.
// This file re-exports the modules for the Tauri build system.
//
// Many functions are only used on Windows (the target platform).
// Suppress dead_code warnings when compiling on macOS for development.
#![allow(dead_code)]

mod commands;
mod utils;

use commands::{
    claude_installer, git_installer, node_installer, path_manager, post_install, system_check,
    uninstaller,
};
use std::sync::{Arc, Mutex};
use utils::logger::AppLogger;

/// Check if the app was launched with --uninstall flag
#[tauri::command]
fn check_cli_args() -> bool {
    std::env::args().any(|arg| arg == "--uninstall")
}

/// Export installer logs to a file and return the file path
#[tauri::command]
fn export_logs(logger: tauri::State<'_, Arc<Mutex<AppLogger>>>) -> Result<String, String> {
    let log = logger.lock().map_err(|e| e.to_string())?;
    let path = log.export_to_file()?;
    Ok(path.to_string_lossy().to_string())
}

/// Open a PowerShell terminal window
#[tauri::command]
fn open_terminal() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("powershell")
            .arg("-NoExit")
            .arg("-Command")
            .arg("Write-Host 'Claude Code is ready! Type: claude' -ForegroundColor Green")
            .spawn()
            .map_err(|e| format!("Failed to open terminal: {}", e))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        eprintln!("[Dev Mode] Would open PowerShell terminal");
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let logger = Arc::new(Mutex::new(AppLogger::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(logger)
        .invoke_handler(tauri::generate_handler![
            check_cli_args,
            export_logs,
            open_terminal,
            system_check::system_check,
            git_installer::install_git,
            node_installer::install_node,
            claude_installer::install_claude,
            path_manager::fix_path,
            post_install::test_claude_code,
            post_install::launch_claude_in_new_terminal,
            uninstaller::detect_installed,
            uninstaller::uninstall_components,
        ])
        .run(tauri::generate_context!())
        .expect("Failed to launch Claude Code Installer");
}
