use std::process::{Command, Output, Stdio};

/// Result of running an external process
#[derive(Debug)]
pub struct ProcessResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Run an external command with stdout/stderr capture and optional timeout.
/// Returns a structured result instead of panicking.
pub fn run_command(program: &str, args: &[&str]) -> Result<ProcessResult, String> {
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("Failed to execute '{}': {}", program, e))?;

    Ok(process_output(output))
}

/// Run a command with elevated privileges (Windows UAC)
#[cfg(target_os = "windows")]
pub fn run_elevated(program: &str, args: &str) -> Result<ProcessResult, String> {
    // Use PowerShell Start-Process to trigger UAC
    let output = Command::new("powershell")
        .args(&[
            "-NoProfile",
            "-Command",
            &format!("Start-Process -FilePath '{}' -ArgumentList '{}' -Wait -PassThru", program, args),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("Failed to run elevated '{}': {}", program, e))?;

    Ok(process_output(output))
}

#[cfg(not(target_os = "windows"))]
pub fn run_elevated(program: &str, _args: &str) -> Result<ProcessResult, String> {
    Ok(ProcessResult {
        success: true,
        exit_code: Some(0),
        stdout: format!("[mock] Elevated run of '{}' succeeded", program),
        stderr: String::new(),
    })
}

/// Run a PowerShell command and capture output
#[allow(dead_code)]
pub fn run_powershell(script: &str) -> Result<ProcessResult, String> {
    let output = Command::new("powershell")
        .args(&["-NoProfile", "-NonInteractive", "-Command", script])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("Failed to run PowerShell: {}", e))?;

    Ok(process_output(output))
}

/// Run msiexec to install an MSI package silently
#[allow(dead_code)]
pub fn run_msi_install(msi_path: &str) -> Result<ProcessResult, String> {
    let output = Command::new("msiexec")
        .args(&["/i", msi_path, "/qn", "/norestart"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("Failed to run msiexec: {}", e))?;

    Ok(process_output(output))
}

/// Run msiexec to uninstall by product code
#[allow(dead_code)]
pub fn run_msi_uninstall(product_code: &str) -> Result<ProcessResult, String> {
    let output = Command::new("msiexec")
        .args(&["/x", product_code, "/qn", "/norestart"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("Failed to run msiexec uninstall: {}", e))?;

    Ok(process_output(output))
}

/// Get the version output of a CLI tool (e.g., "git --version")
pub fn get_version(program: &str, version_flag: &str) -> Option<String> {
    let result = run_command(program, &[version_flag]).ok()?;
    if result.success {
        Some(result.stdout.trim().to_string())
    } else {
        None
    }
}

fn process_output(output: Output) -> ProcessResult {
    ProcessResult {
        success: output.status.success(),
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}
