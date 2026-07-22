use super::commands::{handle_slash_command, CommandContext, CommandResult};
use super::file_ref::expand_file_refs;
use super::session::SessionManager;
use crate::core::agent::run_agent_turn;
use crate::core::client::Client;
use crate::core::tools::{
    edit_file::EditFileTool, glob_tool::GlobTool, grep::GrepTool, list_files::ListFilesTool,
    read_file::ReadFileTool, run_command::RunCommandTool, write_file::WriteFileTool,
    ToolRegistry,
};
use crate::core::types::Message;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::io::{self, Write};

const HISTORY_FILE: &str = ".niche_history";

const DEFAULT_SYSTEM_PROMPT: &str = "You are Niche, an intelligent coding assistant. You help users with software development tasks including writing, reviewing, refactoring, and debugging code.

You have access to tools: read_file, write_file, edit_file, list_files, grep, glob, run_command.

Rules:
1. Always read a file before editing it - never guess its contents.
2. Use list_files to explore project structure, and grep/glob to find specific files or patterns.
3. Prefer edit_file over write_file for modifying existing files - it is safer and more precise.
4. For edit_file, provide old_text that appears exactly once in the file. If it appears multiple times, include more surrounding context to make it unique.
5. Be concise in explanations. Show the result, not the process.
6. When uncertain, ask for clarification rather than guessing.";

pub struct Repl {
    client: Client,
    tools: ToolRegistry,
    history: Vec<Message>,
    rl: DefaultEditor,
    sessions: SessionManager,
}

impl Repl {
    pub fn new(client: Client, system_prompt: Option<String>) -> Self {
        let prompt = system_prompt.unwrap_or_else(|| DEFAULT_SYSTEM_PROMPT.to_string());
        let mut history = vec![Message::system(prompt)];

        let mut tools = ToolRegistry::new();
        tools.register(Box::new(ReadFileTool));
        tools.register(Box::new(WriteFileTool));
        tools.register(Box::new(EditFileTool));
        tools.register(Box::new(ListFilesTool));
        tools.register(Box::new(GrepTool));
        tools.register(Box::new(GlobTool));
        tools.register(Box::new(RunCommandTool));

        let mut rl = DefaultEditor::new().unwrap_or_else(|e| {
            eprintln!("Warning: failed to initialize line editor: {e}");
            DefaultEditor::new().unwrap_or_else(|_| panic!("Cannot init rustyline"))
        });

        let _ = rl.load_history(HISTORY_FILE);

        // Session: try resume latest, else start new
        let mut sessions = SessionManager::new();
        let active_model = client.active_model().name;
        match SessionManager::load_latest() {
            Some(latest) => {
                let title = latest.title.clone();
                let msg_count = latest.messages.iter().filter(|m| m.role != "system").count();
                history = latest.messages.clone();
                sessions.current = Some(latest);
                eprintln!(
                    "Resumed session: {title} ({msg_count} messages)\n  /new for fresh session | /sessions to browse\n"
                );
            }
            None => {
                sessions.start_new(&active_model, history.clone());
            }
        }

        Self {
            client,
            tools,
            history,
            rl,
            sessions,
        }
    }

    pub async fn run(&mut self) {
        self.print_banner();

        loop {
            let prompt = self.prompt_string();
            let readline = self.rl.readline(&prompt);
            let line = match readline {
                Ok(l) => l,
                Err(ReadlineError::Interrupted) => {
                    println!();
                    continue;
                }
                Err(ReadlineError::Eof) => break,
                Err(e) => {
                    eprintln!("Error: {e}");
                    break;
                }
            };

            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let _ = self.rl.add_history_entry(line);

            if line == "exit" || line == "quit" {
                break;
            }

            if line.starts_with('/') {
                let mut ctx = CommandContext {
                    history: &mut self.history,
                    client: &self.client,
                    session: &mut self.sessions,
                };
                if let Some(CommandResult::Exit) = handle_slash_command(line, &mut ctx) {
                    break;
                }
                continue;
            }

            let input = match self.read_multiline(line) {
                Some(input) => input,
                None => continue,
            };

            let expanded = expand_file_refs(&input);
            self.history.push(Message::user(expanded));

            print!("\n");
            let _ = io::stdout().flush();

            let result = run_agent_turn(&self.client, &self.tools, &mut self.history, |token| {
                print!("{token}");
                let _ = io::stdout().flush();
            })
            .await;

            println!("\n");

            if let Err(e) = result {
                eprintln!("Error: {e}\n");
                self.history.pop();
            }

            // Auto-save session
            if let Some(session) = self.sessions.current_mut() {
                session.messages = self.history.clone();
                session.model = self.client.active_model().name;
            }
            self.sessions.save();
        }

        // Save session on exit
        self.sessions.save();
        let _ = self.rl.save_history(HISTORY_FILE);
        println!("Bye!");
    }

    fn prompt_string(&self) -> String {
        let model = self.client.active_model().name;
        let short_model: String = model.split('/').last().unwrap_or(&model).to_string();
        let model_display: String = if short_model.len() > 20 {
            format!("{}...", &short_model[..17])
        } else {
            short_model
        };
        format!("niche({model_display})> ")
    }

    fn read_multiline(&mut self, first_line: &str) -> Option<String> {
        if !first_line.ends_with('\\') {
            return Some(first_line.to_string());
        }

        let mut full_input = first_line[..first_line.len() - 1].to_string();

        loop {
            let next = match self.rl.readline("... ") {
                Ok(l) => l,
                Err(ReadlineError::Interrupted) => return None,
                Err(ReadlineError::Eof) => return None,
                Err(e) => {
                    eprintln!("Error: {e}");
                    return None;
                }
            };

            let next = next.trim();
            if next.is_empty() {
                break;
            }
            let _ = self.rl.add_history_entry(next);

            if next.ends_with('\\') {
                full_input.push('\n');
                full_input.push_str(&next[..next.len() - 1]);
            } else {
                full_input.push('\n');
                full_input.push_str(next);
                break;
            }
        }

        Some(full_input)
    }

    fn print_banner(&self) {
        let active = self.client.active_model();
        let models = self.client.available_models();

        println!("niche v{} - Agentic REPL", env!("CARGO_PKG_VERSION"));
        println!("Model: {}", active.name);
        if !models.is_empty() {
            let names: Vec<&str> = models.iter().map(|(n, _, _)| *n).collect();
            println!("Available: {}", names.join(", "));
        }
        println!("Tools: read_file, write_file, edit_file, list_files, grep, glob, run_command");
        println!("Type /help for commands. Ctrl+D or /exit to quit.");
        println!();
    }
}
