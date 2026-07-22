use crate::core::client::Client;
use crate::core::types::Message;

pub enum CommandResult {
    Continue,
    Exit,
}

/// Context passed to slash command handlers.
pub struct CommandContext<'a> {
    pub history: &'a mut Vec<Message>,
    pub client: &'a Client,
    pub session: &'a mut super::session::SessionManager,
}

pub fn handle_slash_command(input: &str, ctx: &mut CommandContext) -> Option<CommandResult> {
    let input = input.trim();
    let (cmd, args) = match input.split_once(' ') {
        Some((c, a)) => (c, Some(a.trim())),
        None => (input, None),
    };

    match cmd {
        "/help" | "/h" | "/?" => {
            print_help();
            Some(CommandResult::Continue)
        }
        "/clear" => {
            let system_count = ctx.history.iter().filter(|m| m.role == "system").count();
            ctx.history.retain(|m| m.role == "system");
            println!(
                "Cleared {} messages.{}",
                ctx.history.len() - system_count,
                if system_count > 0 {
                    " System prompt retained."
                } else {
                    ""
                }
            );
            Some(CommandResult::Continue)
        }
        "/history" | "/hist" => {
            print_history(ctx.history);
            Some(CommandResult::Continue)
        }
        "/version" | "/v" => {
            println!("niche v{}", env!("CARGO_PKG_VERSION"));
            Some(CommandResult::Continue)
        }
        "/model" | "/m" => {
            handle_model(args, ctx.client);
            Some(CommandResult::Continue)
        }
        "/new" | "/n" => {
            // Save current session, signal that REPL should start fresh
            ctx.session.save();
            println!("Session saved. Starting new session.");
            // Mark for new session by clearing history but keeping system prompt
            let system: Vec<Message> = ctx.history.iter().filter(|m| m.role == "system").cloned().collect();
            *ctx.history = system;
            ctx.session.start_new(&ctx.client.active_model().name, ctx.history.clone());
            println!("New session created.");
            Some(CommandResult::Continue)
        }
        "/sessions" | "/ss" => {
            list_sessions();
            Some(CommandResult::Continue)
        }
        "/session" | "/resume" | "/r" => {
            handle_resume(args, ctx);
            Some(CommandResult::Continue)
        }
        "/exit" | "/quit" | "/q" => Some(CommandResult::Exit),
        _ => None,
    }
}

fn handle_model(args: Option<&str>, client: &Client) {
    let models = client.available_models();
    if models.is_empty() {
        let active = client.active_model();
        println!("Current model: {} (no multi-model config)", active.name);
        println!("Add a 'models' array to config.json to enable /model switching.");
        return;
    }

    match args {
        Some(name) => match client.switch_model(name) {
            Ok(display) => {
                println!("Switched to: {} ({})", display, name);
            }
            Err(e) => {
                println!("{e}");
                println!("\nAvailable models:");
                for (name, display, active) in &models {
                    let marker = if *active { " *" } else { "  " };
                    println!("  {marker} {name} - {display}");
                }
            }
        },
        None => {
            println!("Available models:");
            for (name, display, active) in &models {
                let marker = if *active { " *" } else { "  " };
                println!("  {marker} {name} - {display}");
            }
            println!("\n  * = active   |   /model <name> to switch");
        }
    }
}

fn list_sessions() {
    let sessions = super::session::SessionManager::list_sessions();
    if sessions.is_empty() {
        println!("No saved sessions.");
        return;
    }

    println!("Sessions (most recent first):\n");
    for (i, s) in sessions.iter().enumerate().take(20) {
        let age = super::session::format_age(&s.updated_at);
        let summary = super::session::session_summary(s);
        let title = if s.title.len() > 50 {
            format!("{}...", &s.title[..47])
        } else {
            s.title.clone()
        };
        let current = if i == 0 { " (latest)" } else { "" };
        println!("  {}. [{}] {}", i + 1, age, title);
        println!("     {} | {} |{current}", s.id, summary);
    }

    if sessions.len() > 20 {
        println!("\n  ... and {} more", sessions.len() - 20);
    }

    println!("\n  /session <id> to resume");
}

fn handle_resume(args: Option<&str>, ctx: &mut CommandContext) {
    let id = match args {
        Some(id) => id.to_string(),
        None => {
            match super::session::SessionManager::latest_id() {
                Some(id) => {
                    println!("No session ID given. Resuming latest: {id}");
                    id
                }
                None => {
                    println!("No saved sessions to resume.");
                    return;
                }
            }
        }
    };

    match super::session::SessionManager::load(&id) {
        Some(session) => {
            let msg_count = session.messages.iter().filter(|m| m.role != "system").count();
            let title = session.title.clone();
            *ctx.history = session.messages.clone();
            ctx.session.load_into(&id).ok();
            println!("Resumed: {title} ({msg_count} messages)");
        }
        None => {
            println!("Session '{id}' not found.");
        }
    }
}

fn print_help() {
    println!("niche v{} - Commands", env!("CARGO_PKG_VERSION"));
    println!();
    println!("  Session:");
    println!("    /new, /n           Save current, start new session");
    println!("    /sessions, /ss     List saved sessions");
    println!("    /session <id>      Resume a session (or /session for latest)");
    println!();
    println!("  Model:");
    println!("    /model, /m         List available models");
    println!("    /model <name>      Switch to a specific model");
    println!();
    println!("  Other:");
    println!("    /help, /h          Show this help message");
    println!("    /clear             Clear conversation history");
    println!("    /history, /hist    Show conversation history");
    println!("    /version, /v       Show version");
    println!("    /exit, /quit, /q   Exit");
    println!();
    println!("Tips:");
    println!("  @path/to/file       Include file contents in your message");
    println!("  \\\\ at line end      Continue on next line (multi-line input)");
    println!("  Up/Down arrows      Navigate command history");
    println!("  Ctrl+R              Search command history");
    println!("  Ctrl+C              Cancel current input");
    println!("  Ctrl+D              Exit");
}

fn print_history(history: &[Message]) {
    if history.is_empty() {
        println!("(empty)");
        return;
    }

    let msgs: Vec<&Message> = history.iter().filter(|m| m.role != "system").collect();
    if msgs.is_empty() {
        println!("(no messages)");
        return;
    }

    for (i, msg) in msgs.iter().enumerate() {
        let label = match msg.role.as_str() {
            "user" => "You",
            "assistant" => "niche",
            _ => "System",
        };
        let content = msg.content.as_deref().unwrap_or("(tool call)");
        let preview = if content.len() > 100 {
            format!("{}...", &content[..97])
        } else {
            content.to_string()
        };
        let preview = preview.replace('\n', " ");
        println!("  {}. [{}] {}", i + 1, label, preview);
    }
}
