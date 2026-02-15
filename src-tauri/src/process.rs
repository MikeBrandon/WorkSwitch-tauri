use std::collections::HashSet;
use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn get_running_processes() -> HashSet<String> {
    let mut set = HashSet::new();

    #[cfg(target_os = "windows")]
    {
        let output = Command::new("tasklist")
            .args(["/FO", "CSV", "/NH"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                // Format: "process.exe","PID","Session Name","Session#","Mem Usage"
                if let Some(name) = line.split(',').next() {
                    let name = name.trim_matches('"').to_lowercase();
                    if !name.is_empty() {
                        set.insert(name);
                    }
                }
            }
        }
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        // Use ps to list all process names
        let output = Command::new("ps")
            .args(["-eo", "comm"])
            .output();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) {
                // ps output has full paths on some systems, just the name on others
                let name = line.trim();
                if !name.is_empty() {
                    // Extract just the binary name from path
                    let basename = std::path::Path::new(name)
                        .file_name()
                        .and_then(|f| f.to_str())
                        .unwrap_or(name);
                    set.insert(basename.to_lowercase());
                }
            }
        }
    }

    set
}

pub fn is_running(name: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        // Use targeted tasklist filter instead of listing all processes
        let output = Command::new("tasklist")
            .args(["/FI", &format!("IMAGENAME eq {}", name), "/FO", "CSV", "/NH"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // If the process is found, tasklist returns its info line
            // If not found, it returns "INFO: No tasks are running..."
            return stdout.to_lowercase().contains(&name.to_lowercase())
                && !stdout.contains("No tasks are running");
        }
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        // pgrep -ix does case-insensitive exact match
        let output = Command::new("pgrep")
            .args(["-ix", name])
            .output();

        if let Ok(output) = output {
            return output.status.success();
        }

        // Fallback: strip extension and try again (e.g. "firefox.exe" -> "firefox")
        let name_no_ext = name.strip_suffix(".exe").unwrap_or(name);
        if name_no_ext != name {
            let output = Command::new("pgrep")
                .args(["-ix", name_no_ext])
                .output();
            if let Ok(output) = output {
                return output.status.success();
            }
        }
    }

    false
}

pub fn kill_process(name: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("taskkill")
            .args(["/F", "/IM", name])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to run taskkill: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("taskkill failed: {}", stderr.trim()));
        }
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        // Try pkill with exact match first
        let output = Command::new("pkill")
            .args(["-ix", name])
            .output()
            .map_err(|e| format!("Failed to run pkill: {}", e))?;

        if !output.status.success() {
            // Fallback: try without .exe extension
            let name_no_ext = name.strip_suffix(".exe").unwrap_or(name);
            if name_no_ext != name {
                let output2 = Command::new("pkill")
                    .args(["-ix", name_no_ext])
                    .output()
                    .map_err(|e| format!("Failed to run pkill: {}", e))?;

                if !output2.status.success() {
                    return Err(format!("pkill failed for '{}'", name));
                }
            } else {
                return Err(format!("pkill failed for '{}'", name));
            }
        }
    }

    Ok(())
}
