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

    // 2. Supply ONLY ONE argument (config) to with_config.
    // The Editor struct requires two generic types: Editor<Helper, History>.
    // Use the wildcard '_' to let the compiler infer the default history type.
    let mut rl = Editor::<ShellHelper>::with_config(config)?;

    // 3. Set the helper separately after initialization
    rl.set_helper(Some(ShellHelper));

    // 4. Load history separately (not as a constructor argument)
    let _ = rl.load_history(".shell_history");

    loop {
        let readline = rl.readline("$ ");
        match readline {
            Ok(buffer) => {
                let trimmed = buffer.trim();
                if trimmed.is_empty() { continue; }

                // Collect current history for builtins
                let history_vec: Vec<String> = rl.history()
                    .iter()
                    .map(|s| s.to_string())
                    .collect();

                // add_history_entry returns Result<bool> in 2026
                let _ = rl.add_history_entry(trimmed);

                // Execution routing
                if trimmed.contains('|') {
                    pipeline::execute_pipeline(trimmed, &history_vec);
                } else if trimmed == "history" {
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
