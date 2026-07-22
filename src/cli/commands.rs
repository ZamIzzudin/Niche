use crate::core::types::Message;

pub enum CommandResult {
    Continue,
    Exit,
}

pub fn handle_slash_command(input: &str, history: &mut Vec<Message>) -> Option<CommandResult> {
    let input = input.trim();
    let (cmd, _args) = match input.split_once(' ') {
        Some((c, a)) => (c, Some(a)),
        None => (input, None),
    };

    match cmd {
        "/help" | "/h" | "/?" => {
            print_help();
            Some(CommandResult::Continue)
        }
        "/clear" => {
            let system_count = history.iter().filter(|m| m.role == "system").count();
            history.retain(|m| m.role == "system");
            println!(
                "Cleared {} messages.{}",
                history.len() - system_count,
                if system_count > 0 {
                    " System prompt retained."
                } else {
                    ""
                }
            );
            Some(CommandResult::Continue)
        }
        "/history" | "/hist" => {
            print_history(history);
            Some(CommandResult::Continue)
        }
        "/version" | "/v" => {
            println!("niche v{}", env!("CARGO_PKG_VERSION"));
            Some(CommandResult::Continue)
        }
        "/exit" | "/quit" | "/q" => Some(CommandResult::Exit),
        _ => None,
    }
}

fn print_help() {
    println!("niche v{} - Commands", env!("CARGO_PKG_VERSION"));
    println!();
    println!("  /help, /h        Show this help message");
    println!("  /clear           Clear conversation history");
    println!("  /history, /hist  Show conversation history");
    println!("  /version, /v     Show version");
    println!("  /exit, /quit, /q Exit");
    println!();
    println!("Tips:");
    println!("  @path/to/file    Include file contents in your message");
    println!("  \\\\ at line end   Continue on next line (multi-line input)");
    println!("  Up/Down arrows   Navigate command history");
    println!("  Ctrl+R           Search command history");
    println!("  Ctrl+C           Cancel current input");
    println!("  Ctrl+D           Exit");
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
