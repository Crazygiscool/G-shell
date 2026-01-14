mod commands;
mod parser;

use parser::shell;

fn main() -> rustyline::Result<()> {
    // 1. Initialize the shell
    // This will resolve HISTFILE and load history from disk
    let mut shell = match shell::Shell::new() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize shell: {}", e);
            return Err(e);
        }
    };

    // 2. Start the REPL (Read-Eval-Print Loop)
    // This will save history to HISTFILE automatically when it returns
    if let Err(e) = shell.run() {
        eprintln!("Shell error: {}", e);
        return Err(e);
    }

    Ok(())
}