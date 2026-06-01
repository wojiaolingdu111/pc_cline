use std::path::{Path, PathBuf};

fn main() {
    // Must create the resource directory BEFORE tauri_build::build(),
    // otherwise the resources glob validation in tauri.conf.json fails.
    ensure_libtorch_resources();

    // NOTE: DELAYLOAD was attempted here but REJECTED by the linker.
    // LNK1194: MSVC's delay-load cannot handle DLLs that export data symbols
    // (global variables / constants).  torch_cpu.dll and c10.dll export such
    // symbols, so /DELAYLOAD causes a fatal link error.
    //
    // Instead, the build.rs places the DLLs next to the exe (dev mode) and
    // into `libtorch-dlls/` for Tauri bundling (production).  At startup
    // the Windows loader finds them via the standard exe-directory search.
    // The lib.rs `ensure_libtorch_dlls_searchable` serves as a secondary
    // fallback by modifying PATH and copying DLLs on first launch.

    tauri_build::build();
}

// ---------------------------------------------------------------------------
// Cross-platform: ensure libtorch libraries are available for bundling
// ---------------------------------------------------------------------------

fn ensure_libtorch_resources() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    // OUT_DIR = target/<profile>/build/<crate>/out
    let profile_dir = out_dir
        .parent() // out
        .and_then(|p| p.parent()) // build/<crate>
        .and_then(|p| p.parent()); // <profile> (release or debug)

    let profile_dir = match profile_dir {
        Some(p) => p.to_path_buf(),
        None => return,
    };

    // Path for Tauri bundler resources (relative to src-tauri/)
    let resources_dir = profile_dir
        .parent() // target
        .and_then(|p| p.parent()) // src-tauri (where tauri.conf.json is)
        .map(|p| p.join("libtorch-dlls"));

    // Always create the directory so tauri.conf.json resource glob doesn't fail
    if let Some(ref dir) = resources_dir {
        let _ = std::fs::create_dir_all(dir);
    }

    let lib_dir = find_libtorch_lib_dir(&profile_dir);
    let lib_dir = match lib_dir {
        Some(d) => d,
        None => {
            // Dotfile to ensure glob matches at least one entry on non-Windows
            if let Some(ref dir) = resources_dir {
                let _ = std::fs::write(dir.join(".gitkeep"), "");
            }
            println!("cargo:warning=libtorch libraries not found — skipping copy");
            return;
        }
    };

    println!("cargo:warning=Found libtorch at: {}", lib_dir.display());
    let ext = platform_lib_extension();

    // Copy to target/<profile>/ for cargo run / tauri dev
    copy_libs_to(&lib_dir, &profile_dir, ext);

    // Copy to resources dir for Tauri bundler
    if let Some(ref res_dir) = resources_dir {
        copy_libs_to(&lib_dir, res_dir, ext);
    }

    // Dotfile so glob always matches (only matters if no libs were copied)
    if let Some(ref dir) = resources_dir {
        let _ = std::fs::write(dir.join(".gitkeep"), "");
    }
}

fn copy_libs_to(lib_dir: &Path, dest_dir: &Path, ext: &str) {
    for entry in std::fs::read_dir(lib_dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.extension().map_or(false, |e| e == ext) {
            let dest = dest_dir.join(path.file_name().unwrap());
            match std::fs::copy(&path, &dest) {
                Ok(_) => println!("cargo:warning=Copied: {}", dest.display()),
                Err(e) => println!(
                    "cargo:warning=Failed to copy {}: {}",
                    path.display(),
                    e
                ),
            }
        }
    }
}

fn find_libtorch_lib_dir(profile_dir: &Path) -> Option<PathBuf> {
    // ---------- method 1: cargo metadata from torch-sys ----------
    // torch-sys outputs `cargo:libtorch_lib=...` on Linux/macOS → DEP_TORCH_SYS_LIBTORCH_LIB.
    // On Windows torch-sys does NOT emit this metadata, so this only helps non-Windows.
    let dep_keys = [
        "DEP_TORCH_SYS_LIBTORCH_LIB",
        "DEP_TCH_LIBTORCH_LIB",
        "DEP_TORCH_SYS_LIB_DIR",
        "DEP_TORCH_SYS_LIB",
        "DEP_TORCH_CXX11_LIBTORCH_LIB",
    ];
    for key in &dep_keys {
        if let Ok(val) = std::env::var(key) {
            let p = Path::new(&val);
            if p.join(platform_libtorch_marker()).exists() {
                println!("cargo:warning=find_libtorch: found via env {key}={val}");
                return Some(p.to_path_buf());
            }
            // maybe val is the lib dir itself
            let lib_dir = p.join("lib");
            if libtorch_marker_exists(&lib_dir) {
                println!("cargo:warning=find_libtorch: found via env {key}=.../lib");
                return Some(lib_dir);
            }
        }
    }

    // ---------- method 2: LIBTORCH env var ----------
    if let Ok(libtorch) = std::env::var("LIBTORCH") {
        let lib_dir = Path::new(&libtorch).join("lib");
        if libtorch_marker_exists(&lib_dir) {
            println!("cargo:warning=find_libtorch: found via LIBTORCH env var");
            return Some(lib_dir);
        }
    }

    // ---------- method 3: LIBTORCH_LIB env var ----------
    if let Ok(libtorch_lib) = std::env::var("LIBTORCH_LIB") {
        let p = Path::new(&libtorch_lib);
        if p.join(platform_libtorch_marker()).exists() {
            println!("cargo:warning=find_libtorch: found via LIBTORCH_LIB env var");
            return Some(p.to_path_buf());
        }
    }

    // ---------- method 4: Search cargo build directory for torch-sys output ----------
    let build_base = profile_dir.join("build");
    let entries = std::fs::read_dir(&build_base).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Check both torch-sys and tch build directories
        let is_relevant = name_str.starts_with("torch-sys-") || name_str.starts_with("tch-");
        if !is_relevant {
            continue;
        }

        let entry_path = entry.path();

        // Possible directory layouts:
        //   torch-sys with download-libtorch:
        //     out/libtorch/libtorch/lib/   (Linux/macOS/Windows)
        //     out/libtorch/lib/            (alternative)
        //   tch build output might also reference libtorch

        for candidate in [
            entry_path.join("out").join("libtorch").join("libtorch").join("lib"),
            entry_path.join("out").join("libtorch").join("lib"),
            // Also check flat in entry/out/
            entry_path.join("out"),
        ] {
            if libtorch_marker_exists(&candidate) {
                println!(
                    "cargo:warning=find_libtorch: found via {} in {}",
                    name_str,
                    candidate.display()
                );
                return Some(candidate);
            }
        }
    }

    // ---------- method 5: check system-wide locations ----------
    for sys_path in &[
        "/usr/lib/libtorch.so",
        "/usr/local/lib/libtorch.so",
        "/usr/lib/x86_64-linux-gnu/libtorch.so",
    ] {
        let p = Path::new(sys_path);
        if let Some(parent) = p.parent() {
            // On Linux we look for the parent dir; torch_cpu should be there
            if parent.join("libtorch_cpu.so").exists() {
                println!("cargo:warning=find_libtorch: found at system {}", parent.display());
                return Some(parent.to_path_buf());
            }
        }
    }

    println!("cargo:warning=find_libtorch: no libtorch directory found after all methods");
    None
}

fn libtorch_marker_exists(lib_dir: &Path) -> bool {
    let marker = platform_libtorch_marker();
    lib_dir.join(&marker).exists()
}

fn platform_lib_extension() -> &'static str {
    if cfg!(target_os = "windows") {
        "dll"
    } else if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    }
}

fn platform_libtorch_marker() -> &'static str {
    if cfg!(target_os = "windows") {
        "torch_cpu.dll"
    } else if cfg!(target_os = "macos") {
        // macOS may have libtorch.dylib as the main shared library
        // (libtorch_cpu.dylib may not exist as a separate file)
        "libtorch.dylib"
    } else {
        "libtorch_cpu.so"
    }
}
