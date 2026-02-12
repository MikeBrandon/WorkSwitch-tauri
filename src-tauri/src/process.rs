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

    set
}

pub fn is_running(name: &str) -> bool {
    // Use targeted tasklist filter instead of listing all processes
    #[cfg(target_os = "windows")]
    {
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

    Ok(())
}
