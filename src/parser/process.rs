use std::os::unix::io::RawFd;

use crate::parser::tokenize::tokenize;
use crate::parser::expand::expand_tokens;
use crate::parser::redirect_stdout::{
    redirect_stdout_to, redirect_stdout_append,
    redirect_stderr_to, redirect_stderr_append,
    redirect_stdin_from, restore_fd,
};

use crate::commands::cd::cd;
use crate::commands::echo::echo;
use crate::commands::pwd::pwd;
use crate::commands::r#type::r#type;
use crate::commands::execute::execute;

pub fn process_command(command: &str, last_exit_code: i32) -> i32 {
        let registry: &[[&str; 2]; 6] = &[
            ["echo", "builtin"],
            ["type", "builtin"],
            ["exit", "builtin"],
            ["pwd", "builtin"],
            ["cd", "builtin"],
            ["history", "builtin"],
        ];

        let raw_tokens = tokenize(command);
        if raw_tokens.is_empty() {
            return 0;
        }

        let tokens = expand_tokens(&raw_tokens, last_exit_code);

        let mut redirects: Vec<(String, String, i32)> = Vec::new();
        let mut remaining: Vec<String> = Vec::new();

        let mut i = 0;
        while i < tokens.len() {
            let op = &tokens[i];
            if op == ">" || op == ">>" || op == "<"
                || (op.ends_with('>') && op[..op.len()-1].chars().all(|c| c.is_ascii_digit()))
                || (op.ends_with(">>") && op[..op.len()-2].chars().all(|c| c.is_ascii_digit()))
                || (op.ends_with('<') && op[..op.len()-1].chars().all(|c| c.is_ascii_digit()))
            {
                if i + 1 >= tokens.len() {
                    eprintln!("syntax error: expected filename after redirection");
                    return 1;
                }
                redirects.push((tokens[i].clone(), tokens[i + 1].clone(), parse_fd(&tokens[i])));
                i += 2;
            } else {
                remaining.push(tokens[i].clone());
                i += 1;
            }
        }

        if remaining.is_empty() {
            return 0;
        }

        let cmd = remaining[0].as_str();
        let args_vec: Vec<&str> = remaining.iter().skip(1).map(|s| s.as_str()).collect();

        let mut saved_fds: Vec<(i32, RawFd)> = Vec::new();

        for (op, filename, fd) in &redirects {
            let saved = match (op.as_str(), *fd) {
                (">", 1) | ("1>", 1) => redirect_stdout_to(filename).map(|f| (1, f)),
                (">>", 1) | ("1>>", 1) => redirect_stdout_append(filename).map(|f| (1, f)),
                (">", 2) | ("2>", 2) => redirect_stderr_to(filename).map(|f| (2, f)),
                (">>", 2) | ("2>>", 2) => redirect_stderr_append(filename).map(|f| (2, f)),
                ("<", 0) | ("0<", 0) => redirect_stdin_from(filename).map(|f| (0, f)),
                _ => {
                    eprintln!("{}: unsupported redirection", op);
                    None
                }
            };
            if let Some(pair) = saved {
                saved_fds.push(pair);
            }
        }

        let exit_code = match cmd {
            "exit" => {
                let code = args_vec.first().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                std::process::exit(code);
            }

            "echo" => {
                echo(&args_vec);
                0
            }

            "type" => {
                if let Some(first) = args_vec.first() {
                    r#type(first, registry);
                    0
                } else {
                    eprintln!("type: missing operand");
                    1
                }
            }

            "pwd" => {
                pwd();
                0
            }

            "cd" => {
                if let Some(first) = args_vec.first() {
                    cd(first);
                } else {
                    cd("");
                }
                0
            }

            _ => execute(cmd, &args_vec, redirects.first().map(|(op, _filename, fd)| (op.as_str(), *fd))),
        };

        for (target, saved_fd) in saved_fds.into_iter().rev() {
            restore_fd(saved_fd, target);
        }

        exit_code
}

fn parse_fd(op: &str) -> i32 {
    if op == ">" || op == ">>" || op == "<" {
        if op == "<" { 0 } else { 1 }
    } else {
        let num_part = if op.ends_with(">>") {
            &op[..op.len()-2]
        } else {
            &op[..op.len()-1]
        };
        num_part.parse::<i32>().unwrap_or(if op.contains('<') { 0 } else { 1 })
    }
}
