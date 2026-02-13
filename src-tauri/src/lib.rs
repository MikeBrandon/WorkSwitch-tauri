mod commands;
mod config;
mod discovery;
mod launcher;
mod process;
mod scheduler;
mod tray;

use commands::LaunchState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(LaunchState::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::launch_profile,
            commands::cancel_launch,
            commands::is_process_running,
            commands::kill_process,
            commands::get_running_processes_for_steps,
            commands::browse_file,
            commands::browse_folder,
            commands::scan_apps,
            commands::show_window,
            commands::set_auto_start,
            commands::browse_save_profile,
            commands::browse_import_profile,
            commands::export_profile,
            commands::import_profile,
            commands::save_profile_file,
            commands::load_profile_file,
        ])
        .setup(|app| {
            // Create tray icon
            if let Err(e) = tray::create_tray(app.handle()) {
                eprintln!("Failed to create tray: {}", e);
            }

            // Check start_minimized setting
            let cfg = config::load_config();
            if cfg.settings.start_minimized {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            // Start schedule checker
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                scheduler::run_scheduler(app_handle);
            });

            // Launch startup apps
            if !cfg.startup_apps.is_empty() {
                let startup_apps = cfg.startup_apps.clone();
                std::thread::spawn(move || {
                    for step in &startup_apps {
                        if step.enabled {
                            if let Err(e) = launcher::launch_step(step) {
                                eprintln!("Startup app '{}' failed: {}", step.name, e);
                            }
                        }
                    }
                });
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Check minimize_to_tray setting
                let cfg = config::load_config();
                if cfg.settings.minimize_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
