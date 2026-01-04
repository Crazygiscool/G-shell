use std::io::{self, Write};
use pathsearch::find_executable_in_path;
use std::os::unix::process::CommandExt;


fn main() {
    let mut command: String = String::new();
    let regix: &[[&str; 2]; 5] = &[
        ["echo", "builtin"],
        ["type", "builtin"],
        ["exit", "builtin"],
        ["pwd", "builtin"],
        ["cd", "builtin"],
    ];

    fn echo(string: &str) {
        println!("{}", string);
    }

    fn pwd() {
        match std::env::current_dir() {
            Ok(path) => println!("{}", path.display()),
            Err(e) => eprintln!("Error getting current directory: {}", e),
        }
    }

    fn cd(directory: &str) {
        if directory.is_empty() {
            if let Some(home) = std::env::var_os("HOME") {
                std::env::set_current_dir(home).unwrap_or_else(|e| {
                    eprintln!("cd: {}: No such file or directory", directory);
                });
            }
            return;
        }else if directory == ".." {
            if let Some(parent) = std::env::current_dir().unwrap().parent() {
                std::env::set_current_dir(parent).unwrap_or_else(|e| {
                    eprintln!("cd: {}: No such file or directory", directory);
                });
            }
            return;
        }
        match std::env::set_current_dir(directory) {
            Ok(_) => (),
            Err(e) => eprintln!("cd: {}: No such file or directory", directory),
        }
    }

    fn r#type(command: &str, regix: &[[&str; 2]; 5]) { // change this if u changed the regix size
        if let Some(entry) = regix.iter().find(|cmd| cmd[0] == command) {
            println!("{} is a shell {}", entry[0], entry[1]);
        }else if find_executable_in_path(command).is_some() {
            println!("{} is {}", command, find_executable_in_path(command).unwrap().display());
        }else {
            println!("{}: not found", command);
        }
    }

    fn execute(cmd: &str, args: &[&str]) {
        if let Some(path) = find_executable_in_path(cmd) {
            match std::process::Command::new(path)
                .arg0(cmd)
                .args(args)
                .status()
            {
                Ok(status) => {
                    if !status.success() {
                        eprintln!("Command exited with non-zero status");
                    }
                }
                Err(e) => {
                    eprintln!("Failed to execute command: {}", e);
                }
            }
        } else {
            eprintln!("{}: command not found", cmd);
        }
    }

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut command).unwrap();
        let mut parts = command.trim().split_whitespace();
        let cmd = parts.next().unwrap_or("");
        let args: Vec<&str> = parts.clone().collect();
        let mut content = parts.collect::<Vec<_>>().join(" ");
        
        if cmd.is_empty() {
            command.clear();
            continue;
        }
        
        match cmd {
            "exit" => break,
            "echo" => echo(&content),
            "type" => r#type(&content, regix),
            "pwd" => pwd(),
            "cd" => cd(&content),
            _ => execute(cmd, &args),
        }

        command.clear();
        content.clear();
    }
}
