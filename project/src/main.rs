mod bd;
mod front;
mod globals;
mod model;
mod security_utils;
mod utils;

#[cfg(test)]
mod tests;

use bd::Db;
use front::run_interactive_mode;

fn main() {
    let db = Db::open(None).unwrap();
    let _ = db.init_schema();

    run_interactive_mode();
}
