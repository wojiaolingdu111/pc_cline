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
            #[cfg(target_os = "windows")]
            add_resource_dir_to_dll_search(app);

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

/// On Windows, add the resource directory to the DLL search path so that
/// LibTorch DLLs bundled via `bundle.resources` can be found at runtime.
#[cfg(target_os = "windows")]
fn add_resource_dir_to_dll_search(app: &tauri::App) {
    use std::os::windows::ffi::OsStrExt;

    // Build a list of directories to search for DLLs
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();

    // 1. Resource directory (where Tauri places bundled resources)
    if let Ok(d) = app.path().resource_dir() {
        dirs.push(d);
    }

    // 2. Executable directory (for dev mode / alternative placement)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            dirs.push(parent.to_path_buf());
        }
    }

    if dirs.is_empty() {
        return;
    }

    extern "system" {
        fn SetDefaultDllDirectories(flags: u32) -> i32;
        fn AddDllDirectory(lpPathName: *const u16) -> *mut std::ffi::c_void;
    }

    const LOAD_LIBRARY_SEARCH_DEFAULT_DIRS: u32 = 0x0000_1000;
    const LOAD_LIBRARY_SEARCH_USER_DIRS: u32 = 0x0000_0400;

    unsafe {
        SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_DEFAULT_DIRS | LOAD_LIBRARY_SEARCH_USER_DIRS);
    }

    for dir in &dirs {
        let wide: Vec<u16> = dir
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            let cookie = AddDllDirectory(wide.as_ptr());
            if cookie.is_null() {
                eprintln!(
                    "warning: AddDllDirectory failed for {}",
                    dir.display()
                );
            }
        }
    }
}
