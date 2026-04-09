use crate::utils::logger::AppLogger;
use std::sync::{Arc, Mutex};
use tauri::State;

/// Check and fix PATH entries for Git, Node.js, and Claude Code.
/// Returns a list of paths that were added.
#[tauri::command]
pub fn fix_path(
    logger: State<'_, Arc<Mutex<AppLogger>>>,
) -> Result<Vec<String>, String> {
    {
        let log = logger.lock().map_err(|e| e.to_string())?;
        log.info("Checking and fixing PATH...");
    }

    #[allow(unused_mut)]
    let mut added = Vec::new();

    #[cfg(target_os = "windows")]
    {
        let home = std::env::var("USERPROFILE").unwrap_or_default();

        // Paths that should be in the user PATH
        let required_paths = vec![
            ("Git", "C:\\Program Files\\Git\\cmd".to_string()),
            ("Node.js", "C:\\Program Files\\nodejs".to_string()),
            ("npm global", format!("{}\\AppData\\Roaming\\npm", home)),
            ("Claude Code", format!("{}\\.local\\bin", home)),
        ];

        for (label, path) in &required_paths {
            if std::path::Path::new(path).exists() {
                match add_to_path(path) {
                    Ok(true) => {
                        let log = logger.lock().map_err(|e| e.to_string())?;
                        log.info(format!("Added {} to PATH: {}", label, path));
                        added.push(format!("{}: {}", label, path));
                    }
                    Ok(false) => {
                        // Already in PATH
                    }
                    Err(e) => {
                        let log = logger.lock().map_err(|e2| e2.to_string())?;
                        log.warn(format!("Failed to add {} to PATH: {}", label, e));
                    }
                }
            }
        }

        // Broadcast environment change to all windows
        if !added.is_empty() {
            broadcast_environment_change();
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let log = logger.lock().map_err(|e| e.to_string())?;
        log.info("[Dev Mode] Would fix PATH entries");
    }

    Ok(added)
}

/// Add a directory to the user PATH if it's not already there.
/// Returns Ok(true) if the path was added, Ok(false) if it was already present.
#[cfg(target_os = "windows")]
pub fn add_to_path(new_path: &str) -> Result<bool, String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu
        .open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)
        .map_err(|e| format!("Failed to open Environment registry key: {}", e))?;

    let current_path: String = env
        .get_value("Path")
        .unwrap_or_default();

    // Check if the path is already present (case-insensitive on Windows)
    let lower_current = current_path.to_lowercase();
    let lower_new = new_path.to_lowercase();
    if lower_current
        .split(';')
        .any(|p| p.trim() == lower_new || p.trim().trim_end_matches('\\') == lower_new.trim_end_matches('\\'))
    {
        return Ok(false); // Already in PATH
    }

    // Append the new path
    let new_full_path = if current_path.is_empty() {
        new_path.to_string()
    } else if current_path.ends_with(';') {
        format!("{}{}", current_path, new_path)
    } else {
        format!("{};{}", current_path, new_path)
    };

    env.set_value("Path", &new_full_path)
        .map_err(|e| format!("Failed to update PATH: {}", e))?;

    Ok(true)
}

#[cfg(not(target_os = "windows"))]
pub fn add_to_path(_new_path: &str) -> Result<bool, String> {
    Ok(true) // Mock for dev
}

/// Remove a directory from the user PATH.
#[cfg(target_os = "windows")]
pub fn remove_from_path(path_to_remove: &str) -> Result<bool, String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu
        .open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)
        .map_err(|e| format!("Failed to open Environment registry key: {}", e))?;

    let current_path: String = env.get_value("Path").unwrap_or_default();
    let lower_remove = path_to_remove.to_lowercase();

    let new_parts: Vec<&str> = current_path
        .split(';')
        .filter(|p| {
            let trimmed = p.trim().to_lowercase();
            let trimmed_no_slash = trimmed.trim_end_matches('\\');
            let remove_no_slash = lower_remove.trim_end_matches('\\');
            trimmed != lower_remove && trimmed_no_slash != remove_no_slash
        })
        .collect();

    let new_path = new_parts.join(";");

    env.set_value("Path", &new_path)
        .map_err(|e| format!("Failed to update PATH: {}", e))?;

    Ok(true)
}

#[cfg(not(target_os = "windows"))]
pub fn remove_from_path(_path_to_remove: &str) -> Result<bool, String> {
    Ok(true)
}

/// Broadcast WM_SETTINGCHANGE to notify all windows that environment variables changed.
/// This is necessary after modifying the PATH so that new terminal windows pick up the change.
#[cfg(target_os = "windows")]
pub fn broadcast_environment_change() {
    use windows_sys::Win32::Foundation::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    unsafe {
        let env_str: Vec<u16> = "Environment\0".encode_utf16().collect();
        let mut result: usize = 0;
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            env_str.as_ptr() as isize,
            SMTO_ABORTIFHUNG,
            5000,
            &mut result,
        );
    }
}

#[cfg(not(target_os = "windows"))]
pub fn broadcast_environment_change() {
    eprintln!("[Dev Mode] Would broadcast WM_SETTINGCHANGE");
}
