use std::io::{self, Write};
use pathsearch::find_executable_in_path;
use std::os::unix::process::CommandExt;


fn main() {
    let mut command: String = String::new();

    // Parse arguments supporting single and double quotes:
    // - whitespace separates words (consecutive whitespace collapsed)
    // - single or double quotes preserve spaces and characters literally
    // - adjacent quoted/ unquoted parts without intervening whitespace are concatenated
    fn tokenize(input: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut cur = String::new();

        let mut chars = input.chars().peekable();

        let mut in_single = false;
        let mut in_double = false;

        while let Some(c) = chars.next() {
            match c {
                // -------------------------
                // SINGLE QUOTES
                // -------------------------
                '\'' if !in_double => {
                    in_single = !in_single;
                }

                // -------------------------
                // DOUBLE QUOTES
                // -------------------------
                '"' if !in_single => {
                    in_double = !in_double;
                }

                // -------------------------
                // BACKSLASH HANDLING
                // -------------------------
                '\\' => {
                    if in_single {
                        // literal backslash inside single quotes
                        cur.push('\\');
                    } else if in_double {
                        // only \" and \\ are special
                        match chars.peek() {
                            Some('"') => {
                                chars.next();
                                cur.push('"');
                            }
                            Some('\\') => {
                                chars.next();
                                cur.push('\\');
                            }
                            Some('\n') => {
                                chars.next(); // remove newline
                            }
                            Some(_) => {
                                // literal backslash + char
                                cur.push('\\');
                                cur.push(chars.next().unwrap());
                            }
                            None => cur.push('\\'),
                        }
                    } else {
                        // outside quotes
                        match chars.peek() {
                            Some('\n') => {
                                chars.next(); // remove newline
                            }
                            Some(_) => {
                                cur.push(chars.next().unwrap());
                            }
                            None => cur.push('\\'),
                        }
                    }
                }

                // -------------------------
                // WHITESPACE SPLITTING
                // -------------------------
                c if c.is_whitespace() && !in_single && !in_double => {
                    if !cur.is_empty() {
                        tokens.push(cur);
                        cur = String::new();
                    }
                }

                // -------------------------
                // NORMAL CHARACTER
                // -------------------------
                _ => cur.push(c),
            }
        }

        if !cur.is_empty() {
            tokens.push(cur);
        }

        tokens
    }

    fn echo(args: &[&str]) {
        println!("{}", args.join(" "));
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
                std::env::set_current_dir(home).unwrap_or_else(|_e| {
                    eprintln!("cd: {}: No such file or directory", directory);
                });
            }
            return;
        } else if directory == ".." {
            if let Some(parent) = std::env::current_dir().unwrap().parent() {
                std::env::set_current_dir(parent).unwrap_or_else(|_e| {
                    eprintln!("cd: {}: No such file or directory", directory);
                });
            }
            return;
        } else if directory == "." {
            return;
        } else if directory == "~" {
            if let Some(home) = std::env::var_os("HOME") {
                std::env::set_current_dir(home).unwrap_or_else(|_e| {
                    eprintln!("cd: {}: No such file or directory", directory);
                });
            }
            return;
        }
        match std::env::set_current_dir(directory) {
            Ok(_) => (),
            Err(_e) => eprintln!("cd: {}: No such file or directory", directory),
        }
    }

    fn r#type(command: &str, regix: &[[&str; 2]; 5]) {
        if let Some(entry) = regix.iter().find(|cmd| cmd[0] == command) {
            println!("{} is a shell {}", entry[0], entry[1]);
        } else if find_executable_in_path(command).is_some() {
            println!("{} is {}", command, find_executable_in_path(command).unwrap().display());
        } else {
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

    fn process_command(command: &str) {
        let regix: &[[&str; 2]; 5] = &[
            ["echo", "builtin"],
            ["type", "builtin"],
            ["exit", "builtin"],
            ["pwd", "builtin"],
            ["cd", "builtin"],
        ];

        let tokens = tokenize(command);
        if tokens.is_empty() {
            return;
        }

        let cmd = tokens[0].as_str();
        let args_vec: Vec<&str> = tokens.iter().skip(1).map(|s| s.as_str()).collect();
        let args_joined = args_vec.join(" ");

        match cmd {
            "exit" => std::process::exit(0),
            "echo" => echo(&args_vec),
            "type" => r#type(&args_joined, regix),
            "pwd" => pwd(),
            "cd" => cd(&args_joined),
            _ => execute(cmd, &args_vec),
        }
    }

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut command).unwrap();

        if command.is_empty() {
            command.clear();
            continue;
        }

        process_command(&command);
        command.clear();
    }
}
