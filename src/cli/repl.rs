use crate::cli::spinner::Spinner;
use crate::core::client::Client;
use crate::core::types::Message;
use std::io::{self, Write};

pub struct Repl {
    client: Client,
    history: Vec<Message>,
}

impl Repl {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            history: Vec::new(),
        }
    }

    pub fn run(&mut self) {
        println!("niche CLI - Interactive REPL");
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

            let sp = Spinner::start("thinking...");
            let result = self.client.chat(&self.history);
            sp.stop();

            match result {
                Ok(content) => {
                    println!("{content}\n");
                    self.history.push(Message::assistant(content));
                }
                Err(e) => {
                    eprintln!("{e}\n");
                    self.history.pop();
                }
            }
        }

        println!("Bye!");
    }
}
