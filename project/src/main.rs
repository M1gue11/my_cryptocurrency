mod cli;
mod cli_handler;
mod globals;
mod model;
mod security_utils;
mod utils;

use cli_handler::run_interactive_mode;

fn main() {
    run_interactive_mode();
}
