mod commands;
mod file_manager;
mod license;
mod state;

use anyhow::Result;
use state::AppState;
use tauri::Manager;

fn build_app_state(app_handle: &tauri::AppHandle) -> Result<AppState> {
    AppState::new(app_handle)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let state = build_app_state(app.handle())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::generate_speech,
            commands::list_voices,
            commands::clone_voice,
            commands::delete_voice_profile,
            commands::get_service_status,
            commands::pick_audio_file,
            commands::get_license_status,
            commands::activate_license,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
