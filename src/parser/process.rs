use std::os::unix::io::RawFd;

use crate::parser::tokenize::tokenize;
use crate::parser::expand::expand_tokens;
use crate::parser::glob::expand_globs;
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
use crate::commands::env::{export_var, unset_var, set_vars, env_vars};
use crate::commands::test::test_builtin;
use crate::commands::help::help_cmd;
use std::sync::Mutex;

static ALIASES: std::sync::LazyLock<Mutex<std::collections::HashMap<String, String>>> =
    std::sync::LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

pub fn process_command(command: &str, last_exit_code: i32) -> i32 {
        let registry: &[[&str; 2]; 16] = &[
            ["echo", "builtin"],
            ["type", "builtin"],
            ["exit", "builtin"],
            ["pwd", "builtin"],
            ["cd", "builtin"],
            ["history", "builtin"],
            ["export", "builtin"],
            ["unset", "builtin"],
            ["set", "builtin"],
            ["env", "builtin"],
            ["source", "builtin"],
            ["test", "builtin"],
            ["[", "builtin"],
            ["alias", "builtin"],
            ["unalias", "builtin"],
            ["help", "builtin"],
        ];

        let raw_tokens = tokenize(command);
        if raw_tokens.is_empty() {
            return 0;
        }

        let expanded = expand_tokens(&raw_tokens, last_exit_code);
        let tokens = expand_globs(&expanded);

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

        // FOO=bar cmd support: extract leading KEY=VAL tokens
        let mut env_overrides: Vec<(String, String)> = Vec::new();
        let mut cmd_index = 0;
        for token in &remaining {
            if let Some(eq_pos) = token.find('=') {
                if eq_pos > 0 && token.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '=') {
                    let name = token[..eq_pos].to_string();
                    let value = token[eq_pos + 1..].to_string();
                    env_overrides.push((name, value));
                    cmd_index += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        let cmd = if cmd_index < remaining.len() { remaining[cmd_index].as_str() } else { return 0; };
        let args_vec: Vec<&str> = remaining.iter().skip(cmd_index + 1).map(|s| s.as_str()).collect();

        // Push env overrides
        let originals: Vec<(String, Option<String>)> = env_overrides.iter().map(|(k, v)| {
            let old = std::env::var(k).ok();
            unsafe { std::env::set_var(k, v); }
            (k.clone(), old)
        }).collect();

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
                if args_vec.first().map(|s| *s == "--help").unwrap_or(false) {
                    println!("echo: echo [string ...]");
                    println!("    Write arguments to standard output.");
                    0
                } else {
                    echo(&args_vec);
                    0
                }
            }

            "type" => {
                if args_vec.first().map(|s| *s == "--help").unwrap_or(false) {
                    println!("type: type [name ...]");
                    println!("    Display information about command type.");
                    0
                } else if let Some(first) = args_vec.first() {
                    r#type(first, registry);
                    0
                } else {
                    eprintln!("type: missing operand");
                    1
                }
            }

            "pwd" => {
                if args_vec.first().map(|s| *s == "--help").unwrap_or(false) {
                    println!("pwd: pwd");
                    println!("    Print the current working directory.");
                    0
                } else {
                    pwd();
                    0
                }
            }

            "cd" => {
                if args_vec.first().map(|s| *s == "--help").unwrap_or(false) {
                    println!("cd: cd [dir]");
                    println!("    Change the current working directory.");
                    0
                } else {
                    if let Some(first) = args_vec.first() {
                        cd(first);
                    } else {
                        cd("");
                    }
                    0
                }
            }

            "export" => {
                if args_vec.is_empty() {
                    set_vars();
                } else {
                    export_var(&args_vec);
                }
                0
            }

            "unset" => {
                unset_var(&args_vec);
                0
            }

            "set" => {
                set_vars();
                0
            }

            "env" => {
                env_vars();
                0
            }

            "source" => {
                if let Some(path) = args_vec.first() {
                    match std::fs::read_to_string(path) {
                        Ok(contents) => {
                            for line in contents.lines() {
                                let line = line.trim();
                                if line.is_empty() || line.starts_with('#') {
                                    continue;
                                }
                                if line.contains('|') {
                                    crate::parser::pipeline::execute_pipeline(line, &[], last_exit_code);
                                } else {
                                    process_command(line, last_exit_code);
                                }
                            }
                            0
                        }
                        Err(e) => {
                            eprintln!("{}: {}: {}", cmd, path, e);
                            1
                        }
                    }
                } else {
                    eprintln!("source: missing filename");
                    1
                }
            }

            "alias" => {
                let mut alias_table = ALIASES.lock().unwrap();
                for arg in &args_vec {
                    if let Some(eq_pos) = arg.find('=') {
                        let name = arg[..eq_pos].to_string();
                        let value = arg[eq_pos + 1..].trim_matches('\'').trim_matches('"').to_string();
                        alias_table.insert(name, value);
                    } else {
                        if let Some(value) = alias_table.get(*arg) {
                            println!("alias {}='{}'", arg, value);
                        }
                    }
                }
                0
            }

            "unalias" => {
                let mut alias_table = ALIASES.lock().unwrap();
                for arg in &args_vec {
                    alias_table.remove(*arg);
                }
                0
            }

            "help" => {
                help_cmd(&args_vec);
                0
            }

            "test" | "[" => test_builtin(&args_vec),

            _ => execute(cmd, &args_vec, redirects.first().map(|(op, _filename, fd)| (op.as_str(), *fd))),
        };

        // Restore original env vars
        for (name, old) in originals.into_iter().rev() {
            match old {
                Some(v) => unsafe { std::env::set_var(&name, v); },
                None => unsafe { std::env::remove_var(&name); },
            }
        }

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
