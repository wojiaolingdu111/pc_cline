use std::path::{Path, PathBuf};

fn main() {
    tauri_build::build();

    copy_libtorch_libs();
}

// ---------------------------------------------------------------------------
// Cross-platform: find libtorch libraries and copy them for bundling
// ---------------------------------------------------------------------------

fn copy_libtorch_libs() {
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
        .and_then(|p| p.parent()) // project root
        .map(|p| p.join("src-tauri").join("libtorch-dlls"));

    // Always create the directory so tauri.conf.json resource glob doesn't fail
    if let Some(ref dir) = resources_dir {
        let _ = std::fs::create_dir_all(dir);
    }

    let lib_dir = find_libtorch_lib_dir(&profile_dir);
    let lib_dir = match lib_dir {
        Some(d) => d,
        None => {
            println!("cargo:warning=libtorch libraries not found — skipping copy");
            return;
        }
    };

    println!("cargo:warning=Found libtorch at: {}", lib_dir.display());

    // Copy to target/<profile>/ for cargo run / tauri dev
    copy_libs_to(
        &lib_dir,
        &profile_dir,
        platform_lib_extension(),
    );

    // Copy to resources dir for Tauri bundler
    if let Some(ref res_dir) = resources_dir {
        copy_libs_to(
            &lib_dir,
            res_dir,
            platform_lib_extension(),
        );
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
    // 1. Check LIBTORCH env var
    if let Ok(libtorch) = std::env::var("LIBTORCH") {
        let lib_dir = Path::new(&libtorch).join("lib");
        if libtorch_marker_exists(&lib_dir) {
            return Some(lib_dir);
        }
    }

    // 2. Search the cargo build directory for torch-sys output
    let build_base = profile_dir.join("build");
    let entries = std::fs::read_dir(&build_base).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        if !name.to_string_lossy().starts_with("torch-sys-") {
            continue;
        }

        // torch-sys with download-libtorch extracts to out/libtorch/libtorch/lib/
        let candidate = entry
            .path()
            .join("out")
            .join("libtorch")
            .join("libtorch")
            .join("lib");
        if libtorch_marker_exists(&candidate) {
            return Some(candidate);
        }

        // Alternative: out/libtorch/lib/
        let candidate2 = entry.path().join("out").join("libtorch").join("lib");
        if libtorch_marker_exists(&candidate2) {
            return Some(candidate2);
        }
    }

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
        "libtorch_cpu.dylib"
    } else {
        "libtorch_cpu.so"
    }
}
