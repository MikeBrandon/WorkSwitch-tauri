use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredApp {
    pub name: String,
    pub target: String,
    pub process_name: String,
    pub source: String,
}

pub fn scan_all() -> Vec<DiscoveredApp> {
    #[cfg(target_os = "windows")]
    {
        return scan_all_windows();
    }

    #[cfg(not(target_os = "windows"))]
    {
        Vec::new()
    }
}

#[cfg(target_os = "windows")]
fn scan_all_windows() -> Vec<DiscoveredApp> {
    use std::collections::HashSet;

    let mut apps = Vec::new();
    let mut seen = HashSet::new();

    for app in scan_steam() {
        add_unique(&mut apps, &mut seen, app);
    }
    for app in scan_epic() {
        add_unique(&mut apps, &mut seen, app);
    }
    for app in scan_windows() {
        add_unique(&mut apps, &mut seen, app);
    }

    apps
}

#[cfg(target_os = "windows")]
fn add_unique(apps: &mut Vec<DiscoveredApp>, seen: &mut std::collections::HashSet<String>, app: DiscoveredApp) {
    let key = if !app.target.is_empty() {
        format!("t:{}", app.target.to_lowercase())
    } else {
        format!("n:{}:{}", app.source, app.name.to_lowercase())
    };
    if seen.insert(key) {
        apps.push(app);
    }
}

#[cfg(target_os = "windows")]
fn scan_steam() -> Vec<DiscoveredApp> {
    use std::fs;

    let mut results = Vec::new();
    for lib in steam_library_paths() {
        let steamapps = lib.join("steamapps");
        if !steamapps.is_dir() {
            continue;
        }
        let entries = match fs::read_dir(&steamapps) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = match path.file_name().and_then(|s| s.to_str()) {
                Some(name) => name,
                None => continue,
            };
            if !file_name.starts_with("appmanifest_") || !file_name.ends_with(".acf") {
                continue;
            }
            let contents = match fs::read_to_string(&path) {
                Ok(contents) => contents,
                Err(_) => continue,
            };
            let (appid, name) = match parse_steam_manifest(&contents) {
                Some(values) => values,
                None => continue,
            };

            let target = format!("steam://rungameid/{}", appid);
            results.push(DiscoveredApp {
                name,
                target,
                process_name: String::new(),
                source: "steam".to_string(),
            });
        }
    }

    results
}

#[cfg(target_os = "windows")]
fn steam_library_paths() -> Vec<std::path::PathBuf> {
    use std::collections::HashSet;
    use std::fs;
    use std::path::PathBuf;
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let mut paths = Vec::new();
    let mut seen = HashSet::new();

    if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey("Software\\Valve\\Steam") {
        if let Ok(raw_path) = hkcu.get_value::<String, _>("SteamPath") {
            let normalized = raw_path.replace('/', "\\");
            let path = PathBuf::from(normalized);
            if path.is_dir() && seen.insert(path.clone()) {
                paths.push(path);
            }
        }
    }

    let fallback_paths = [
        r"C:\Program Files (x86)\Steam",
        r"C:\Program Files\Steam",
    ];
    for fallback in fallback_paths {
        let path = PathBuf::from(fallback);
        if path.is_dir() && seen.insert(path.clone()) {
            paths.push(path);
        }
    }

    let steam_path = paths.get(0).cloned();
    if let Some(steam_root) = steam_path {
        let vdf_path = steam_root.join("steamapps").join("libraryfolders.vdf");
        if let Ok(contents) = fs::read_to_string(&vdf_path) {
            for line in contents.lines() {
                if let Some((key, value)) = parse_vdf_kv(line) {
                    if key == "path" {
                        let mut normalized = value.replace("\\\\", "\\");
                        normalized = normalized.replace('/', "\\");
                        let path = PathBuf::from(normalized);
                        if path.is_dir() && seen.insert(path.clone()) {
                            paths.push(path);
                        }
                    }
                }
            }
        }
    }

    paths
}

#[cfg(target_os = "windows")]
fn parse_steam_manifest(contents: &str) -> Option<(String, String)> {
    let mut appid: Option<String> = None;
    let mut name: Option<String> = None;

    for line in contents.lines() {
        if let Some((key, value)) = parse_vdf_kv(line) {
            match key.as_str() {
                "appid" => appid = Some(value),
                "name" => name = Some(value),
                _ => {}
            }
        }
        if appid.is_some() && name.is_some() {
            break;
        }
    }

    match (appid, name) {
        (Some(id), Some(n)) if !id.is_empty() && !n.is_empty() => Some((id, n)),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
fn parse_vdf_kv(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    if !line.starts_with('"') {
        return None;
    }

    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in line.chars() {
        if ch == '"' {
            if in_quotes {
                parts.push(current.clone());
                current.clear();
            }
            in_quotes = !in_quotes;
        } else if in_quotes {
            current.push(ch);
        }
    }

    if parts.len() >= 2 {
        Some((parts[0].clone(), parts[1].clone()))
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
fn scan_epic() -> Vec<DiscoveredApp> {
    use std::fs;
    use std::path::PathBuf;

    #[derive(Debug, Deserialize)]
    struct EpicManifest {
        #[serde(rename = "DisplayName")]
        display_name: Option<String>,
        #[serde(rename = "AppName")]
        app_name: Option<String>,
        #[serde(rename = "InstallLocation")]
        install_location: Option<String>,
        #[serde(rename = "LaunchExecutable")]
        launch_executable: Option<String>,
        #[serde(rename = "LaunchCommand")]
        launch_command: Option<String>,
    }

    let program_data = std::env::var("PROGRAMDATA").unwrap_or_else(|_| "C:\\ProgramData".to_string());
    let manifest_dir = PathBuf::from(program_data)
        .join("Epic")
        .join("EpicGamesLauncher")
        .join("Data")
        .join("Manifests");

    let mut results = Vec::new();

    let entries = match fs::read_dir(&manifest_dir) {
        Ok(entries) => entries,
        Err(_) => return results,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()).unwrap_or("") != "item" {
            continue;
        }

        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(_) => continue,
        };

        let manifest: EpicManifest = match serde_json::from_str(&contents) {
            Ok(manifest) => manifest,
            Err(_) => continue,
        };

        let name = manifest
            .display_name
            .clone()
            .or(manifest.app_name.clone())
            .unwrap_or_default();
        if name.is_empty() {
            continue;
        }

        let mut target: Option<String> = None;
        let mut process_name = String::new();

        if let (Some(install), Some(exec)) = (manifest.install_location.clone(), manifest.launch_executable.clone()) {
            let exe_path = if PathBuf::from(&exec).is_absolute() {
                PathBuf::from(exec)
            } else {
                let mut path = PathBuf::from(install);
                path.push(exec);
                path
            };
            if exe_path.exists() {
                target = Some(exe_path.to_string_lossy().to_string());
                process_name = process_name_from_path(target.as_deref().unwrap_or(""));
            }
        }

        if target.is_none() {
            if let Some(cmd) = manifest.launch_command.clone() {
                if cmd.contains("com.epicgames.launcher://") {
                    target = Some(cmd);
                }
            }
        }

        if target.is_none() {
            if let Some(app_name) = manifest.app_name.clone() {
                target = Some(format!(
                    "com.epicgames.launcher://apps/{}?action=launch&silent=true",
                    app_name
                ));
            }
        }

        if let Some(target) = target {
            results.push(DiscoveredApp {
                name,
                target,
                process_name,
                source: "epic".to_string(),
            });
        }
    }

    results
}

#[cfg(target_os = "windows")]
fn scan_windows() -> Vec<DiscoveredApp> {
    use std::path::Path;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    let mut results = Vec::new();
    let uninstall_paths = [
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
        (
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        ),
        (HKEY_CURRENT_USER, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
    ];

    for (root, path) in uninstall_paths {
        let root = RegKey::predef(root);
        let key = match root.open_subkey(path) {
            Ok(key) => key,
            Err(_) => continue,
        };

        for subkey_name in key.enum_keys().flatten() {
            let subkey = match key.open_subkey(&subkey_name) {
                Ok(subkey) => subkey,
                Err(_) => continue,
            };

            if let Ok(component) = subkey.get_value::<u32, _>("SystemComponent") {
                if component == 1 {
                    continue;
                }
            }

            let display_name: String = match subkey.get_value("DisplayName") {
                Ok(name) => name,
                Err(_) => continue,
            };
            if display_name.trim().is_empty() {
                continue;
            }

            let display_icon: Option<String> = subkey.get_value("DisplayIcon").ok();
            let target = display_icon
                .and_then(|icon| extract_exe_path(&icon))
                .and_then(|path| {
                    let p = Path::new(&path);
                    if p.exists() { Some(path) } else { None }
                });

            let target = match target {
                Some(target) => target,
                None => continue,
            };

            let process_name = process_name_from_path(&target);

            results.push(DiscoveredApp {
                name: display_name,
                target,
                process_name,
                source: "windows".to_string(),
            });
        }
    }

    results
}

#[cfg(target_os = "windows")]
fn process_name_from_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase()
}

#[cfg(target_os = "windows")]
fn extract_exe_path(raw: &str) -> Option<String> {
    let expanded = expand_env_vars(raw);
    let mut text = expanded.trim().to_string();
    if text.is_empty() {
        return None;
    }

    if text.starts_with('"') {
        if let Some(end) = text[1..].find('"') {
            let end_idx = 1 + end;
            text = text[1..end_idx].to_string();
        }
    }

    let lower = text.to_lowercase();
    if let Some(idx) = lower.find(".exe") {
        let path = text[..idx + 4].trim().to_string();
        if path.is_empty() {
            None
        } else {
            Some(path)
        }
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
fn expand_env_vars(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            let mut var = String::new();
            while let Some(&next) = chars.peek() {
                chars.next();
                if next == '%' {
                    break;
                }
                var.push(next);
            }
            if var.is_empty() {
                result.push('%');
            } else if let Ok(value) = std::env::var(&var) {
                result.push_str(&value);
            } else {
                result.push('%');
                result.push_str(&var);
                result.push('%');
            }
        } else {
            result.push(ch);
        }
    }

    result
}
