use std::io::{self, Write};
use pathsearch::find_executable_in_path;

fn main() {
    let mut command: String = String::new();
    let regix: &[[&str; 2]; 3] = &[
        ["echo", "builtin"],
        ["type", "builtin"],
        ["exit", "builtin"],
    ];

    fn echo(string: &str) {
        println!("{}", string);
    }

    fn r#type(command: &str, regix: &[[&str; 2]; 3]) {
        if let Some(entry) = regix.iter().find(|cmd| cmd[0] == command) {
            println!("{} is a shell {}", entry[0], entry[1]);
        }else if find_executable_in_path(command).is_some() {
            println!("{} is {}", command, find_executable_in_path(command).unwrap().display());
        }else {
            println!("{}: not found", command);
        }
    }

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut command).unwrap();
        let mut parts = command.trim().split_whitespace();
        let cmd = parts.next().unwrap_or("");
        let mut content = parts.collect::<Vec<_>>().join(" ");
        
        
        if cmd.is_empty() {
            command.clear();
            continue;
        }
        
        match cmd {
            "exit" => break,
            "echo" => echo(&content),
            "type" => r#type(&content, regix),
            _ => println!("{}: command not found", cmd),
        }

        command.clear();
        content.clear();
    }
}
