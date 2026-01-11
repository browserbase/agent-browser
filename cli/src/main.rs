mod commands;
mod connection;
mod flags;
mod install;
mod output;
mod session;

use serde_json::json;
use std::env;
use std::process::exit;

use commands::{gen_id, parse_command};
use connection::{ensure_daemon, send_command};
use flags::{clean_args, parse_flags};
use install::run_install;
use output::{print_help, print_response};
use session::{find_local_sessions, is_uuid, kill_local_session, show_local_session_info};

/// Try to get cloud sessions from daemon (returns empty vec if daemon not running or API key not set)
fn try_get_cloud_sessions(session: &str, _json: bool) -> Vec<serde_json::Value> {
    // Check if daemon is running first
    if !connection::is_daemon_running(session) {
        return Vec::new();
    }

    let cmd = json!({
        "id": gen_id(),
        "action": "bb_session_list"
    });

    match send_command(cmd, session) {
        Ok(resp) if resp.success => {
            if let Some(data) = &resp.data {
                if let Some(sessions) = data.get("sessions") {
                    if let Some(arr) = sessions.as_array() {
                        return arr.clone();
                    }
                }
            }
            Vec::new()
        }
        _ => Vec::new(),
    }
}

/// Handle cloud session info query
fn handle_cloud_session_info(id: &str, flags: &flags::Flags) {
    if let Err(e) = ensure_daemon(&flags.session, flags.headed) {
        if flags.json {
            println!(r#"{{"success":false,"error":"{}"}}"#, e);
        } else {
            eprintln!("\x1b[31m✗\x1b[0m {}", e);
        }
        exit(1);
    }

    let cmd = json!({
        "id": gen_id(),
        "action": "bb_session_get",
        "sessionId": id
    });

    match send_command(cmd, &flags.session) {
        Ok(resp) => {
            print_response(&resp, flags.json);
            if !resp.success {
                exit(1);
            }
        }
        Err(e) => {
            if flags.json {
                println!(r#"{{"success":false,"error":"{}"}}"#, e);
            } else {
                eprintln!("\x1b[31m✗\x1b[0m {}", e);
            }
            exit(1);
        }
    }
}

/// Handle cloud session stop
fn handle_cloud_session_stop(id: &str, flags: &flags::Flags) {
    if let Err(e) = ensure_daemon(&flags.session, flags.headed) {
        if flags.json {
            println!(r#"{{"success":false,"error":"{}"}}"#, e);
        } else {
            eprintln!("\x1b[31m✗\x1b[0m {}", e);
        }
        exit(1);
    }

    let cmd = json!({
        "id": gen_id(),
        "action": "bb_session_stop",
        "sessionId": id
    });

    match send_command(cmd, &flags.session) {
        Ok(resp) => {
            print_response(&resp, flags.json);
            if !resp.success {
                exit(1);
            }
        }
        Err(e) => {
            if flags.json {
                println!(r#"{{"success":false,"error":"{}"}}"#, e);
            } else {
                eprintln!("\x1b[31m✗\x1b[0m {}", e);
            }
            exit(1);
        }
    }
}

/// Handle cloud session debug
fn handle_cloud_session_debug(id: &str, flags: &flags::Flags) {
    if let Err(e) = ensure_daemon(&flags.session, flags.headed) {
        if flags.json {
            println!(r#"{{"success":false,"error":"{}"}}"#, e);
        } else {
            eprintln!("\x1b[31m✗\x1b[0m {}", e);
        }
        exit(1);
    }

    let cmd = json!({
        "id": gen_id(),
        "action": "bb_session_debug",
        "sessionId": id
    });

    match send_command(cmd, &flags.session) {
        Ok(resp) => {
            print_response(&resp, flags.json);
            if !resp.success {
                exit(1);
            }
        }
        Err(e) => {
            if flags.json {
                println!(r#"{{"success":false,"error":"{}"}}"#, e);
            } else {
                eprintln!("\x1b[31m✗\x1b[0m {}", e);
            }
            exit(1);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let flags = parse_flags(&args);
    let clean = clean_args(&args);

    if clean.is_empty() || args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return;
    }

    // Handle install separately
    if clean.get(0).map(|s| s.as_str()) == Some("install") {
        let with_deps = args.iter().any(|a| a == "--with-deps" || a == "-d");
        run_install(with_deps);
        return;
    }

    // Handle unified session commands
    if clean.get(0).map(|s| s.as_str()) == Some("session") {
        match clean.get(1).map(|s| s.as_str()) {
            Some("list") => {
                // Get local sessions first
                let local_sessions = find_local_sessions();

                // Try to get cloud sessions from daemon (non-blocking)
                let cloud_sessions = try_get_cloud_sessions(&flags.session, flags.json);

                // Print combined results
                output::print_session_list(&local_sessions, &cloud_sessions, flags.json);
                return;
            }
            Some("info") => {
                let id = clean.get(2).map(|s| s.as_str()).unwrap_or(&flags.session);

                // If looks like UUID, query cloud; otherwise check local
                if is_uuid(id) {
                    // Query cloud session via daemon
                    handle_cloud_session_info(id, &flags);
                } else {
                    show_local_session_info(id, flags.json);
                }
                return;
            }
            Some("kill") => {
                if let Some(id) = clean.get(2) {
                    // If looks like UUID, stop cloud session; otherwise kill local
                    if is_uuid(id) {
                        handle_cloud_session_stop(id, &flags);
                    } else {
                        kill_local_session(id, flags.json);
                    }
                } else {
                    if flags.json {
                        println!(r#"{{"success":false,"error":"session kill requires a session name or ID"}}"#);
                    } else {
                        eprintln!("\x1b[31m✗\x1b[0m session kill requires a session name or ID");
                        eprintln!("\x1b[2mUsage: agent-browser session kill <name|id>\x1b[0m");
                    }
                    exit(1);
                }
                return;
            }
            Some("debug") => {
                if let Some(id) = clean.get(2) {
                    handle_cloud_session_debug(id, &flags);
                } else {
                    if flags.json {
                        println!(r#"{{"success":false,"error":"session debug requires a session ID"}}"#);
                    } else {
                        eprintln!("\x1b[31m✗\x1b[0m session debug requires a session ID");
                        eprintln!("\x1b[2mUsage: agent-browser session debug <id>\x1b[0m");
                    }
                    exit(1);
                }
                return;
            }
            Some(_) => {
                if flags.json {
                    println!(r#"{{"success":false,"error":"Unknown session subcommand"}}"#);
                } else {
                    eprintln!("\x1b[31m✗\x1b[0m Unknown session subcommand");
                    eprintln!("\x1b[2mUsage: agent-browser session [list|info|kill|debug]\x1b[0m");
                }
                exit(1);
            }
            None => {
                // `session` with no subcommand shows current session
                show_local_session_info(&flags.session, flags.json);
                return;
            }
        }
    }

    let cmd = match parse_command(&clean, &flags) {
        Some(c) => c,
        None => {
            eprintln!(
                "\x1b[31mUnknown command:\x1b[0m {}",
                clean.get(0).unwrap_or(&String::new())
            );
            eprintln!("\x1b[2mRun: agent-browser --help\x1b[0m");
            exit(1);
        }
    };

    if let Err(e) = ensure_daemon(&flags.session, flags.headed) {
        if flags.json {
            println!(r#"{{"success":false,"error":"{}"}}"#, e);
        } else {
            eprintln!("\x1b[31m✗\x1b[0m {}", e);
        }
        exit(1);
    }

    // If --headed flag is set, send launch command first to switch to headed mode
    if flags.headed {
        let launch_cmd = json!({ "id": gen_id(), "action": "launch", "headless": false });
        if let Err(e) = send_command(launch_cmd, &flags.session) {
            if !flags.json {
                eprintln!("\x1b[33m⚠\x1b[0m Could not switch to headed mode: {}", e);
            }
        }
    }

    match send_command(cmd, &flags.session) {
        Ok(resp) => {
            let success = resp.success;
            print_response(&resp, flags.json);
            if !success {
                exit(1);
            }
        }
        Err(e) => {
            if flags.json {
                println!(r#"{{"success":false,"error":"{}"}}"#, e);
            } else {
                eprintln!("\x1b[31m✗\x1b[0m {}", e);
            }
            exit(1);
        }
    }
}
