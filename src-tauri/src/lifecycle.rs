use crate::commands::LastLaunch;
use crate::config;
use crate::process;
use tauri::Manager;

pub fn close_apps_on_exit(app: &tauri::AppHandle) {
    let cfg = config::load_config();
    if !cfg.settings.close_on_exit {
        return;
    }

    let state = app.state::<LastLaunch>();
    let process_names = state.get_processes();
    if process_names.is_empty() {
        return;
    }

    for name in process_names {
        let _ = process::kill_process(&name);
    }
}
