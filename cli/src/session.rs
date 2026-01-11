use serde_json::json;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::exit;

/// Check if a string looks like a UUID (Browserbase session ID)
pub fn is_uuid(s: &str) -> bool {
    // UUIDs are 36 chars: 8-4-4-4-12 with hyphens
    if s.len() != 36 {
        return false;
    }
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 5 {
        return false;
    }
    let expected_lens = [8, 4, 4, 4, 12];
    for (i, part) in parts.iter().enumerate() {
        if part.len() != expected_lens[i] {
            return false;
        }
        if !part.chars().all(|c| c.is_ascii_hexdigit()) {
            return false;
        }
    }
    true
}

/// Get the temp directory for session files
fn get_temp_dir() -> PathBuf {
    env::temp_dir()
}

/// Check if a process is running by PID
#[cfg(unix)]
fn is_process_running(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

#[cfg(windows)]
fn is_process_running(_pid: i32) -> bool {
    // On Windows, simplified check - could use tasklist or Windows APIs
    // For now, assume running if PID file exists
    true
}

/// Information about a local session
#[derive(Debug)]
pub struct LocalSession {
    pub name: String,
    pub pid: i32,
    pub running: bool,
    pub socket_path: PathBuf,
    pub socket_exists: bool,
}

/// Find all local sessions by scanning PID files in temp directory
pub fn find_local_sessions() -> Vec<LocalSession> {
    let tmp = get_temp_dir();
    let mut sessions = Vec::new();

    if let Ok(entries) = fs::read_dir(&tmp) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("agent-browser-") && name.ends_with(".pid") {
                let session_name = name
                    .strip_prefix("agent-browser-")
                    .and_then(|s| s.strip_suffix(".pid"))
                    .unwrap_or("")
                    .to_string();

                if session_name.is_empty() {
                    continue;
                }

                if let Ok(pid_str) = fs::read_to_string(entry.path()) {
                    if let Ok(pid) = pid_str.trim().parse::<i32>() {
                        let running = is_process_running(pid);
                        let socket_path = tmp.join(format!("agent-browser-{}.sock", session_name));
                        let socket_exists = socket_path.exists();

                        sessions.push(LocalSession {
                            name: session_name,
                            pid,
                            running,
                            socket_path,
                            socket_exists,
                        });
                    }
                }
            }
        }
    }

    // Sort by name for consistent output
    sessions.sort_by(|a, b| a.name.cmp(&b.name));
    sessions
}

/// Show detailed info about a specific local session
pub fn show_local_session_info(name: &str, json_mode: bool) {
    let tmp = get_temp_dir();
    let pid_path = tmp.join(format!("agent-browser-{}.pid", name));
    let socket_path = tmp.join(format!("agent-browser-{}.sock", name));

    if !pid_path.exists() {
        if json_mode {
            println!(
                "{}",
                json!({ "success": false, "error": format!("Session '{}' not found", name) })
            );
        } else {
            eprintln!("\x1b[31m✗\x1b[0m Session '{}' not found", name);
        }
        exit(1);
    }

    let pid = fs::read_to_string(&pid_path)
        .ok()
        .and_then(|s| s.trim().parse::<i32>().ok())
        .unwrap_or(0);

    let running = is_process_running(pid);
    let socket_exists = socket_path.exists();

    if json_mode {
        println!(
            "{}",
            json!({
                "success": true,
                "session": {
                    "name": name,
                    "pid": pid,
                    "running": running,
                    "socket": socket_path.to_string_lossy(),
                    "socketExists": socket_exists,
                    "pidFile": pid_path.to_string_lossy()
                }
            })
        );
    } else {
        println!("Session: \x1b[1m{}\x1b[0m", name);
        println!();
        let status_icon = if running {
            "\x1b[32m●\x1b[0m"
        } else {
            "\x1b[31m○\x1b[0m"
        };
        let status_text = if running { "running" } else { "stopped" };
        println!("  Status:      {} {}", status_icon, status_text);
        println!("  PID:         {}", pid);
        println!("  PID File:    {}", pid_path.display());
        println!("  Socket:      {}", socket_path.display());
        println!(
            "  Socket OK:   {}",
            if socket_exists { "yes" } else { "no" }
        );
    }
}

/// Kill a local daemon session
pub fn kill_local_session(name: &str, json_mode: bool) {
    let tmp = get_temp_dir();
    let pid_path = tmp.join(format!("agent-browser-{}.pid", name));
    let socket_path = tmp.join(format!("agent-browser-{}.sock", name));

    if !pid_path.exists() {
        if json_mode {
            println!(
                "{}",
                json!({ "success": false, "error": format!("Session '{}' not found", name) })
            );
        } else {
            eprintln!("\x1b[31m✗\x1b[0m Session '{}' not found", name);
        }
        exit(1);
    }

    let pid = match fs::read_to_string(&pid_path)
        .ok()
        .and_then(|s| s.trim().parse::<i32>().ok())
    {
        Some(p) => p,
        None => {
            if json_mode {
                println!(
                    "{}",
                    json!({ "success": false, "error": "Failed to read PID file" })
                );
            } else {
                eprintln!("\x1b[31m✗\x1b[0m Failed to read PID file");
            }
            exit(1);
        }
    };

    // Send SIGTERM to the process
    #[cfg(unix)]
    {
        let result = unsafe { libc::kill(pid, libc::SIGTERM) };
        if result != 0 && is_process_running(pid) {
            if json_mode {
                println!(
                    "{}",
                    json!({ "success": false, "error": format!("Failed to kill process {}", pid) })
                );
            } else {
                eprintln!("\x1b[31m✗\x1b[0m Failed to kill process {}", pid);
            }
            exit(1);
        }
    }

    #[cfg(windows)]
    {
        // On Windows, use taskkill
        let status = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .status();

        if status.is_err() || !status.unwrap().success() {
            if json_mode {
                println!(
                    "{}",
                    json!({ "success": false, "error": format!("Failed to kill process {}", pid) })
                );
            } else {
                eprintln!("\x1b[31m✗\x1b[0m Failed to kill process {}", pid);
            }
            exit(1);
        }
    }

    // Clean up files
    let _ = fs::remove_file(&pid_path);
    let _ = fs::remove_file(&socket_path);

    if json_mode {
        println!(
            "{}",
            json!({ "success": true, "killed": name, "pid": pid })
        );
    } else {
        println!(
            "\x1b[32m✓\x1b[0m Killed session '{}' (PID: {})",
            name, pid
        );
    }
}
