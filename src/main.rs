// src/main.rs
mod commands;
mod parser;

use parser::shell;

fn main() -> rustyline::Result<()> {
    // Initialize and start the shell
    let mut shell = shell::Shell::new()?;
    shell.run()
}
