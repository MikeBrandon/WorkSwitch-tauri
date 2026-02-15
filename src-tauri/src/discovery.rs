use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredApp {
    pub name: String,
    pub target: String,
    pub process_name: String,
    pub source: String,
}

/// Run all scanners, merge, deduplicate by target, sort by name.
pub fn scan_all() -> Vec<DiscoveredApp> {
    let mut apps = Vec::new();
    apps.extend(scan_steam());
    apps.extend(scan_epic());
    apps.extend(scan_installed_apps());

    // Deduplicate by lowercase target
    let mut seen = HashSet::new();
    apps.retain(|app| seen.insert(app.target.to_lowercase()));

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

// ═══════════════════════════════════════════════════════════════
// Steam scanner (cross-platform: ACF parsing is the same, path differs)
// ═══════════════════════════════════════════════════════════════

/// Known non-game appids to skip (tools, redistributables, etc.)
const STEAM_SKIP_IDS: &[&str] = &[
    "228980",  // Steamworks Common Redistributables
    "1007",    // Steam Client
    "1070560", // Steam Linux Runtime
    "1391110", // Steam Linux Runtime - Soldier
    "1628350", // Steam Linux Runtime - Sniper
    "250820",  // SteamVR
];

fn scan_steam() -> Vec<DiscoveredApp> {
    let steam_path = match get_steam_path() {
        Some(p) => p,
        None => return vec![],
    };

    let library_paths = get_steam_library_paths(&steam_path);
    let mut apps = Vec::new();

    for lib_path in library_paths {
        let steamapps = lib_path.join("steamapps");
        if !steamapps.is_dir() {
            continue;
        }

        let pattern = steamapps.join("appmanifest_*.acf");
        let pattern_str = pattern.to_string_lossy().to_string();
        let entries = match glob_files(&pattern_str) {
            Some(e) => e,
            None => continue,
        };

        for acf_path in entries {
            if let Some(app) = parse_acf(&acf_path) {
                apps.push(app);
            }
        }
    }

    apps
}

fn get_steam_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let steam_key = hkcu.open_subkey("SOFTWARE\\Valve\\Steam").ok()?;
        let steam_path: String = steam_key.get_value("SteamPath").ok()?;
        let path = PathBuf::from(steam_path);
        if path.is_dir() {
            return Some(path);
        }
        return None;
    }

    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir()?;
        let path = home.join("Library/Application Support/Steam");
        if path.is_dir() {
            return Some(path);
        }
        return None;
    }

    #[cfg(target_os = "linux")]
    {
        let home = dirs::home_dir()?;
        // Check common Steam locations on Linux
        let candidates = [
            home.join(".steam/steam"),
            home.join(".local/share/Steam"),
            home.join(".steam/debian-installation"),
        ];
        for path in &candidates {
            if path.is_dir() {
                return Some(path.clone());
            }
        }
        return None;
    }
}

fn get_steam_library_paths(steam_path: &Path) -> Vec<PathBuf> {
    let mut paths = vec![steam_path.to_path_buf()];

    let vdf_path = steam_path.join("steamapps").join("libraryfolders.vdf");
    let content = match std::fs::read_to_string(&vdf_path) {
        Ok(c) => c,
        Err(_) => return paths,
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(value) = extract_vdf_value(trimmed, "path") {
            let lib_path = PathBuf::from(value.replace("\\\\", "\\"));
            if lib_path.is_dir() && lib_path != steam_path.to_path_buf() {
                paths.push(lib_path);
            }
        }
    }

    paths
}

fn parse_acf(path: &Path) -> Option<DiscoveredApp> {
    let content = std::fs::read_to_string(path).ok()?;

    let mut appid = String::new();
    let mut name = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(v) = extract_vdf_value(trimmed, "appid") {
            appid = v;
        }
        if let Some(v) = extract_vdf_value(trimmed, "name") {
            name = v;
        }
    }

    if appid.is_empty() || name.is_empty() {
        return None;
    }

    if STEAM_SKIP_IDS.contains(&appid.as_str()) {
        return None;
    }

    let name_lower = name.to_lowercase();
    if name_lower.contains("redistributable")
        || name_lower.contains("redist")
        || name_lower.contains("directx")
        || name_lower.contains("vcredist")
        || name_lower.contains("proton ")
        || name_lower.starts_with("steamworks")
    {
        return None;
    }

    Some(DiscoveredApp {
        name,
        target: format!("steam://rungameid/{}", appid),
        process_name: String::new(),
        source: "steam".to_string(),
    })
}

/// Extract a value from a VDF line like `"key"		"value"`
fn extract_vdf_value(line: &str, key: &str) -> Option<String> {
    let trimmed = line.trim();
    let key_pattern = format!("\"{}\"", key);
    if !trimmed.starts_with(&key_pattern) {
        return None;
    }

    let rest = trimmed[key_pattern.len()..].trim();
    if rest.starts_with('"') && rest.len() > 1 {
        let end = rest[1..].find('"')?;
        Some(rest[1..1 + end].to_string())
    } else {
        None
    }
}

/// Simple glob for a pattern like `/path/to/appmanifest_*.acf`
fn glob_files(pattern: &str) -> Option<Vec<PathBuf>> {
    let path = Path::new(pattern);
    let dir = path.parent()?;
    let file_pattern = path.file_name()?.to_string_lossy();

    let parts: Vec<&str> = file_pattern.splitn(2, '*').collect();
    if parts.len() != 2 {
        return None;
    }
    let prefix = parts[0];
    let suffix = parts[1];

    let mut results = Vec::new();
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with(prefix) && name_str.ends_with(suffix) {
            results.push(entry.path());
        }
    }
    Some(results)
}

// ═══════════════════════════════════════════════════════════════
// Epic Games scanner
// ═══════════════════════════════════════════════════════════════

fn scan_epic() -> Vec<DiscoveredApp> {
    let manifests_dir = get_epic_manifests_dir();
    let manifests_dir = match manifests_dir {
        Some(d) if d.is_dir() => d,
        _ => return vec![],
    };

    let mut apps = Vec::new();
    let entries = match std::fs::read_dir(&manifests_dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("item") {
            continue;
        }
        if let Some(app) = parse_epic_manifest(&path) {
            apps.push(app);
        }
    }

    apps
}

fn get_epic_manifests_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let path = PathBuf::from(r"C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests");
        return Some(path);
    }

    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir()?;
        let path = home.join("Library/Application Support/Epic/EpicGamesLauncher/Data/Manifests");
        return Some(path);
    }

    #[cfg(target_os = "linux")]
    {
        // Epic doesn't have a native Linux launcher
        // Could potentially scan Heroic Games Launcher in the future
        return None;
    }
}

fn parse_epic_manifest(path: &Path) -> Option<DiscoveredApp> {
    let content = std::fs::read_to_string(path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;

    if json.get("bIsApplication").and_then(|v| v.as_bool()) == Some(false) {
        return None;
    }

    let display_name = json.get("DisplayName")?.as_str()?.to_string();
    let app_name = json.get("AppName")?.as_str()?;
    let namespace = json.get("CatalogNamespace")?.as_str()?;
    let item_id = json.get("CatalogItemId")?.as_str()?;
    let launch_exe = json
        .get("LaunchExecutable")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let target = format!(
        "com.epicgames.launcher://apps/{}%3A{}%3A{}?action=launch",
        namespace, item_id, app_name
    );

    let process_name = Path::new(launch_exe)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("")
        .to_string();

    if display_name.is_empty() {
        return None;
    }

    Some(DiscoveredApp {
        name: display_name,
        target,
        process_name,
        source: "epic".to_string(),
    })
}

// ═══════════════════════════════════════════════════════════════
// Installed apps scanner (platform-specific)
// ═══════════════════════════════════════════════════════════════

fn scan_installed_apps() -> Vec<DiscoveredApp> {
    #[cfg(target_os = "windows")]
    {
        scan_installed_apps_windows()
    }

    #[cfg(target_os = "macos")]
    {
        scan_installed_apps_macos()
    }

    #[cfg(target_os = "linux")]
    {
        scan_installed_apps_linux()
    }
}

// ── Windows: registry-based scanner ──

#[cfg(target_os = "windows")]
fn scan_installed_apps_windows() -> Vec<DiscoveredApp> {
    use winreg::enums::*;
    use winreg::RegKey;

    let mut apps = Vec::new();
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    let uninstall_paths = [
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ];

    let mut seen_names = HashSet::new();

    for base_path in &uninstall_paths {
        let uninstall_key = match hklm.open_subkey(base_path) {
            Ok(k) => k,
            Err(_) => continue,
        };

        for subkey_name in uninstall_key.enum_keys().flatten() {
            let subkey = match uninstall_key.open_subkey(&subkey_name) {
                Ok(k) => k,
                Err(_) => continue,
            };

            if let Some(app) = parse_uninstall_entry(&subkey) {
                let key = app.name.to_lowercase();
                if !seen_names.contains(&key) {
                    seen_names.insert(key);
                    apps.push(app);
                }
            }
        }
    }

    apps
}

#[cfg(target_os = "windows")]
fn parse_uninstall_entry(key: &winreg::RegKey) -> Option<DiscoveredApp> {
    let display_name: String = key.get_value("DisplayName").ok()?;
    let name = display_name.trim().to_string();

    if name.is_empty()
        || name.starts_with("KB")
        || name.starts_with('{')
        || name.contains("Update for")
        || name.contains("Security Update")
        || name.contains("Hotfix for")
        || name.contains("Microsoft Visual C++")
        || name.contains(".NET Framework")
        || name.contains("Windows SDK")
    {
        return None;
    }

    let sys_component: u32 = key.get_value("SystemComponent").unwrap_or(0);
    if sys_component == 1 {
        return None;
    }

    let display_icon: String = key.get_value("DisplayIcon").unwrap_or_default();
    let install_location: String = key.get_value("InstallLocation").unwrap_or_default();

    let exe_path = extract_exe_from_icon(&display_icon)
        .or_else(|| find_exe_in_location(&install_location));

    let target = match &exe_path {
        Some(p) => p.clone(),
        None => return None,
    };

    let process_name = Path::new(&target)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("")
        .to_string();

    let process_lower = process_name.to_lowercase();
    if process_lower.contains("unins") || process_lower.contains("uninst") {
        return None;
    }

    Some(DiscoveredApp {
        name,
        target,
        process_name,
        source: "windows".to_string(),
    })
}

#[cfg(target_os = "windows")]
fn extract_exe_from_icon(icon: &str) -> Option<String> {
    if icon.is_empty() {
        return None;
    }

    let path = if let Some(comma_pos) = icon.rfind(',') {
        let after = &icon[comma_pos + 1..];
        if after.trim().chars().all(|c| c == '-' || c.is_ascii_digit()) {
            icon[..comma_pos].trim().trim_matches('"').to_string()
        } else {
            icon.trim().trim_matches('"').to_string()
        }
    } else {
        icon.trim().trim_matches('"').to_string()
    };

    if path.to_lowercase().ends_with(".exe") && Path::new(&path).exists() {
        Some(path)
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
fn find_exe_in_location(location: &str) -> Option<String> {
    if location.is_empty() {
        return None;
    }

    let dir = Path::new(location);
    if !dir.is_dir() {
        return None;
    }

    let entries = std::fs::read_dir(dir).ok()?;
    let mut exes: Vec<PathBuf> = entries
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            if p.extension().and_then(|ext| ext.to_str()) == Some("exe") {
                let name = p.file_name()?.to_string_lossy().to_lowercase();
                if !name.contains("unins")
                    && !name.contains("uninst")
                    && !name.contains("update")
                    && !name.contains("crash")
                    && !name.contains("helper")
                    && !name.contains("setup")
                {
                    return Some(p);
                }
            }
            None
        })
        .collect();

    if exes.is_empty() {
        return None;
    }

    if exes.len() == 1 {
        return Some(exes.remove(0).to_string_lossy().to_string());
    }

    exes.sort_by(|a, b| {
        let size_a = std::fs::metadata(a).map(|m| m.len()).unwrap_or(0);
        let size_b = std::fs::metadata(b).map(|m| m.len()).unwrap_or(0);
        size_b.cmp(&size_a)
    });

    Some(exes[0].to_string_lossy().to_string())
}

// ── macOS: .app bundle scanner ──

#[cfg(target_os = "macos")]
fn scan_installed_apps_macos() -> Vec<DiscoveredApp> {
    let mut apps = Vec::new();
    let mut seen_names = HashSet::new();

    // Scan /Applications and ~/Applications
    let mut dirs_to_scan = vec![PathBuf::from("/Applications")];
    if let Some(home) = dirs::home_dir() {
        dirs_to_scan.push(home.join("Applications"));
    }

    for dir in &dirs_to_scan {
        if !dir.is_dir() {
            continue;
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let name_os = entry.file_name();
            let name_str = name_os.to_string_lossy();

            if !name_str.ends_with(".app") {
                continue;
            }

            // Display name: strip .app extension
            let display_name = name_str.trim_end_matches(".app").to_string();

            // Skip duplicates
            let key = display_name.to_lowercase();
            if seen_names.contains(&key) {
                continue;
            }
            seen_names.insert(key);

            // Get the actual executable from Contents/MacOS/
            let macos_dir = path.join("Contents/MacOS");
            let process_name = if macos_dir.is_dir() {
                find_main_executable_in_dir(&macos_dir).unwrap_or_default()
            } else {
                String::new()
            };

            apps.push(DiscoveredApp {
                name: display_name,
                target: path.to_string_lossy().to_string(),
                process_name,
                source: "macos".to_string(),
            });
        }
    }

    apps
}

#[cfg(target_os = "macos")]
fn find_main_executable_in_dir(dir: &Path) -> Option<String> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut executables: Vec<PathBuf> = entries
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            if p.is_file() {
                // Check if executable
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = p.metadata() {
                    if meta.permissions().mode() & 0o111 != 0 {
                        return Some(p);
                    }
                }
            }
            None
        })
        .collect();

    if executables.is_empty() {
        return None;
    }

    // If only one, use it. Otherwise pick the largest.
    if executables.len() == 1 {
        return executables[0]
            .file_name()
            .and_then(|f| f.to_str())
            .map(|s| s.to_string());
    }

    executables.sort_by(|a, b| {
        let size_a = std::fs::metadata(a).map(|m| m.len()).unwrap_or(0);
        let size_b = std::fs::metadata(b).map(|m| m.len()).unwrap_or(0);
        size_b.cmp(&size_a)
    });

    executables[0]
        .file_name()
        .and_then(|f| f.to_str())
        .map(|s| s.to_string())
}

// ── Linux: .desktop file scanner ──

#[cfg(target_os = "linux")]
fn scan_installed_apps_linux() -> Vec<DiscoveredApp> {
    let mut apps = Vec::new();
    let mut seen_names = HashSet::new();

    // XDG data directories for .desktop files
    let mut dirs_to_scan = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
    ];

    // Add user-local applications
    if let Some(home) = dirs::home_dir() {
        dirs_to_scan.push(home.join(".local/share/applications"));
    }

    // Also check XDG_DATA_DIRS
    if let Ok(xdg_dirs) = std::env::var("XDG_DATA_DIRS") {
        for dir in xdg_dirs.split(':') {
            let path = PathBuf::from(dir).join("applications");
            if !dirs_to_scan.contains(&path) {
                dirs_to_scan.push(path);
            }
        }
    }

    for dir in &dirs_to_scan {
        if !dir.is_dir() {
            continue;
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                continue;
            }

            if let Some(app) = parse_desktop_file(&path) {
                let key = app.name.to_lowercase();
                if !seen_names.contains(&key) {
                    seen_names.insert(key);
                    apps.push(app);
                }
            }
        }
    }

    apps
}

#[cfg(target_os = "linux")]
fn parse_desktop_file(path: &Path) -> Option<DiscoveredApp> {
    let content = std::fs::read_to_string(path).ok()?;

    let mut name = String::new();
    let mut exec = String::new();
    let mut app_type = String::new();
    let mut no_display = false;
    let mut in_desktop_entry = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "[Desktop Entry]" {
            in_desktop_entry = true;
            continue;
        }
        if trimmed.starts_with('[') {
            // Another section started
            if in_desktop_entry {
                break;
            }
            continue;
        }

        if !in_desktop_entry {
            continue;
        }

        if let Some(value) = trimmed.strip_prefix("Name=") {
            if name.is_empty() {
                name = value.to_string();
            }
        } else if let Some(value) = trimmed.strip_prefix("Exec=") {
            exec = value.to_string();
        } else if let Some(value) = trimmed.strip_prefix("Type=") {
            app_type = value.to_string();
        } else if trimmed.starts_with("NoDisplay=true") {
            no_display = true;
        }
    }

    // Must be an Application type and visible
    if app_type != "Application" || no_display || name.is_empty() || exec.is_empty() {
        return None;
    }

    // Clean up Exec: remove field codes like %f, %u, %F, %U, etc.
    let exec_clean = exec
        .replace("%f", "")
        .replace("%F", "")
        .replace("%u", "")
        .replace("%U", "")
        .replace("%d", "")
        .replace("%D", "")
        .replace("%n", "")
        .replace("%N", "")
        .replace("%i", "")
        .replace("%c", "")
        .replace("%k", "")
        .trim()
        .to_string();

    // Extract just the command (first token) for process name
    let cmd_part = exec_clean.split_whitespace().next().unwrap_or("");
    let process_name = Path::new(cmd_part)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("")
        .to_string();

    // Use the full Exec as target
    Some(DiscoveredApp {
        name,
        target: exec_clean,
        process_name,
        source: "linux".to_string(),
    })
}
