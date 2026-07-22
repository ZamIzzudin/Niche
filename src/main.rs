mod cli;
mod core;

use cli::repl::Repl;
use core::client::Client;
use core::config::Config;

fn main() {
    let config = Config::load("config.json");
    let client = Client::new(config);
    let mut repl = Repl::new(client);
    repl.run();
}
