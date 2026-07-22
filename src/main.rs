mod cli;
mod core;

use cli::repl::Repl;
use core::client::Client;
use core::config::Config;

#[tokio::main]
async fn main() {
    let config = Config::load("config.json");
    let system_prompt = config.system_prompt.clone();
    let client = Client::new(config);
    let mut repl = Repl::new(client, system_prompt);
    repl.run().await;
}
