use rustyline::{Editor, Config};
use rustyline::config::{BellStyle, CompletionType};
mod commands;
mod parser;

use parser::process::process_command;
use crate::parser::helper::ShellHelper;
use crate::parser::pipeline;
use crate::commands::history::history;

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

    // 3. Load history separately (not as a constructor argument)
    let _ = rl.load_history(".shell_history");

    loop {
        let readline = rl.readline("$ ");
        match readline {
            Ok(buffer) => {
                let trimmed = buffer.trim();
                if trimmed.is_empty() { continue; }

                // FIX FOR CODECRAFTERS: Add the current command to history FIRST.
                // This ensures the current command (e.g., "history") appears in its own output.
                let _ = rl.add_history_entry(trimmed);

                // 4. Collect current history for builtins AFTER adding the current line.
                let history_vec: Vec<String> = rl.history()
                    .iter()
                    .map(|s| s.to_string())
                    .collect();

                // 5. Execution routing
                if trimmed.contains('|') {
                    // Pipeline logic handles history redirection
                    pipeline::execute_pipeline(trimmed, &history_vec);
                } else if trimmed == "history" {
                    // Direct builtin call including the self-entry
                    history(&history_vec);
                } else {
                    process_command(trimmed);
                }
            }
            Err(_) => break, // Exit on Ctrl+C or Ctrl+D
        }
    }
    
    // Save history before exiting
    let _ = rl.save_history(".shell_history");
    Ok(())
}
