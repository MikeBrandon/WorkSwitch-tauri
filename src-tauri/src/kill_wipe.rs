use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Deserialize)]
pub struct KillWipeOptions {
    pub kill_processes: bool,
    pub clear_temp: bool,
    pub clear_browsers: bool,
    pub flush_dns: bool,
    pub logout: bool,
}

#[derive(Debug, Serialize)]
pub struct KillWipeReport {
    pub killed_count: usize,
    pub kill_failures: Vec<String>,
    pub temp_failures: Vec<String>,
    pub browser_cleared: Vec<String>,
    pub browser_failures: Vec<String>,
    pub dns_flushed: bool,
}

pub fn run(options: &KillWipeOptions) -> KillWipeReport {
    let mut report = KillWipeReport {
        killed_count: 0,
        kill_failures: Vec::new(),
        temp_failures: Vec::new(),
        browser_cleared: Vec::new(),
        browser_failures: Vec::new(),
        dns_flushed: false,
    };

    if options.kill_processes {
        let (killed, failures) = kill_user_processes();
        report.killed_count = killed;
        report.kill_failures = failures;
    }

    if options.clear_temp {
        report
            .temp_failures
            .extend(clear_temp_folders().into_iter());
    }

    if options.clear_browsers {
        let (cleared, failures) = clear_browser_data();
        report.browser_cleared = cleared;
        report.browser_failures = failures;
    }

    if options.flush_dns {
        report.dns_flushed = flush_dns_cache();
    }

    report
}

pub fn request_logout() {
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("shutdown")
            .args(["/l"])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    }
}

pub fn create_desktop_shortcut(immediate: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get exe path: {}", e))?;
        let exe_str = exe_path.to_string_lossy().to_string();
        let desktop = desktop_dir().ok_or_else(|| "Unable to resolve Desktop path".to_string())?;
        let shortcut_path = desktop.join("WorkSwitch Kill & Wipe.lnk");
        let args = if immediate {
            "--kill-and-wipe-immediate"
        } else {
            "--kill-and-wipe"
        };

        let script = format!(
            "$WshShell = New-Object -ComObject WScript.Shell; \
             $Shortcut = $WshShell.CreateShortcut('{}'); \
             $Shortcut.TargetPath = '{}'; \
             $Shortcut.Arguments = '{}'; \
             $Shortcut.WorkingDirectory = '{}'; \
             $Shortcut.IconLocation = '{}'; \
             $Shortcut.Save();",
            escape_ps_string(&shortcut_path.to_string_lossy()),
            escape_ps_string(&exe_str),
            args,
            escape_ps_string(
                &exe_path
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "".to_string()),
            ),
            escape_ps_string(&exe_str)
        );

        let status = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .map_err(|e| format!("Failed to create shortcut: {}", e))?;

        if !status.success() {
            return Err("PowerShell failed to create shortcut".to_string());
        }
    }

    Ok(())
}

fn desktop_dir() -> Option<PathBuf> {
    let user_profile = std::env::var("USERPROFILE").ok()?;
    Some(PathBuf::from(user_profile).join("Desktop"))
}

fn escape_ps_string(input: &str) -> String {
    input.replace('\'', "''")
}

fn kill_user_processes() -> (usize, Vec<String>) {
    #[cfg(target_os = "windows")]
    {
        let current_exe = std::env::current_exe()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_default()
            .to_lowercase();
        let current_user = std::env::var("USERNAME").unwrap_or_default().to_lowercase();

        let critical = critical_processes();

        let output = Command::new("tasklist")
            .args(["/V", "/FO", "CSV", "/NH"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        let mut killed = 0usize;
        let mut failures = Vec::new();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut targets = HashSet::new();

            for line in stdout.lines() {
                let fields = parse_csv_line(line);
                if fields.len() < 7 {
                    continue;
                }
                let image = fields[0].trim().to_lowercase();
                let user = fields[6].trim().to_lowercase();

                if image.is_empty() {
                    continue;
                }
                if image == current_exe {
                    continue;
                }
                if critical.contains(&image) {
                    continue;
                }
                if is_system_user(&user) {
                    continue;
                }
                if !is_current_user(&user, &current_user) {
                    continue;
                }
                targets.insert(image);
            }

            for name in targets {
                let output = Command::new("taskkill")
                    .args(["/F", "/IM", &name])
                    .creation_flags(CREATE_NO_WINDOW)
                    .output();

                match output {
                    Ok(out) => {
                        if out.status.success() {
                            killed += 1;
                        } else {
                            let stderr = String::from_utf8_lossy(&out.stderr);
                            failures.push(format!("{}: {}", name, stderr.trim()));
                        }
                    }
                    Err(e) => failures.push(format!("{}: {}", name, e)),
                }
            }
        }

        return (killed, failures);
    }
    #[cfg(not(target_os = "windows"))]
    {
        (0, vec![])
    }
}

fn clear_temp_folders() -> Vec<String> {
    let mut failures = Vec::new();

    let mut paths = Vec::new();
    if let Ok(temp) = std::env::var("TEMP") {
        paths.push(PathBuf::from(temp));
    }
    if let Ok(tmp) = std::env::var("TMP") {
        paths.push(PathBuf::from(tmp));
    }
    paths.push(PathBuf::from(r"C:\Windows\Temp"));

    for path in paths {
        if !path.exists() {
            continue;
        }
        if let Err(e) = clear_directory_contents(&path) {
            failures.push(format!("{}: {}", path.to_string_lossy(), e));
        }
    }

    failures
}

fn clear_browser_data() -> (Vec<String>, Vec<String>) {
    let mut cleared = Vec::new();
    let mut failures = Vec::new();

    let local_app = std::env::var("LOCALAPPDATA").ok();
    let roam_app = std::env::var("APPDATA").ok();

    if let Some(local) = local_app {
        let local = PathBuf::from(local);
        let chromium = vec![
            ("Chrome", local.join(r"Google\Chrome\User Data")),
            ("Edge", local.join(r"Microsoft\Edge\User Data")),
            ("Brave", local.join(r"BraveSoftware\Brave-Browser\User Data")),
        ];

        for (name, base) in chromium {
            if base.exists() {
                let (ok, err) = clear_chromium_profiles(&base);
                if ok {
                    cleared.push(name.to_string());
                }
                failures.extend(err.into_iter().map(|e| format!("{}: {}", name, e)));
            }
        }
    }

    if let Some(roam) = roam_app {
        let ff_base = PathBuf::from(roam).join(r"Mozilla\Firefox\Profiles");
        if ff_base.exists() {
            let (ok, err) = clear_firefox_profiles(&ff_base);
            if ok {
                cleared.push("Firefox".to_string());
            }
            failures.extend(err.into_iter().map(|e| format!("Firefox: {}", e)));
        }
    }

    (cleared, failures)
}

fn clear_chromium_profiles(base: &Path) -> (bool, Vec<String>) {
    let mut errors = Vec::new();
    let profiles = list_profile_dirs(base);

    for profile in &profiles {
        let paths = vec![
            profile.join("Cache"),
            profile.join("Code Cache"),
            profile.join("GPUCache"),
            profile.join("History"),
            profile.join("History-wal"),
            profile.join("History-journal"),
            profile.join("Cookies"),
            profile.join("Cookies-wal"),
            profile.join("Cookies-journal"),
            profile.join("Network").join("Cookies"),
            profile.join("Network").join("Cookies-wal"),
            profile.join("Network").join("Cookies-journal"),
            profile.join("Service Worker").join("CacheStorage"),
        ];

        for p in paths {
            if let Err(e) = remove_path(&p) {
                errors.push(format!("{}: {}", p.to_string_lossy(), e));
            }
        }
    }

    (!profiles.is_empty(), errors)
}

fn clear_firefox_profiles(base: &Path) -> (bool, Vec<String>) {
    let mut errors = Vec::new();
    let profiles = list_all_dirs(base);

    for profile in &profiles {
        let paths = vec![
            profile.join("cache2"),
            profile.join("cookies.sqlite"),
            profile.join("cookies.sqlite-wal"),
            profile.join("cookies.sqlite-shm"),
            profile.join("places.sqlite"),
            profile.join("places.sqlite-wal"),
            profile.join("places.sqlite-shm"),
        ];

        for p in paths {
            if let Err(e) = remove_path(&p) {
                errors.push(format!("{}: {}", p.to_string_lossy(), e));
            }
        }
    }

    (!profiles.is_empty(), errors)
}

fn list_profile_dirs(base: &Path) -> Vec<PathBuf> {
    let mut profiles = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(base) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "Default" || name.starts_with("Profile ") {
                profiles.push(path);
            }
        }
    }
    profiles
}

fn list_all_dirs(base: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(base) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            }
        }
    }
    dirs
}

fn remove_path(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        std::fs::remove_dir_all(path).map_err(|e| e.to_string())?;
    } else {
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn clear_directory_contents(path: &Path) -> Result<(), String> {
    let entries = std::fs::read_dir(path).map_err(|e| e.to_string())?;
    let mut errors: HashMap<String, String> = HashMap::new();

    for entry in entries.flatten() {
        let p = entry.path();
        let result = if p.is_dir() {
            std::fs::remove_dir_all(&p).map_err(|e| e.to_string())
        } else {
            std::fs::remove_file(&p).map_err(|e| e.to_string())
        };
        if let Err(e) = result {
            errors.insert(p.to_string_lossy().to_string(), e);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("Failed to delete {} items", errors.len()))
    }
}

fn flush_dns_cache() -> bool {
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = Command::new("ipconfig")
            .args(["/flushdns"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            return output.status.success();
        }
    }
    false
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in line.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
            continue;
        }
        if ch == ',' && !in_quotes {
            fields.push(current.clone());
            current.clear();
        } else {
            current.push(ch);
        }
    }
    fields.push(current);
    fields
}

fn is_current_user(user: &str, current: &str) -> bool {
    if current.is_empty() || user.is_empty() {
        return false;
    }
    if user == current {
        return true;
    }
    if let Some(pos) = user.rfind('\\') {
        return user[pos + 1..].eq_ignore_ascii_case(current);
    }
    false
}

fn is_system_user(user: &str) -> bool {
    let u = user.to_lowercase();
    u == "system" || u == "local service" || u == "network service"
}

fn critical_processes() -> HashSet<String> {
    let mut set = HashSet::new();
    for name in [
        "system",
        "system idle process",
        "smss.exe",
        "csrss.exe",
        "wininit.exe",
        "winlogon.exe",
        "services.exe",
        "lsass.exe",
        "lsm.exe",
        "svchost.exe",
        "fontdrvhost.exe",
        "dwm.exe",
        "registry",
        "memcompression",
        "securityhealthservice.exe",
        "sihost.exe",
        "ctfmon.exe",
    ] {
        set.insert(name.to_string());
    }
    set
}
