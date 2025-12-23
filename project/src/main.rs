mod bd;
mod front;
mod globals;
mod model;
mod security_utils;
mod utils;

#[cfg(test)]
mod tests;

use bd::init_db;
use front::run_interactive_mode;

fn main() {
    init_db(None).expect("Failed to initialize database");

    run_interactive_mode();
}
