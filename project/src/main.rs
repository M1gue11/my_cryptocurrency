mod cli;
mod cli_handler;
mod globals;
mod model;
mod security_utils;
mod utils;

use clap::Parser;
use cli::Cli;
use cli_handler::handle_command;

fn main() {
    let cli = Cli::parse();
    handle_command(cli.command);
}
