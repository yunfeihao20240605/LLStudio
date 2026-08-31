use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=MPV_PREFIX");
    println!("cargo:rerun-if-env-changed=TARGET");

    if let Some(prefix) = detect_mpv_prefix() {
        // Homebrew uses <prefix>/lib. The Windows MSVC build generates
        // mpv.lib beside the downloaded MinGW import archive.
        for directory in [prefix.clone(), prefix.join("lib")] {
            if directory.exists() {
                println!("cargo:rustc-link-search=native={}", directory.display());
            }
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

    if std::env::var("TARGET")
        .map(|target| target.contains("apple-darwin"))
        .unwrap_or(false)
    {
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
    }

    if std::env::var("TARGET")
        .map(|target| target.contains("apple-darwin"))
        .unwrap_or(false)
    {
        for default in ["/opt/homebrew/opt/mpv", "/usr/local/opt/mpv"] {
            let path = PathBuf::from(default);
            if path.exists() {
                return Some(path);
            }
        }
    }

    None
}
