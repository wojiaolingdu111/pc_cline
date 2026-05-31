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
            ensure_libtorch_dlls_searchable(app);

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

/// Ensure LibTorch DLLs (c10.dll, torch_cpu.dll, …) are findable at runtime.
///
/// On Windows, DLL search is notoriously brittle after `SetDefaultDllDirectories`.
/// This function uses a belt-and-suspenders approach:
///   1. `SetDefaultDllDirectories` + `AddDllDirectory` — clean modern approach.
///   2. `PATH` environment variable — guaranteed fallback that works even when
///      security software blocks `AddDllDirectory`.
#[cfg(target_os = "windows")]
fn ensure_libtorch_dlls_searchable(app: &tauri::App) {
    use std::os::windows::ffi::OsStrExt;

    // ---------- collect candidate directories ----------
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();

    // App resource directory (where Tauri places bundled resources)
    if let Ok(d) = app.path().resource_dir() {
        dirs.push(d);
    }
    // Executable directory (dev mode, or where DLLs land with ".." target)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            dirs.push(parent.to_path_buf());
        }
    }

    if dirs.is_empty() {
        return;
    }

    // ---------- method 1: AddDllDirectory ----------
    extern "system" {
        fn SetDefaultDllDirectories(flags: u32) -> i32;
        fn AddDllDirectory(lpPathName: *const u16) -> *mut std::ffi::c_void;
    }

    const LOAD_LIBRARY_SEARCH_APPLICATION_DIR: u32 = 0x0000_0200;
    const LOAD_LIBRARY_SEARCH_DEFAULT_DIRS: u32 = 0x0000_1000;
    const LOAD_LIBRARY_SEARCH_USER_DIRS: u32 = 0x0000_0400;

    unsafe {
        SetDefaultDllDirectories(
            LOAD_LIBRARY_SEARCH_APPLICATION_DIR
                | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS
                | LOAD_LIBRARY_SEARCH_USER_DIRS,
        );
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
                eprintln!("[warn] AddDllDirectory failed for {}", dir.display());
            } else {
                eprintln!("[info] AddDllDirectory OK: {}", dir.display());
            }
        }
    }

    // ---------- method 2: PATH environment variable (guaranteed fallback) ----------
    let paths_to_add: Vec<String> = dirs
        .iter()
        .filter_map(|d| d.to_str())
        .map(|s| s.to_owned())
        .collect();

    if !paths_to_add.is_empty() {
        let current_path = std::env::var("PATH").unwrap_or_default();
        let mut parts: Vec<&str> = current_path.split(';').collect();

        for p in &paths_to_add {
            if !parts.contains(&p.as_str()) {
                parts.insert(0, p);
            }
        }

        let new_path = parts.join(";");
        std::env::set_var("PATH", &new_path);
        eprintln!("[info] PATH updated with LibTorch DLL directories");
    }
}
