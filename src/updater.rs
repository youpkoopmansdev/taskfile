use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime};

use colored::Colorize;

const REPO: &str = "youpkoopmansdev/taskfile";
const CHECK_INTERVAL: Duration = Duration::from_secs(86400); // once per day

pub fn check_for_update_background() {
    if should_check() {
        // Spawn check in a thread so it doesn't slow down task execution
        std::thread::spawn(|| {
            if let Some(latest) = fetch_latest_version() {
                let current = env!("CARGO_PKG_VERSION");
                if version_newer(&latest, current) {
                    eprintln!(
                        "\n{} A new version of Task is available: {} → {}",
                        "update:".yellow().bold(),
                        current.dimmed(),
                        latest.green().bold()
                    );
                    eprintln!("  Run {} to update.\n", "task --update".green());
                }
            }
            touch_check_file();
        });
    }
}

pub fn self_update(version: Option<&str>) {
    let target_version = if let Some(v) = version {
        let v = v.to_string();
        if v.starts_with('v') {
            v
        } else {
            format!("v{v}")
        }
    } else {
        match fetch_latest_version() {
            Some(v) => format!("v{v}"),
            None => {
                eprintln!(
                    "{} Could not fetch latest version from GitHub.",
                    "error:".red().bold()
                );
                std::process::exit(1);
            }
        }
    };

    let current = env!("CARGO_PKG_VERSION");
    let target_bare = target_version.strip_prefix('v').unwrap_or(&target_version);

    if target_bare == current {
        println!("Already on version {}.", current.green());
        return;
    }

    println!(
        "Updating Task: {} → {}",
        current.dimmed(),
        target_bare.green().bold()
    );

    let platform = detect_platform();
    let arch = detect_arch();
    let filename = format!("task-{platform}-{arch}.tar.gz");
    let url = format!("https://github.com/{REPO}/releases/download/{target_version}/{filename}");

    let current_exe = env::current_exe().unwrap_or_else(|e| {
        eprintln!(
            "{} Cannot determine binary path: {e}",
            "error:".red().bold()
        );
        std::process::exit(1);
    });

    let tmp_dir = env::temp_dir().join("task-update");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    // Download
    let tar_path = tmp_dir.join(&filename);
    let status = Command::new("curl")
        .args(["-fsSL", &url, "-o"])
        .arg(&tar_path)
        .status();

    match status {
        Ok(s) if s.success() => {}
        _ => {
            eprintln!(
                "{} Failed to download {}. Does this version exist?",
                "error:".red().bold(),
                target_version
            );
            let _ = fs::remove_dir_all(&tmp_dir);
            std::process::exit(1);
        }
    }

    // Extract
    let status = Command::new("tar")
        .args(["-xzf"])
        .arg(&tar_path)
        .arg("-C")
        .arg(&tmp_dir)
        .status();

    match status {
        Ok(s) if s.success() => {}
        _ => {
            eprintln!("{} Failed to extract archive.", "error:".red().bold());
            let _ = fs::remove_dir_all(&tmp_dir);
            std::process::exit(1);
        }
    }

    // Replace binary
    let new_bin = tmp_dir.join("task");
    if let Err(e) = fs::copy(&new_bin, &current_exe) {
        // Try with sudo
        eprintln!(
            "Needs elevated permissions to replace {}...",
            current_exe.display()
        );
        let status = Command::new("sudo")
            .arg("cp")
            .arg(&new_bin)
            .arg(&current_exe)
            .status();

        match status {
            Ok(s) if s.success() => {}
            _ => {
                eprintln!("{} Failed to replace binary: {e}", "error:".red().bold());
                let _ = fs::remove_dir_all(&tmp_dir);
                std::process::exit(1);
            }
        }
    }

    let _ = fs::remove_dir_all(&tmp_dir);
    touch_check_file();

    println!("✓ Updated to Task {}.", target_bare.green().bold());
}

fn fetch_latest_version() -> Option<String> {
    let output = Command::new("curl")
        .args([
            "-fsSL",
            "--max-time",
            "5",
            &format!("https://api.github.com/repos/{REPO}/releases/latest"),
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let body = String::from_utf8_lossy(&output.stdout);
    // Simple JSON extraction — avoid adding serde just for this
    let tag = body.split("\"tag_name\"").nth(1)?.split('"').nth(1)?;

    Some(tag.strip_prefix('v').unwrap_or(tag).to_string())
}

fn version_newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        // Strip pre-release suffix (e.g., "1.0.0-beta" → "1.0.0")
        let v = v.split('-').next().unwrap_or(v);
        v.split('.').filter_map(|s| s.parse().ok()).collect()
    };
    let l = parse(latest);
    let c = parse(current);
    // Only consider newer if it's a clean release (no pre-release in latest)
    if latest.contains('-') {
        return false;
    }
    l > c
}

fn should_check() -> bool {
    let path = check_file_path();
    match fs::metadata(&path) {
        Ok(meta) => {
            let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            SystemTime::now()
                .duration_since(modified)
                .unwrap_or(CHECK_INTERVAL)
                >= CHECK_INTERVAL
        }
        Err(_) => true,
    }
}

fn touch_check_file() {
    let path = check_file_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&path, "");
}

fn check_file_path() -> PathBuf {
    dirs_hint().join(".task-update-check")
}

fn dirs_hint() -> PathBuf {
    env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::temp_dir())
}

fn detect_platform() -> &'static str {
    if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "unknown"
    }
}

fn detect_arch() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "unknown"
    }
}
