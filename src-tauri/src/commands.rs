use crate::config::{self, AppConfig, Profile, Step};
use crate::discovery;
use crate::launcher;
use crate::process;
use crate::tray;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Manager, State};

pub struct LaunchState {
    pub cancel_flag: Arc<AtomicBool>,
    pub is_running: AtomicBool,
}

impl Default for LaunchState {
    fn default() -> Self {
        LaunchState {
            cancel_flag: Arc::new(AtomicBool::new(false)),
            is_running: AtomicBool::new(false),
        }
    }
}

#[tauri::command]
pub fn get_config() -> Result<AppConfig, String> {
    Ok(config::load_config())
}

#[tauri::command]
pub fn save_config(config: AppConfig, app: tauri::AppHandle) -> Result<(), String> {
    config::save_config(&config)?;
    // Rebuild tray menu to reflect profile changes
    let _ = tray::rebuild_tray_menu(&app, &config);
    Ok(())
}

#[tauri::command]
pub async fn launch_profile(
    steps: Vec<Step>,
    default_delay: u64,
    state: State<'_, LaunchState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    if state
        .is_running
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("Launch already in progress".to_string());
    }

    state.cancel_flag.store(false, Ordering::SeqCst);
    let cancel_flag = state.cancel_flag.clone();

    let total = steps.len();
    for (i, step) in steps.iter().enumerate() {
        // Check cancel
        if cancel_flag.load(Ordering::SeqCst) {
            let _ = app.emit("launch-cancelled", ());
            state.is_running.store(false, Ordering::SeqCst);
            return Ok(());
        }

        // Emit progress
        let _ = app.emit(
            "launch-progress",
            serde_json::json!({
                "step_name": step.name,
                "current": i + 1,
                "total": total
            }),
        );

        // Launch the step in a blocking task with timeout so it can't freeze us
        let step_clone = step.clone();
        let step_name = step.name.clone();
        let cancel = cancel_flag.clone();

        let launch_result = tokio::select! {
            result = tokio::task::spawn_blocking(move || {
                launcher::launch_step(&step_clone)
            }) => {
                match result {
                    Ok(inner) => inner,
                    Err(e) => Err(format!("Task panicked: {}", e)),
                }
            }
            _ = cancel_wait(cancel) => {
                let _ = app.emit("launch-cancelled", ());
                state.is_running.store(false, Ordering::SeqCst);
                return Ok(());
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(15)) => {
                Err("Step timed out after 15s".to_string())
            }
        };

        if let Err(e) = launch_result {
            eprintln!("Step '{}' failed: {}", step_name, e);
            // Emit error but continue
            let _ = app.emit(
                "launch-step-error",
                serde_json::json!({
                    "step_name": step_name,
                    "error": e
                }),
            );
        }

        // Delay after step (check cancel every 100ms)
        let delay = step.delay_after.max(default_delay);
        if delay > 0 {
            let mut remaining = delay;
            while remaining > 0 {
                if cancel_flag.load(Ordering::SeqCst) {
                    let _ = app.emit("launch-cancelled", ());
                    state.is_running.store(false, Ordering::SeqCst);
                    return Ok(());
                }
                let sleep_ms = remaining.min(100);
                tokio::time::sleep(tokio::time::Duration::from_millis(sleep_ms)).await;
                remaining = remaining.saturating_sub(sleep_ms);
            }
        }
    }

    let _ = app.emit("launch-complete", ());
    state.is_running.store(false, Ordering::SeqCst);
    Ok(())
}

/// Polls the cancel flag every 50ms, resolves when cancelled.
async fn cancel_wait(flag: Arc<AtomicBool>) {
    loop {
        if flag.load(Ordering::SeqCst) {
            return;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
}

#[tauri::command]
pub fn cancel_launch(state: State<'_, LaunchState>) -> Result<(), String> {
    state.cancel_flag.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub async fn is_process_running(name: String) -> bool {
    let result = tokio::time::timeout(
        tokio::time::Duration::from_secs(5),
        tokio::task::spawn_blocking(move || process::is_running(&name)),
    )
    .await;

    match result {
        Ok(Ok(val)) => val,
        _ => false, // timeout or error = assume not running
    }
}

#[tauri::command]
pub async fn kill_process(name: String) -> Result<(), String> {
    let result = tokio::time::timeout(
        tokio::time::Duration::from_secs(5),
        tokio::task::spawn_blocking(move || process::kill_process(&name)),
    )
    .await;

    match result {
        Ok(Ok(inner)) => inner,
        Ok(Err(e)) => Err(format!("Kill task failed: {}", e)),
        Err(_) => Err("Kill timed out".to_string()),
    }
}

#[tauri::command]
pub async fn get_running_processes_for_steps(process_names: Vec<String>) -> Vec<String> {
    let result = tokio::time::timeout(
        tokio::time::Duration::from_secs(5),
        tokio::task::spawn_blocking(move || {
            let running = process::get_running_processes();
            process_names
                .into_iter()
                .filter(|name| running.contains(&name.to_lowercase()))
                .collect()
        }),
    )
    .await;

    match result {
        Ok(Ok(list)) => list,
        _ => vec![], // timeout or error = return empty
    }
}

#[tauri::command]
pub async fn browse_file(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let mut dialog = app.dialog().file();

    #[cfg(target_os = "windows")]
    {
        dialog = dialog.add_filter("Executables", &["exe", "bat", "cmd", "lnk"]);
    }

    #[cfg(target_os = "macos")]
    {
        dialog = dialog.add_filter("Applications", &["app", "sh", "command"]);
    }

    #[cfg(target_os = "linux")]
    {
        dialog = dialog.add_filter("Executables", &["sh", "desktop", "AppImage"]);
    }

    let file = dialog.add_filter("All Files", &["*"]).blocking_pick_file();

    Ok(file.map(|f| f.to_string()))
}

#[tauri::command]
pub async fn browse_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let folder = app.dialog().file().blocking_pick_folder();

    Ok(folder.map(|f| f.to_string()))
}

#[tauri::command]
pub async fn scan_apps() -> Vec<discovery::DiscoveredApp> {
    tokio::task::spawn_blocking(|| discovery::scan_all())
        .await
        .unwrap_or_default()
}

#[tauri::command]
pub fn set_auto_start(enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let run_key = hkcu
            .open_subkey_with_flags(
                r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run",
                KEY_WRITE,
            )
            .map_err(|e| format!("Failed to open Run key: {}", e))?;

        if enabled {
            let exe_path = std::env::current_exe()
                .map_err(|e| format!("Failed to get exe path: {}", e))?;
            run_key
                .set_value("WorkSwitch", &exe_path.to_string_lossy().to_string())
                .map_err(|e| format!("Failed to set registry value: {}", e))?;
        } else {
            let _ = run_key.delete_value("WorkSwitch");
        }
    }

    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir()
            .ok_or_else(|| "Could not find home directory".to_string())?;
        let plist_path = home.join("Library/LaunchAgents/com.workswitch.app.plist");

        if enabled {
            let exe_path = std::env::current_exe()
                .map_err(|e| format!("Failed to get exe path: {}", e))?;

            let plist_content = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.workswitch.app</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>"#,
                exe_path.display()
            );

            // Ensure LaunchAgents directory exists
            if let Some(parent) = plist_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            std::fs::write(&plist_path, plist_content)
                .map_err(|e| format!("Failed to write LaunchAgent plist: {}", e))?;
        } else {
            let _ = std::fs::remove_file(&plist_path);
        }
    }

    #[cfg(target_os = "linux")]
    {
        let home = dirs::home_dir()
            .ok_or_else(|| "Could not find home directory".to_string())?;
        let autostart_dir = home.join(".config/autostart");
        let desktop_path = autostart_dir.join("workswitch.desktop");

        if enabled {
            let exe_path = std::env::current_exe()
                .map_err(|e| format!("Failed to get exe path: {}", e))?;

            let desktop_content = format!(
                "[Desktop Entry]\n\
                 Type=Application\n\
                 Name=WorkSwitch\n\
                 Exec={}\n\
                 X-GNOME-Autostart-enabled=true\n\
                 Hidden=false\n",
                exe_path.display()
            );

            let _ = std::fs::create_dir_all(&autostart_dir);
            std::fs::write(&desktop_path, desktop_content)
                .map_err(|e| format!("Failed to write autostart desktop file: {}", e))?;
        } else {
            let _ = std::fs::remove_file(&desktop_path);
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn browse_save_profile(
    default_name: String,
    app: tauri::AppHandle,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let file = app
        .dialog()
        .file()
        .set_file_name(&default_name)
        .add_filter("JSON", &["json"])
        .blocking_save_file();

    Ok(file.map(|f| f.to_string()))
}

#[tauri::command]
pub async fn browse_import_profile(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let file = app
        .dialog()
        .file()
        .add_filter("JSON", &["json"])
        .blocking_pick_file();

    Ok(file.map(|f| f.to_string()))
}

#[tauri::command]
pub fn export_profile(profile_id: String) -> Result<String, String> {
    let cfg = config::load_config();
    let profile = cfg
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| "Profile not found".to_string())?;

    serde_json::to_string_pretty(profile).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_profile(json: String) -> Result<Profile, String> {
    serde_json::from_str(&json).map_err(|e| format!("Invalid profile JSON: {}", e))
}

#[tauri::command]
pub fn save_profile_file(profile_id: String, path: String) -> Result<(), String> {
    let json = export_profile(profile_id)?;
    std::fs::write(&path, &json).map_err(|e| format!("Failed to write file: {}", e))
}

#[tauri::command]
pub fn load_profile_file(path: String) -> Result<Profile, String> {
    let json = std::fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))?;
    import_profile(json)
}

#[tauri::command]
pub fn show_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    Ok(())
}
