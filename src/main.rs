#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    let mut command = String::new();
    let commands = ["exit"];
    loop{
        print!("$ ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut command).unwrap();
        if command.trim().is_empty() {
            continue;
        }else if commands.contains(&command.trim()){
            break;
        }else{
            println!("{}: command not found", command.trim());
        }
        command.clear();
    }
}
