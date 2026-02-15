use crate::config::AppConfig;
use crate::lifecycle;
use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    Emitter, Manager,
};

fn create_default_icon() -> Image<'static> {
    // Create a simple 32x32 RGBA icon (blue square)
    let size = 32u32;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for _ in 0..size * size {
        rgba.extend_from_slice(&[37, 99, 235, 255]); // #2563eb blue
    }
    Image::new_owned(rgba, size, size)
}

pub fn create_tray(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::config::load_config();
    let menu = build_tray_menu(app, &config)?;

    let _tray = TrayIconBuilder::with_id("main")
        .icon(create_default_icon())
        .menu(&menu)
        .tooltip("WorkSwitch")
        .on_menu_event(move |app, event| {
            let id = event.id().as_ref();
            if id == "show" {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            } else if id == "quit" {
                lifecycle::close_apps_on_exit(app);
                app.exit(0);
            } else if let Some(profile_id) = id.strip_prefix("profile-") {
                let _ = app.emit("tray-launch-profile", profile_id.to_string());
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

fn build_tray_menu(
    app: &tauri::AppHandle,
    config: &AppConfig,
) -> Result<tauri::menu::Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    let mut builder = MenuBuilder::new(app);

    // Show WorkSwitch
    let show_item = MenuItemBuilder::with_id("show", "Show WorkSwitch").build(app)?;
    builder = builder.item(&show_item).separator();

    // Profile items
    for profile in &config.profiles {
        let item = MenuItemBuilder::with_id(
            format!("profile-{}", profile.id),
            format!("Launch: {}", profile.name),
        )
        .build(app)?;
        builder = builder.item(&item);
    }

    // Quit
    let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
    builder = builder.separator().item(&quit_item);

    Ok(builder.build()?)
}

pub fn rebuild_tray_menu(
    app: &tauri::AppHandle,
    config: &AppConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let menu = build_tray_menu(app, config)?;
    // Get the existing tray icon and update its menu
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(menu))?;
    }
    Ok(())
}
