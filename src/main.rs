use std::io::{self, Write};

fn main() {
    let mut command: String = String::new();

    fn echo(string: &str) {
        let echo_content = string[5..].trim();
        println!("{}", echo_content);
    }

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut command).unwrap();
        
        let cmd = command.trim();
        
        if cmd.is_empty() {
            command.clear();
            continue;
        }
        
        let command_name = cmd.split_whitespace().next().unwrap_or("");
        
        match command_name {
            "exit" => break,
            "echo" => echo(&command),
            _ => println!("{}: command not found", command_name),
        }

        command.clear();
    }
}
