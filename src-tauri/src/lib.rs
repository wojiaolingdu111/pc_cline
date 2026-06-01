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
/// On Windows, `torch_cpu.dll` and `c10.dll` are loaded at **process startup**
/// via the import table (from `torch-sys`). They MUST be in the standard
/// Windows DLL search path before `main()` — which means either:
///   - **Dev mode**: the exe directory (`target/<profile>/`), where
///     `build.rs` copies them.
///   - **Production mode**: the install directory, where the MSI/NSIS
///     installer (via `bundle.resources` in `tauri.conf.json`) places them.
///
/// What this function adds as a safety net:
///   1. Prepend exe + resource dirs to `PATH` — helps if the DLLs happen
///      to be somewhere else (e.g. a manual install).
///   2. Emergency copy — if resource_dir ≠ exe_dir, copy DLLs from
///      resource_dir to exe_dir.  Next launch will find them directly.
#[cfg(target_os = "windows")]
fn ensure_libtorch_dlls_searchable(app: &tauri::App) {
    // ---------- collect candidate directories ----------
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();

    // Executable directory — the most reliable location
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            dirs.push(parent.to_path_buf());
        }
    }

    // App resource directory (where Tauri places bundled resources)
    if let Ok(d) = app.path().resource_dir() {
        dirs.push(d);
    }

    if dirs.is_empty() {
        return;
    }

    // ---------- method: PATH environment variable ----------
    // Prepend our directories to PATH.  The standard LoadLibraryEx search
    // includes PATH, so the delay-load helper (used by DELAYLOAD) will
    // find the DLLs here.
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

    // ---------- emergency copy to exe dir ----------
    // If resources are in a separate directory (production install), copy
    // DLLs from resource dir to exe dir.  With DELAYLOAD the DLL hasn't
    // been loaded yet, so the copy happens before the first torch call,
    // AND subsequent launches find them directly in the exe dir.
    if dirs.len() >= 2 {
        let exe_dir = &dirs[0];
        let res_dir = &dirs[1];
        if exe_dir != res_dir && res_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(res_dir) {
                let mut copied = false;
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |e| e == "dll") {
                        let dest = exe_dir.join(path.file_name().unwrap());
                        if !dest.exists() && std::fs::copy(&path, &dest).is_ok() {
                            eprintln!("[info] Copied {} to exe dir", dest.display());
                            copied = true;
                        }
                    }
                }
                if copied {
                    let new_path = format!(
                        "{}{}{}",
                        exe_dir.display(),
                        ";",
                        std::env::var("PATH").unwrap_or_default()
                    );
                    std::env::set_var("PATH", &new_path);
                    eprintln!("[info] PATH re-updated after DLL copy");
                }
            }
        }
    }
}
