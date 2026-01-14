use rustyline::{Editor, Config};
use rustyline::config::{BellStyle, CompletionType};
mod commands;
mod parser;

use parser::process::process_command;
use crate::parser::helper::ShellHelper;
use crate::parser::pipeline;

fn main() -> rustyline::Result<()> {
    // 2026 Config: Enable audible bell and list-style completion
    let config = Config::builder()
        .completion_type(CompletionType::List)
        .bell_style(BellStyle::Audible)
        .build();

    // Editor in 2026 takes 1 generic: Editor<ShellHelper>
    let mut rl = Editor::<ShellHelper>::with_config(config)?;
    rl.set_helper(Some(ShellHelper));

    loop {
        let readline = rl.readline("$ ");
        match readline {
            Ok(buffer) => {
                let trimmed = buffer.trim();
                if !trimmed.is_empty() {
                    // Modern add_history_entry returns Result<bool>
                    let _ = rl.add_history_entry(trimmed);

                    // Check for pipelines
                    if trimmed.contains('|') {
                        pipeline::execute_pipeline(trimmed);
                    } else {
                        process_command(trimmed);
                    }
                }
            }
            Err(_) => break, // Exit on Ctrl+C or Ctrl+D
        }
    }
    Ok(())
}
