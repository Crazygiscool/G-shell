use std::io::{self, Write};
use pathsearch::find_executable_in_path;
use std::os::unix::process::CommandExt;
use std::fs::File;
use std::process::Command;
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::fs::OpenOptions;

fn main() {
    let mut command: String = String::new();

    fn tokenize(input: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut cur = String::new();

        let mut chars = input.chars().peekable();
        let mut in_single = false;
        let mut in_double = false;

        while let Some(c) = chars.next() {
            match c {
                // SINGLE QUOTES
                '\'' if !in_double => {
                    in_single = !in_single;
                }

                // DOUBLE QUOTES
                '"' if !in_single => {
                    in_double = !in_double;
                }

                // BACKSLASH HANDLING
                '\\' => {
                    if in_single {
                        cur.push('\\');
                    } else if in_double {
                        match chars.next() {
                            Some('"') => cur.push('"'),
                            Some('\\') => cur.push('\\'),
                            Some('\n') => { /* line continuation */ }
                            Some(ch) => {
                                cur.push('\\');
                                cur.push(ch);
                            }
                            None => cur.push('\\'),
                        }
                    } else {
                        match chars.next() {
                            Some('\n') => { /* line continuation */ }
                            Some(ch) => cur.push(ch),
                            None => cur.push('\\'),
                        }
                    }
                }

                // WHITESPACE SPLITTING
                c if c.is_whitespace() && !in_single && !in_double => {
                    if !cur.is_empty() {
                        tokens.push(cur);
                        cur = String::new();
                    }
                }

                // FD REDIRECTION (e.g., 1>, 2>)
                c if !in_single && !in_double && c.is_ascii_digit() => {
                    if let Some('>') = chars.peek().copied() {
                        chars.next(); // consume '>'
                        if !cur.is_empty() {
                            tokens.push(cur.clone());
                            cur.clear();
                        }
                        let mut op = String::new();
                        op.push(c);
                        op.push('>');
                        tokens.push(op);
                    } else {
                        cur.push(c);
                    }
                }

                // PLAIN REDIRECTION OPERATOR ">"
                '>' if !in_single && !in_double => {
                    if !cur.is_empty() {
                        tokens.push(cur.clone());
                        cur.clear();
                    }
                    tokens.push(">".to_string());
                }

                // NORMAL CHARACTER
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
        } else if let Some(path) = find_executable_in_path(command) {
            println!("{} is {}", command, path.display());
        } else {
            println!("{}: not found", command);
        }
    }

    fn execute(cmd: &str, args: &[&str], redirect: Option<(&str, i32)>) {
        if let Some(path) = find_executable_in_path(cmd) {
            let mut child = Command::new(&path);

            child.arg0(cmd);
            child.args(args);

            if let Some((filename, fd)) = redirect {
                if fd == 1 {
                    match File::create(filename) {
                        Ok(file) => {
                            child.stdout(file);
                        }
                        Err(e) => {
                            eprintln!("{}: {}", filename, e);
                            return;
                        }
                    }
                }
            }

            match child.status() {
                Ok(status) => {
                    if !status.success() {
                        return;
                    }
                }
                Err(e) => {
                    return;
                }
            }
        } else {
            eprintln!("{}: command not found", cmd);
        }
    }

    fn redirect_stdout_to(filename: &str) -> Option<RawFd> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(filename)
            .ok()?;

        let new_fd = file.as_raw_fd();
        let old_fd = unsafe { libc::dup(1) };

        unsafe {
            libc::dup2(new_fd, 1);
        }

        Some(old_fd)
    }

    fn restore_stdout(old_fd: RawFd) {
        unsafe {
            libc::dup2(old_fd, 1);
            libc::close(old_fd);
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

        let mut tokens = tokenize(command);
        if tokens.is_empty() {
            return;
        }

        // Handle redirection: ">", "1>", "2>", etc. (we only honor fd 1)
        let mut redirect: Option<(String, i32)> = None;

        if let Some(i) = tokens.iter().position(|t| t == ">" || (t.ends_with('>') && t[..t.len()-1].chars().all(|c| c.is_ascii_digit()))) {
            if i + 1 >= tokens.len() {
                eprintln!("syntax error: expected filename after redirection");
                return;
            }

            let op = tokens[i].clone();
            let fd = if op == ">" {
                1
            } else {
                op[..op.len() - 1].parse::<i32>().unwrap_or(1)
            };

            let filename = tokens[i + 1].clone();
            redirect = Some((filename, fd));

            tokens.drain(i..=i + 1);
        }

        if tokens.is_empty() {
            return;
        }

        let cmd = tokens[0].as_str();
        let args_vec: Vec<&str> = tokens.iter().skip(1).map(|s| s.as_str()).collect();

        let mut old_stdout: Option<RawFd> = None;

        if let Some((filename, fd)) = redirect.as_ref() {
            if *fd == 1 {
                old_stdout = redirect_stdout_to(filename);
            }
        }

        match cmd {
            "exit" => std::process::exit(0),

            "echo" => echo(&args_vec),

            "type" => {
                if let Some(first) = args_vec.first() {
                    r#type(first, regix);
                } else {
                    eprintln!("type: missing operand");
                }
            }

            "pwd" => pwd(),

            "cd" => {
                if let Some(first) = args_vec.first() {
                    cd(first);
                } else {
                    cd("");
                }
            }

            _ => execute(cmd, &args_vec, redirect.as_ref().map(|(f, fd)| (f.as_str(), *fd))),
        }

        if let Some(fd) = old_stdout {
            restore_stdout(fd);
        }
    }

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        if io::stdin().read_line(&mut command).is_err() {
            break;
        }

        if command.is_empty() {
            command.clear();
            continue;
        }

        process_command(&command);
        command.clear();
    }
}
