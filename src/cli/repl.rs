use super::commands::{handle_slash_command, CommandResult};
use super::file_ref::expand_file_refs;
use crate::core::agent::run_agent_turn;
use crate::core::client::Client;
use crate::core::tools::{read_file::ReadFileTool, list_files::ListFilesTool, run_command::RunCommandTool, write_file::WriteFileTool, ToolRegistry};
use crate::core::types::Message;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::io::{self, Write};

const HISTORY_FILE: &str = ".niche_history";

const DEFAULT_SYSTEM_PROMPT: &str = "You are Niche, an intelligent coding assistant. You help users with software development tasks including writing, reviewing, refactoring, and debugging code.

You have access to tools: read_file, write_file, list_files, run_command.

Rules:
1. Always read a file before editing it - never guess its contents.
2. Use list_files to explore project structure before assuming paths.
3. Be concise in explanations. Show the result, not the process.
4. When a task requires file changes, use write_file to make them directly.
5. When uncertain, ask for clarification rather than guessing.";

pub struct Repl {
    client: Client,
    tools: ToolRegistry,
    history: Vec<Message>,
    rl: DefaultEditor,
}

impl Repl {
    pub fn new(client: Client, system_prompt: Option<String>) -> Self {
        let prompt = system_prompt.unwrap_or_else(|| DEFAULT_SYSTEM_PROMPT.to_string());
        let history = vec![Message::system(prompt)];

        let mut tools = ToolRegistry::new();
        tools.register(Box::new(ReadFileTool));
        tools.register(Box::new(WriteFileTool));
        tools.register(Box::new(ListFilesTool));
        tools.register(Box::new(RunCommandTool));

        let mut rl = DefaultEditor::new().unwrap_or_else(|e| {
            eprintln!("Warning: failed to initialize line editor: {e}");
            DefaultEditor::new().unwrap_or_else(|_| panic!("Cannot init rustyline"))
        });

        let _ = rl.load_history(HISTORY_FILE);

        Self {
            client,
            tools,
            history,
            rl,
        }
    }

    pub async fn run(&mut self) {
        self.print_banner();

        loop {
            let readline = self.rl.readline("> ");
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
                if let Some(CommandResult::Exit) = handle_slash_command(line, &mut self.history) {
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
        }

        let _ = self.rl.save_history(HISTORY_FILE);
        println!("Bye!");
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
        println!("niche v{} - Agentic REPL", env!("CARGO_PKG_VERSION"));
        println!("Tools: read_file, write_file, list_files, run_command");
        println!("Type /help for commands. Ctrl+D or /exit to quit.");
        println!();
    }
}
