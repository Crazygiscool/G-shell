use std::process::{Command, Stdio};
use std::io::Write;
use std::env;
use std::fs;
use std::path::{PathBuf};
use os_pipe::pipe;
use crate::parser::tokenize::tokenize;

pub fn execute_pipeline(line: &str, history_data: &[String]) -> i32 {
    let segments: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
    let mut prev_stdin: Option<Stdio> = None;
    let mut children = Vec::new();
    let total = segments.len();
    let builtins = ["echo", "cd", "pwd", "type", "exit", "history"];
    let mut exit_code = 0;

    for (i, segment) in segments.into_iter().enumerate() {
        let mut parts = tokenize(segment);
        if parts.is_empty() { continue; }

        // Extract input redirect: look for "<" or "N<"
        let mut stdin_file: Option<String> = None;
        if let Some(pos) = parts.iter().position(|t| t == "<" || (t.ends_with('<') && t[..t.len()-1].chars().all(|c| c.is_ascii_digit()))) {
            if pos + 1 < parts.len() {
                stdin_file = Some(parts[pos + 1].clone());
                parts.drain(pos..=pos + 1);
            }
        }

        if parts.is_empty() { continue; }
        let program = parts.remove(0);
        let is_last = i == total - 1;

        if builtins.contains(&program.as_str()) {
            if is_last {
                run_builtin(&program, parts, history_data);
                prev_stdin = None;
            } else {
                let (reader, mut writer) = pipe().expect("Pipe failed");
                let output = get_builtin_output(&program, parts, history_data);
                let _ = writer.write_all(output.as_bytes());
                drop(writer);
                prev_stdin = Some(Stdio::from(reader));
            }
            continue;
        }

        let mut cmd = Command::new(&program);
        cmd.args(&parts);

        if let Some(file) = stdin_file {
            if let Ok(f) = fs::File::open(&file) {
                cmd.stdin(Stdio::from(f));
            } else {
                eprintln!("{}: No such file or directory", file);
                exit_code = 1;
                break;
            }
        } else if let Some(stdin) = prev_stdin.take() {
            cmd.stdin(stdin);
        }

        if !is_last {
            cmd.stdout(Stdio::piped());
        } else {
            cmd.stdout(Stdio::inherit());
        }
        cmd.stderr(Stdio::inherit());

        match cmd.spawn() {
            Ok(mut child) => {
                if !is_last {
                    if let Some(out) = child.stdout.take() {
                        prev_stdin = Some(Stdio::from(out));
                    }
                }
                children.push(child);
            }
            Err(e) => {
                eprintln!("Execution error: {}", e);
                for c in &mut children { let _ = c.kill(); }
                exit_code = 1;
                break;
            }
        }
    }

    for mut child in children {
        match child.wait() {
            Ok(status) => {
                if let Some(code) = status.code() {
                    exit_code = code;
                }
            }
            Err(_) => { exit_code = 1; }
        }
    }

    exit_code
}

fn find_in_path(cmd: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    for path in env::split_paths(&paths) {
        let full_path = path.join(cmd);
        if fs::metadata(&full_path).map(|m| m.is_file()).unwrap_or(false) {
            return Some(full_path);
        }
    }
    None
}

fn get_builtin_output(name: &str, args: Vec<String>, history_data: &[String]) -> String {
    match name {
        "echo" => format!("{}\n", args.join(" ")),
        "pwd" => format!("{}\n", env::current_dir().unwrap_or_default().display()),
        "history" => {
            history_data.iter().enumerate()
                .map(|(i, s)| format!("  {:>3}  {}\n", i + 1, s))
                .collect::<String>()
        },
        "type" => {
            if let Some(cmd) = args.first() {
                let builtins = ["echo", "cd", "pwd", "type", "exit", "history"];
                if builtins.contains(&cmd.as_str()) {
                    format!("{} is a shell builtin\n", cmd)
                } else if let Some(path) = find_in_path(cmd) {
                    format!("{} is {}\n", cmd, path.display())
                } else {
                    format!("{}: not found\n", cmd)
                }
            } else {
                String::new()
            }
        },
        _ => String::new(),
    }
}

fn run_builtin(name: &str, args: Vec<String>, history_data: &[String]) {
    match name {
        "echo" | "pwd" | "type" | "history" => {
            print!("{}", get_builtin_output(name, args, history_data));
            let _ = std::io::stdout().flush();
        }
        "exit" => {
            let code = args.first().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
            std::process::exit(code);
        }
        "cd" => {
            if let Some(path) = args.first() {
                if let Err(e) = env::set_current_dir(path) {
                    eprintln!("cd: {}: {}", path, e);
                }
            }
        }
        _ => (),
    }
}
