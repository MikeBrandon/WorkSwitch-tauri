use crate::config;
use crate::launcher;
use chrono::Datelike;
use std::collections::HashSet;
use tauri::Emitter;

pub fn run_scheduler(app: tauri::AppHandle) {
    let mut last_triggered: HashSet<String> = HashSet::new();
    let mut last_minute: String = String::new();

    loop {
        std::thread::sleep(std::time::Duration::from_secs(30));

        let now = chrono::Local::now();
        let current_time = now.format("%H:%M").to_string();
        let current_day = now.weekday().num_days_from_sunday() as u8;

        // Reset triggers when the minute changes
        if current_time != last_minute {
            last_triggered.clear();
            last_minute = current_time.clone();
        }

        let cfg = config::load_config();

        for profile in &cfg.profiles {
            if let Some(schedule) = &profile.schedule {
                if !schedule.enabled {
                    continue;
                }
                if schedule.time != current_time {
                    continue;
                }
                if !schedule.days.is_empty() && !schedule.days.contains(&current_day) {
                    continue;
                }
                if last_triggered.contains(&profile.id) {
                    continue;
                }

                last_triggered.insert(profile.id.clone());

                // Launch profile steps
                let steps: Vec<_> = profile.steps.iter().filter(|s| s.enabled).cloned().collect();
                let profile_name = profile.name.clone();

                let _ = app.emit(
                    "scheduled-launch",
                    serde_json::json!({ "profile_name": profile_name }),
                );

                for step in &steps {
                    if let Err(e) = launcher::launch_step(step) {
                        eprintln!("Scheduled launch '{}' step '{}' failed: {}", profile_name, step.name, e);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(step.delay_after.max(500)));
                }
            }
        }
    }
}
