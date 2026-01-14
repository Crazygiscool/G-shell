use std::os::unix::io::RawFd;

use crate::parser::tokenize::tokenize;
use crate::parser::redirect_stdout::{redirect_stdout_to, restore_stdout};

use crate::commands::cd::cd;
use crate::commands::echo::echo;
use crate::commands::pwd::pwd;
use crate::commands::r#type::r#type;
use crate::commands::execute::execute;

pub fn process_command(command: &str) {
        let regix: &[[&str; 2]; 6] = &[
            ["echo", "builtin"],
            ["type", "builtin"],
            ["exit", "builtin"],
            ["pwd", "builtin"],
            ["cd", "builtin"],
            ["history", "builtin"],
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