use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=MPV_PREFIX");

    if let Some(prefix) = detect_mpv_prefix() {
        let lib_dir = prefix.join("lib");
        if lib_dir.exists() {
            println!("cargo:rustc-link-search=native={}", lib_dir.display());
        }
    }

    println!("cargo:rustc-link-lib=dylib=mpv");
}

fn detect_mpv_prefix() -> Option<PathBuf> {
    if let Ok(value) = std::env::var("MPV_PREFIX") {
        let path = PathBuf::from(value);
        if path.exists() {
            return Some(path);
        }
    }

    if let Ok(output) = Command::new("brew").args(["--prefix", "mpv"]).output() {
        if output.status.success() {
            let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !prefix.is_empty() {
                let path = PathBuf::from(prefix);
                if path.exists() {
                    return Some(path);
                }
            }
        }
    }

    let default = PathBuf::from("/usr/local/opt/mpv");
    if default.exists() {
        Some(default)
    } else {
        None
    }
}
