use crate::core::client::Client;
use crate::core::types::Message;
use std::io::{self, Write};

pub struct Repl {
    client: Client,
    history: Vec<Message>,
}

impl Repl {
    pub fn new(client: Client, system_prompt: Option<String>) -> Self {
        let mut history = Vec::new();
        if let Some(sp) = system_prompt {
            history.push(Message::system(sp));
        }
        Self { client, history }
    }

    pub async fn run(&mut self) {
        println!("niche v{} - Interactive REPL", env!("CARGO_PKG_VERSION"));
        println!("Type 'exit' or press Ctrl+C to quit.");
        println!();

        loop {
            print!("> ");
            io::stdout().flush().unwrap_or(());

            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(0) => break,
                Ok(_) => {}
                Err(_) => break,
            }

            let input = input.trim();
            if input.is_empty() {
                continue;
            }
            if input == "exit" {
                break;
            }

            self.history.push(Message::user(input));

            let result = self
                .client
                .chat_stream(&self.history, |token| {
                    print!("{token}");
                    io::stdout().flush().unwrap_or(());
                })
                .await;

            println!("\n");

            match result {
                Ok(content) => {
                    if content.trim().is_empty() {
                        eprintln!("(empty response)\n");
                        self.history.pop();
                    } else {
                        self.history.push(Message::assistant(content));
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}\n");
                    self.history.pop();
                }
            }
        }

        println!("Bye!");
    }
}
