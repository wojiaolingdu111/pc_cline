fn main() {
    tauri_build::build();

    #[cfg(target_os = "windows")]
    copy_libtorch_dlls();
}

#[cfg(target_os = "windows")]
fn copy_libtorch_dlls() {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    // OUT_DIR = target/<profile>/build/<crate>/out
    let profile_dir = out_dir
        .parent() // out
        .and_then(|p| p.parent()) // build/<crate>
        .and_then(|p| p.parent()); // <profile> (release or debug)

    let profile_dir = match profile_dir {
        Some(p) => p.to_path_buf(),
        None => return,
    };

    let lib_dir = find_libtorch_lib_dir(&profile_dir);
    let lib_dir = match lib_dir {
        Some(d) => d,
        None => {
            println!("cargo:warning=libtorch DLLs not found — skipping copy");
            return;
        }
    };

    println!("cargo:warning=Found libtorch DLLs at: {}", lib_dir.display());

    // Copy to target/<profile>/ for cargo run / tauri dev
    copy_dlls_to(&lib_dir, &profile_dir);

    // Copy to src-tauri/libtorch-dlls/ for Tauri bundler resources
    let resources_dir = profile_dir
        .parent() // target
        .and_then(|p| p.parent()) // project root (src-tauri's parent)
        .map(|p| p.join("src-tauri").join("libtorch-dlls"));
    if let Some(res_dir) = resources_dir {
        let _ = std::fs::create_dir_all(&res_dir);
        copy_dlls_to(&lib_dir, &res_dir);
    }
}

#[cfg(target_os = "windows")]
fn copy_dlls_to(lib_dir: &std::path::Path, dest_dir: &std::path::Path) {
    for entry in std::fs::read_dir(lib_dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "dll") {
            let dest = dest_dir.join(path.file_name().unwrap());
            match std::fs::copy(&path, &dest) {
                Ok(_) => println!("cargo:warning=Copied DLL: {}", dest.display()),
                Err(e) => println!(
                    "cargo:warning=Failed to copy {}: {}",
                    path.display(),
                    e
                ),
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn find_libtorch_lib_dir(profile_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    // 1. Check LIBTORCH env var
    if let Ok(libtorch) = std::env::var("LIBTORCH") {
        let lib_dir = std::path::Path::new(&libtorch).join("lib");
        if lib_dir.join("torch_cpu.dll").exists() {
            return Some(lib_dir);
        }
    }

    // 2. Search the cargo build directory for torch-sys output
    let build_base = profile_dir.join("build");
    let entries = std::fs::read_dir(&build_base).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with("torch-sys-") {
            // torch-sys with download-libtorch extracts to out/libtorch/libtorch/lib/
            let candidate = entry
                .path()
                .join("out")
                .join("libtorch")
                .join("libtorch")
                .join("lib");
            if candidate.join("torch_cpu.dll").exists() {
                return Some(candidate);
            }

            // Alternative: out/libtorch/lib/
            let candidate2 = entry.path().join("out").join("libtorch").join("lib");
            if candidate2.join("torch_cpu.dll").exists() {
                return Some(candidate2);
            }
        }
    }

    None
}
