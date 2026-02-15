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

    // Detect URI vs file path vs command
    if is_uri(&target) {
        launch_uri(&target)
    } else if std::path::Path::new(&target).exists() {
        launch_exe(&target)
    } else {
        // Try as a command (e.g. "chrome" which might be in PATH)
        launch_via_open(&target)
    }
}

/// Check if a string looks like a URI protocol
fn is_uri(target: &str) -> bool {
    // URI has ":" but is not a Windows drive letter like "C:\"
    if let Some(colon_pos) = target.find(':') {
        // Scheme must be at least 2 chars (to avoid matching "C:")
        if colon_pos >= 2 {
            let scheme = &target[..colon_pos];
            return scheme.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '.' || c == '-');
        }
    }
    false
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

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(uri)
            .spawn()
            .map_err(|e| format!("Failed to launch URI {}: {}", uri, e))?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(uri)
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

    #[cfg(target_os = "macos")]
    {
        // If it's a .app bundle, use 'open'
        if path.ends_with(".app") {
            Command::new("open")
                .arg(path)
                .spawn()
                .map_err(|e| format!("Failed to launch {}: {}", path, e))?;
        } else {
            Command::new(path)
                .spawn()
                .map_err(|e| format!("Failed to launch {}: {}", path, e))?;
        }
    }

    #[cfg(target_os = "linux")]
    {
        Command::new(path)
            .spawn()
            .map_err(|e| format!("Failed to launch {}: {}", path, e))?;
    }

    Ok(())
}

fn launch_via_open(target: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", target])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("Failed to start {}: {}", target, e))?;
    }

    #[cfg(target_os = "macos")]
    {
        // Try 'open -a' to launch by app name
        Command::new("open")
            .args(["-a", target])
            .spawn()
            .map_err(|e| format!("Failed to open {}: {}", target, e))?;
    }

    #[cfg(target_os = "linux")]
    {
        // Try running directly (might be in PATH)
        Command::new(target)
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

    #[cfg(target_os = "macos")]
    {
        // Use osascript to open Terminal.app and run the command
        let script = if keep_open {
            if working_dir.is_empty() {
                format!(
                    r#"tell application "Terminal" to do script "{}""#,
                    command.replace('"', r#"\""#)
                )
            } else {
                format!(
                    r#"tell application "Terminal" to do script "cd '{}' && {}""#,
                    working_dir.replace('\'', "'\\''"),
                    command.replace('"', r#"\""#)
                )
            }
        } else {
            let cmd_str = if working_dir.is_empty() {
                command.to_string()
            } else {
                format!("cd '{}' && {}", working_dir.replace('\'', "'\\''"), command)
            };
            format!(
                r#"tell application "Terminal" to do script "{} ; exit""#,
                cmd_str.replace('"', r#"\""#)
            )
        };

        Command::new("osascript")
            .args(["-e", &script])
            .spawn()
            .map_err(|e| format!("Failed to launch terminal: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        // Try common terminal emulators in order of preference
        let shell_cmd = if keep_open {
            format!("{}; exec bash", command)
        } else {
            command.to_string()
        };

        let terminals = [
            ("x-terminal-emulator", vec!["-e", "bash", "-c"]),
            ("gnome-terminal", vec!["--", "bash", "-c"]),
            ("konsole", vec!["-e", "bash", "-c"]),
            ("xfce4-terminal", vec!["-e", "bash -c"]),
            ("xterm", vec!["-e", "bash", "-c"]),
        ];

        let mut launched = false;
        for (term, args) in &terminals {
            let mut cmd = Command::new(term);
            for arg in args {
                cmd.arg(arg);
            }
            cmd.arg(&shell_cmd);

            if !working_dir.is_empty() {
                cmd.current_dir(&working_dir);
            }

            if cmd.spawn().is_ok() {
                launched = true;
                break;
            }
        }

        if !launched {
            return Err("No terminal emulator found. Install gnome-terminal, konsole, xfce4-terminal, or xterm.".to_string());
        }
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

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&target)
            .spawn()
            .map_err(|e| format!("Failed to open folder {}: {}", target, e))?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&target)
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

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(target)
            .spawn()
            .map_err(|e| format!("Failed to open URL {}: {}", target, e))?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(target)
            .spawn()
            .map_err(|e| format!("Failed to open URL {}: {}", target, e))?;
    }

    Ok(())
}

fn expand_env_vars(input: &str) -> String {
    let mut result = input.to_string();

    // Windows: expand %VAR% patterns
    #[cfg(target_os = "windows")]
    {
        while let Some(start) = result.find('%') {
            if let Some(end) = result[start + 1..].find('%') {
                let var_name = &result[start + 1..start + 1 + end];
                if let Ok(value) = std::env::var(var_name) {
                    result = format!("{}{}{}", &result[..start], value, &result[start + 2 + end..]);
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    // Unix: expand $VAR and ${VAR} patterns
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        // Handle ${VAR} first
        while let Some(start) = result.find("${") {
            if let Some(end) = result[start + 2..].find('}') {
                let var_name = &result[start + 2..start + 2 + end];
                if let Ok(value) = std::env::var(var_name) {
                    result = format!("{}{}{}", &result[..start], value, &result[start + 3 + end..]);
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // Handle $VAR (word boundary: var name is alphanumeric + underscore)
        let mut i = 0;
        let bytes = result.as_bytes();
        let mut expanded = String::new();
        while i < bytes.len() {
            if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1] != b'{' {
                // Read var name
                let start = i + 1;
                let mut end = start;
                while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
                    end += 1;
                }
                if end > start {
                    let var_name = &result[start..end];
                    if let Ok(value) = std::env::var(var_name) {
                        expanded.push_str(&value);
                    } else {
                        expanded.push('$');
                        expanded.push_str(var_name);
                    }
                    i = end;
                    continue;
                }
            }
            expanded.push(result.as_bytes()[i] as char);
            i += 1;
        }
        result = expanded;

        // Also support ~ for home directory
        if result.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                result = format!("{}{}", home.display(), &result[1..]);
            }
        }
    }

    result
}
