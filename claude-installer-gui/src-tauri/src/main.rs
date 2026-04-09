// Prevents a console window from appearing on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
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

fn main() {
    // Set up a custom panic handler to show errors instead of silently crashing
    std::panic::set_hook(Box::new(|info| {
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown error".to_string()
        };

        let location = info
            .location()
            .map(|l| format!(" at {}:{}", l.file(), l.line()))
            .unwrap_or_default();

        eprintln!("PANIC{}: {}", location, msg);

        // On Windows, show a message box so the user sees the error
        #[cfg(target_os = "windows")]
        {
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;

            let title: Vec<u16> = OsStr::new("Claude Code Installer - Error\0")
                .encode_wide()
                .collect();
            let message: Vec<u16> = OsStr::new(&format!(
                "An unexpected error occurred:\n\n{}\n\nPlease try restarting the installer.\0",
                msg
            ))
            .encode_wide()
            .collect();

            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                    0,
                    message.as_ptr(),
                    title.as_ptr(),
                    0x10, // MB_ICONERROR
                );
            }
        }
    }));

    // Create the shared logger instance
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
