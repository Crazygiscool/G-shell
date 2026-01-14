use rustyline::{Editor, Config};
mod commands;
mod parser;

use parser::process::process_command;

use crate::parser::helper::ShellHelper;

fn main() {
    let config = Config::builder().build();
    let mut rl = Editor::<ShellHelper>::with_config(config).unwrap();
    rl.set_helper(Some(ShellHelper));

    loop {
        let readline = rl.readline("$ ");
        match readline {
            Ok(buffer) => {
                process_command(&buffer);
            }
            Err(_) => break,
        }
    }
}
