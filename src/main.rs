use rustyline::{Editor, Config};
use rustyline::config::{BellStyle, CompletionType};
mod commands;
mod parser;

use parser::process::process_command;
use crate::parser::helper::ShellHelper;
use crate::parser::pipeline;
use crate::commands::history::history;
use crate::parser::tokenize::tokenize; // Import your tokenizer

fn main() -> rustyline::Result<()> {
    // 1. Create your config
    let config = Config::builder()
        .completion_type(CompletionType::List)
        .bell_style(BellStyle::Audible)
        .build();

    // FIX: Supply exactly ONE generic argument (the Helper). 
    // Supply exactly ONE argument (config) to with_config.
    let mut rl = Editor::<ShellHelper>::with_config(config)?;

    // 2. Set the helper separately after initialization
    rl.set_helper(Some(ShellHelper));

    // 3. Load history separately
    let _ = rl.load_history(".shell_history");

    loop {
        let readline = rl.readline("$ ");
        match readline {
            Ok(buffer) => {
                let trimmed = buffer.trim();
                if trimmed.is_empty() { continue; }

                // FIX FOR CODECRAFTERS: Add the current command to history FIRST.
                let _ = rl.add_history_entry(trimmed);

                // 4. Collect current history for builtins AFTER adding current line.
                let history_vec: Vec<String> = rl.history()
                    .iter()
                    .map(|s| s.to_string())
                    .collect();

                // 5. Tokenize to separate the command from arguments (e.g., "history", "2")
                let mut tokens = tokenize(trimmed);
                if tokens.is_empty() { continue; }
                let command = tokens.remove(0);

                // 6. Execution routing
                if trimmed.contains('|') {
                    pipeline::execute_pipeline(trimmed, &history_vec);
                } else if command == "history" {
                    // Pass the history list and the remaining tokens (arguments)
                    history(&history_vec, &tokens);
                } else {
                    process_command(trimmed);
                }
            }
            Err(_) => break, 
        }
    }
    
    let _ = rl.save_history(".shell_history");
    Ok(())
}
