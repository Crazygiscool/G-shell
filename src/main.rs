use rustyline::{Editor, Config};
use rustyline::config::{BellStyle, CompletionType}; // Fixed import
mod commands;
mod parser;

use parser::process::process_command;
use crate::parser::helper::ShellHelper;

fn main() {
    let config = Config::builder()
        .completion_type(CompletionType::List)
        .bell_style(BellStyle::Audible)
        .build();

    // Fixed: Editor takes 1 generic argument: Editor<ShellHelper, _> 
    // or just Editor<ShellHelper, History>
    let mut rl = Editor::<ShellHelper>::with_config(config).unwrap();
    rl.set_helper(Some(ShellHelper));

    loop {
        let readline = rl.readline("$ ");
        match readline {
            Ok(buffer) => {
                if !buffer.trim().is_empty() {
                    // Fixed: add_history_entry returns a Result (bool), 
                    // remove .unwrap() as it's often not needed or use let _ =
                    let _ = rl.add_history_entry(&buffer);
                    process_command(&buffer);
                }
            }
            Err(_) => break,
        }
    }
}
