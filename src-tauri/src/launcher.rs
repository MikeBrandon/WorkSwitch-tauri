use crate::config::Step;
use crate::process;
use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const DETACHED_PROCESS: u32 = 0x00000008;
#[cfg(target_os = "windows")]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn launch_step(step: &Step) -> Result<(), String> {
    match step.step_type.as_str() {
        "app" => launch_app(step),
        "terminal" => launch_terminal(step),
        "folder" => launch_folder(step),
        "url" => launch_url(step),
        _ => Err(format!("Unknown step type: {}", step.step_type)),
    }
}

fn launch_app(step: &Step) -> Result<(), String> {
    let target = step.target.as_deref().unwrap_or("");
    if target.is_empty() {
        return Err("No target specified".to_string());
    }

    // Check if already running
    if step.check_running.unwrap_or(true) && !step.process_name.is_empty() {
        if process::is_running(&step.process_name) {
            return Ok(()); // Already running, skip
        }
    }

    let target = expand_env_vars(target);

    // Check if it's a URI protocol (contains ":" but not ":\")
    if target.contains(':') && !target.contains(":\\") && !target.starts_with("\\\\") {
        // URI protocol like spotify:, figma:, etc.
        launch_uri(&target)
    } else if std::path::Path::new(&target).exists() {
        // It's a file path
        launch_exe(&target)
    } else {
        // Try as a command (e.g. "chrome" which might be in PATH)
        launch_via_start(&target)
    }
}

fn launch_uri(uri: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", uri])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("Failed to launch URI {}: {}", uri, e))?;
    }
    Ok(())
}

fn launch_exe(path: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new(path)
            .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
            .spawn()
            .map_err(|e| format!("Failed to launch {}: {}", path, e))?;
    }
    Ok(())
}

fn launch_via_start(target: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", target])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("Failed to start {}: {}", target, e))?;
    }
    Ok(())
}

fn launch_terminal(step: &Step) -> Result<(), String> {
    let command = step.command.as_deref().unwrap_or("");
    if command.is_empty() {
        return Err("No command specified".to_string());
    }

    let working_dir = step
        .working_dir
        .as_deref()
        .map(|d| expand_env_vars(d))
        .unwrap_or_default();

    let keep_open = step.keep_open.unwrap_or(true);

    #[cfg(target_os = "windows")]
    {
        let flag = if keep_open { "/K" } else { "/C" };
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "start", "cmd", flag, command]);

        if !working_dir.is_empty() {
            cmd.current_dir(&working_dir);
        }

        cmd.creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("Failed to launch terminal: {}", e))?;
    }

    Ok(())
}

fn launch_folder(step: &Step) -> Result<(), String> {
    let target = step.target.as_deref().unwrap_or("");
    if target.is_empty() {
        return Err("No folder specified".to_string());
    }

    let target = expand_env_vars(target);

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(&target)
            .creation_flags(DETACHED_PROCESS)
            .spawn()
            .map_err(|e| format!("Failed to open folder {}: {}", target, e))?;
    }

    Ok(())
}

fn launch_url(step: &Step) -> Result<(), String> {
    let target = step.target.as_deref().unwrap_or("");
    if target.is_empty() {
        return Err("No URL specified".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", target])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("Failed to open URL {}: {}", target, e))?;
    }

    Ok(())
}

fn expand_env_vars(input: &str) -> String {
    let mut result = input.to_string();
    // Expand %VAR% patterns
    while let Some(start) = result.find('%') {
        if let Some(end) = result[start + 1..].find('%') {
            let var_name = &result[start + 1..start + 1 + end];
            if let Ok(value) = std::env::var(var_name) {
                result = format!("{}{}{}", &result[..start], value, &result[start + 2 + end..]);
            } else {
                // Can't expand, skip this one
                break;
            }
        } else {
            break;
        }
    }
    result
}
