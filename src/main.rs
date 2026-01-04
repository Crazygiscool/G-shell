use std::io::{self, Write};
use pathsearch::find_executable_in_path;
use std::os::unix::process::CommandExt;


fn main() {
    let mut command: String = String::new();

    // Parse arguments supporting single and double quotes:
    // - whitespace separates words (consecutive whitespace collapsed)
    // - single or double quotes preserve spaces and characters literally
    // - adjacent quoted/ unquoted parts without intervening whitespace are concatenated
    fn parse_args(input: &str) -> Vec<String> {
        let s = input.trim_end_matches(|c| c == '\n' || c == '\r');
        let mut args: Vec<String> = Vec::new();
        let mut cur = String::new();
        let mut in_sq = false;
        let mut in_dq = false;
        let mut escape = false;

        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if escape {
                // interpret common escape sequences when not in single quotes
                let pushed = match c {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '\\' => '\\',
                    '\'' => '\'',
                    '"' => '"',
                    d if d.is_ascii_digit() => {
                        // parse up to 3 octal digits (including this one)
                        let mut val = d as u32 - '0' as u32;
                        for _ in 0..2 {
                            if let Some(peek) = chars.peek() {
                                if peek.is_ascii_digit() && *peek <= '7' {
                                    let dd = chars.next().unwrap();
                                    val = val * 8 + (dd as u32 - '0' as u32);
                                } else {
                                    break;
                                }
                            }
                        }
                        // safety: map to a valid char (0-255)
                        let byte = (val & 0xff) as u8;
                        cur.push(byte as char);
                        escape = false;
                        continue;
                    }
                    other => other,
                };
                cur.push(pushed);
                escape = false;
                continue;
            }

            if c == '\\' && !in_sq {
                escape = true;
                continue;
            }

            if c == '\'' && !in_dq {
                in_sq = !in_sq;
                continue;
            }
            if c == '"' && !in_sq {
                in_dq = !in_dq;
                continue;
            }

            if c.is_whitespace() && !in_sq && !in_dq {
                if !cur.is_empty() {
                    args.push(cur);
                    cur = String::new();
                }
            } else {
                cur.push(c);
            }
        }

        if escape {
            // trailing backslash: keep it as literal backslash
            cur.push('\\');
        }

        if !cur.is_empty() {
            args.push(cur);
        }

        args
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

        let tokens = parse_args(command);
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
