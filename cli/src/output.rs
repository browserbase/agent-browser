use crate::connection::Response;
use crate::session::LocalSession;
use serde_json::json;

/// Print unified session list (local + cloud)
pub fn print_session_list(
    local_sessions: &[LocalSession],
    cloud_sessions: &[serde_json::Value],
    json_mode: bool,
) {
    if json_mode {
        let local_json: Vec<_> = local_sessions
            .iter()
            .map(|s| {
                json!({
                    "type": "local",
                    "name": s.name,
                    "pid": s.pid,
                    "running": s.running,
                    "socket": s.socket_path.to_string_lossy(),
                })
            })
            .collect();

        let cloud_json: Vec<_> = cloud_sessions
            .iter()
            .map(|s| {
                let mut obj = s.clone();
                if let Some(map) = obj.as_object_mut() {
                    map.insert("type".to_string(), json!("cloud"));
                }
                obj
            })
            .collect();

        let combined: Vec<_> = local_json.into_iter().chain(cloud_json).collect();
        println!("{}", serde_json::to_string(&combined).unwrap_or_default());
        return;
    }

    let has_local = !local_sessions.is_empty();
    let has_cloud = !cloud_sessions.is_empty();

    if !has_local && !has_cloud {
        println!("No active sessions.");
        println!("\x1b[2mStart a session with: agent-browser --session <name> open <url>\x1b[0m");
        return;
    }

    if has_local {
        println!("Local Sessions:");
        for s in local_sessions {
            let status = if s.running {
                "\x1b[32m●\x1b[0m"
            } else {
                "\x1b[31m○\x1b[0m"
            };
            let state = if s.running { "running" } else { "stopped" };
            println!("  {} {} (PID: {}, {})", status, s.name, s.pid, state);
        }
    }

    if has_cloud {
        if has_local {
            println!();
        }
        println!("Cloud Sessions (Browserbase):");
        for session in cloud_sessions {
            let id = session.get("id").and_then(|v| v.as_str()).unwrap_or("?");
            let status = session.get("status").and_then(|v| v.as_str()).unwrap_or("?");
            let region = session.get("region").and_then(|v| v.as_str()).unwrap_or("?");
            let status_color = match status {
                "RUNNING" => "\x1b[32m",
                "COMPLETED" => "\x1b[34m",
                "ERROR" | "TIMED_OUT" => "\x1b[31m",
                _ => "\x1b[0m",
            };
            println!("  {}●\x1b[0m {} [{}] ({})", status_color, id, status, region);
        }
    }

    println!();
    println!("\x1b[2mUse 'session info <name|id>' for details, 'session kill <name|id>' to stop\x1b[0m");
}

pub fn print_response(resp: &Response, json_mode: bool) {
    if json_mode {
        println!("{}", serde_json::to_string(resp).unwrap_or_default());
        return;
    }

    if !resp.success {
        eprintln!(
            "\x1b[31m✗\x1b[0m {}",
            resp.error.as_deref().unwrap_or("Unknown error")
        );
        return;
    }

    if let Some(data) = &resp.data {
        // Navigation response
        if let Some(url) = data.get("url").and_then(|v| v.as_str()) {
            if let Some(title) = data.get("title").and_then(|v| v.as_str()) {
                println!("\x1b[32m✓\x1b[0m \x1b[1m{}\x1b[0m", title);
                println!("\x1b[2m  {}\x1b[0m", url);
                return;
            }
            println!("{}", url);
            return;
        }
        // Snapshot
        if let Some(snapshot) = data.get("snapshot").and_then(|v| v.as_str()) {
            println!("{}", snapshot);
            return;
        }
        // Title
        if let Some(title) = data.get("title").and_then(|v| v.as_str()) {
            println!("{}", title);
            return;
        }
        // Text
        if let Some(text) = data.get("text").and_then(|v| v.as_str()) {
            println!("{}", text);
            return;
        }
        // HTML
        if let Some(html) = data.get("html").and_then(|v| v.as_str()) {
            println!("{}", html);
            return;
        }
        // Value
        if let Some(value) = data.get("value").and_then(|v| v.as_str()) {
            println!("{}", value);
            return;
        }
        // Count
        if let Some(count) = data.get("count").and_then(|v| v.as_i64()) {
            println!("{}", count);
            return;
        }
        // Boolean results
        if let Some(visible) = data.get("visible").and_then(|v| v.as_bool()) {
            println!("{}", visible);
            return;
        }
        if let Some(enabled) = data.get("enabled").and_then(|v| v.as_bool()) {
            println!("{}", enabled);
            return;
        }
        if let Some(checked) = data.get("checked").and_then(|v| v.as_bool()) {
            println!("{}", checked);
            return;
        }
        // Eval result
        if let Some(result) = data.get("result") {
            println!(
                "{}",
                serde_json::to_string_pretty(result).unwrap_or_default()
            );
            return;
        }
        // Tabs
        if let Some(tabs) = data.get("tabs").and_then(|v| v.as_array()) {
            for (i, tab) in tabs.iter().enumerate() {
                let title = tab
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Untitled");
                let url = tab.get("url").and_then(|v| v.as_str()).unwrap_or("");
                let active = tab.get("active").and_then(|v| v.as_bool()).unwrap_or(false);
                let marker = if active { "→" } else { " " };
                println!("{} [{}] {} - {}", marker, i, title, url);
            }
            return;
        }
        // Console logs
        if let Some(logs) = data.get("logs").and_then(|v| v.as_array()) {
            for log in logs {
                let level = log.get("type").and_then(|v| v.as_str()).unwrap_or("log");
                let text = log.get("text").and_then(|v| v.as_str()).unwrap_or("");
                let color = match level {
                    "error" => "\x1b[31m",
                    "warning" => "\x1b[33m",
                    "info" => "\x1b[36m",
                    _ => "\x1b[0m",
                };
                println!("{}[{}]\x1b[0m {}", color, level, text);
            }
            return;
        }
        // Errors
        if let Some(errors) = data.get("errors").and_then(|v| v.as_array()) {
            for err in errors {
                let msg = err.get("message").and_then(|v| v.as_str()).unwrap_or("");
                println!("\x1b[31m✗\x1b[0m {}", msg);
            }
            return;
        }
        // Cookies
        if let Some(cookies) = data.get("cookies").and_then(|v| v.as_array()) {
            for cookie in cookies {
                let name = cookie.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let value = cookie.get("value").and_then(|v| v.as_str()).unwrap_or("");
                println!("{}={}", name, value);
            }
            return;
        }
        // Bounding box
        if let Some(box_data) = data.get("box") {
            println!(
                "{}",
                serde_json::to_string_pretty(box_data).unwrap_or_default()
            );
            return;
        }
        // Closed
        if data.get("closed").is_some() {
            println!("\x1b[32m✓\x1b[0m Browser closed");
            return;
        }
        // Screenshot path
        if let Some(path) = data.get("path").and_then(|v| v.as_str()) {
            println!("\x1b[32m✓\x1b[0m Screenshot saved to {}", path);
            return;
        }
        // Browserbase session list
        if let Some(sessions) = data.get("sessions").and_then(|v| v.as_array()) {
            if sessions.is_empty() {
                println!("No Browserbase sessions found.");
            } else {
                println!("Browserbase Sessions:");
                for session in sessions {
                    let id = session.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                    let status = session.get("status").and_then(|v| v.as_str()).unwrap_or("?");
                    let region = session.get("region").and_then(|v| v.as_str()).unwrap_or("?");
                    let created = session.get("createdAt").and_then(|v| v.as_str()).unwrap_or("?");
                    let status_color = match status {
                        "RUNNING" => "\x1b[32m",
                        "COMPLETED" => "\x1b[34m",
                        "ERROR" | "TIMED_OUT" => "\x1b[31m",
                        _ => "\x1b[0m",
                    };
                    println!("  {}●\x1b[0m {} [{}]", status_color, id, status);
                    println!("      Region: {}, Created: {}", region, created);
                }
            }
            return;
        }
        // Browserbase session details
        if let Some(session) = data.get("session") {
            let id = session.get("id").and_then(|v| v.as_str()).unwrap_or("?");
            let status = session.get("status").and_then(|v| v.as_str()).unwrap_or("?");
            let region = session.get("region").and_then(|v| v.as_str()).unwrap_or("?");
            let created = session.get("createdAt").and_then(|v| v.as_str()).unwrap_or("?");
            let keep_alive = session.get("keepAlive").and_then(|v| v.as_bool()).unwrap_or(false);
            let project = session.get("projectId").and_then(|v| v.as_str()).unwrap_or("?");

            println!("Session: \x1b[1m{}\x1b[0m", id);
            println!();
            let status_icon = match status {
                "RUNNING" => "\x1b[32m●\x1b[0m",
                "COMPLETED" => "\x1b[34m●\x1b[0m",
                _ => "\x1b[31m●\x1b[0m",
            };
            println!("  Status:     {} {}", status_icon, status);
            println!("  Region:     {}", region);
            println!("  Project:    {}", project);
            println!("  Created:    {}", created);
            println!("  Keep Alive: {}", if keep_alive { "yes" } else { "no" });

            if let Some(connect_url) = session.get("connectUrl").and_then(|v| v.as_str()) {
                println!("  Connect:    {}", connect_url);
            }
            return;
        }
        // Browserbase debug URLs
        if let Some(debug) = data.get("debug") {
            let debugger_url = debug.get("debuggerUrl").and_then(|v| v.as_str()).unwrap_or("?");
            let fullscreen = debug.get("debuggerFullscreenUrl").and_then(|v| v.as_str());
            let ws_url = debug.get("wsUrl").and_then(|v| v.as_str());

            println!("Debug URLs:");
            println!("  Debugger:   {}", debugger_url);
            if let Some(fs) = fullscreen {
                println!("  Fullscreen: {}", fs);
            }
            if let Some(ws) = ws_url {
                println!("  WebSocket:  {}", ws);
            }

            if let Some(pages) = debug.get("pages").and_then(|v| v.as_array()) {
                if !pages.is_empty() {
                    println!("\nPages:");
                    for page in pages {
                        let title = page.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled");
                        let url = page.get("url").and_then(|v| v.as_str()).unwrap_or("?");
                        println!("  - {} ({})", title, url);
                    }
                }
            }
            return;
        }
        // Session stopped
        if data.get("stopped").is_some() {
            let session_id = data.get("sessionId").and_then(|v| v.as_str()).unwrap_or("session");
            println!("\x1b[32m✓\x1b[0m Session {} stopped", session_id);
            return;
        }
        // Default success
        println!("\x1b[32m✓\x1b[0m Done");
    }
}

pub fn print_help() {
    println!(
        r#"
agent-browser - fast browser automation CLI for AI agents

Usage: agent-browser <command> [args] [options]

Core Commands:
  open <url>                 Navigate to URL
  click <sel>                Click element (or @ref)
  dblclick <sel>             Double-click element
  type <sel> <text>          Type into element
  fill <sel> <text>          Clear and fill
  press <key>                Press key (Enter, Tab, Control+a)
  hover <sel>                Hover element
  focus <sel>                Focus element
  check <sel>                Check checkbox
  uncheck <sel>              Uncheck checkbox
  select <sel> <val>         Select dropdown option
  drag <src> <dst>           Drag and drop
  upload <sel> <files...>    Upload files
  scroll <dir> [px]          Scroll (up/down/left/right)
  scrollintoview <sel>       Scroll element into view
  wait <sel|ms>              Wait for element or time
  screenshot [path]          Take screenshot
  pdf <path>                 Save as PDF
  snapshot                   Accessibility tree with refs (for AI)
  eval <js>                  Run JavaScript
  close                      Close browser

Navigation:
  back                       Go back
  forward                    Go forward
  reload                     Reload page

Get Info:  agent-browser get <what> [selector]
  text, html, value, attr <name>, title, url, count, box

Check State:  agent-browser is <what> <selector>
  visible, enabled, checked

Find Elements:  agent-browser find <locator> <value> <action> [text]
  role, text, label, placeholder, alt, title, testid, first, last, nth

Mouse:  agent-browser mouse <action> [args]
  move <x> <y>, down [btn], up [btn], wheel <dy> [dx]

Browser Settings:  agent-browser set <setting> [value]
  viewport <w> <h>, device <name>, geo <lat> <lng>
  offline [on|off], headers <json>, credentials <user> <pass>
  media [dark|light] [reduced-motion]

Network:  agent-browser network <action>
  route <url> [--abort|--body <json>]
  unroute [url]
  requests [--clear] [--filter <pattern>]

Storage:
  cookies [get|set|clear]    Manage cookies
  storage <local|session>    Manage web storage

Tabs:
  tab [new|list|close|<n>]   Manage tabs

Debug:
  trace start|stop [path]    Record trace
  console [--clear]          View console logs
  errors [--clear]           View page errors
  highlight <sel>            Highlight element

Sessions:
  session list               List all active sessions (local + cloud)
  session                    Show current session
  session info <name|id>     Show session details
  session kill <name|id>     Stop/kill a session
  session debug <id>         Get debug URLs (cloud sessions only)

Setup:
  install                    Install browser binaries
  install --with-deps        Also install system dependencies (Linux)

Snapshot Options:
  -i, --interactive          Only interactive elements
  -c, --compact              Remove empty structural elements
  -d, --depth <n>            Limit tree depth
  -s, --selector <sel>       Scope to CSS selector

Options:
  --session <name>           Isolated session (or AGENT_BROWSER_SESSION env)
  --json                     JSON output
  --full, -f                 Full page screenshot
  --headed                   Show browser window (not headless)
  --debug                    Debug output

Examples:
  agent-browser open example.com
  agent-browser snapshot -i              # Interactive elements only
  agent-browser click @e2                # Click by ref from snapshot
  agent-browser fill @e3 "test@example.com"
  agent-browser find role button click --name Submit
  agent-browser get text @e1
  agent-browser screenshot --full
"#
    );
}
