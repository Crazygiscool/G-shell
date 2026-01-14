use std::process::{Command, Stdio};
use std::io::Write;
use std::env;
use std::fs;
use std::path::{PathBuf};
use os_pipe::pipe; 
use crate::parser::tokenize::tokenize;

pub fn execute_pipeline(line: &str) {
    let segments: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
    let mut prev_stdin: Option<Stdio> = None;
    let mut children = Vec::new();
    let total = segments.len();

    for (i, segment) in segments.into_iter().enumerate() {
        let mut parts = tokenize(segment);
        if parts.is_empty() { continue; }
        let program = parts.remove(0);
        let is_last = i == total - 1;

        // 1. Handle Builtins
        if ["echo", "cd", "pwd", "type", "exit"].contains(&program.as_str()) {
            if is_last {
                run_builtin(&program, parts);
                prev_stdin = None;
            } else {
                let (reader, mut writer) = pipe().expect("Pipe failed");
                let output = get_builtin_output(&program, parts);
                let _ = writer.write_all(output.as_bytes());
                drop(writer); // Send EOF to next command

                prev_stdin = Some(Stdio::from(reader));
            }
            continue;
        }

        // 2. Handle External Commands
        let mut cmd = Command::new(program);
        cmd.args(parts);

        if let Some(stdin) = prev_stdin.take() {
            cmd.stdin(stdin);
        }

        if !is_last {
            cmd.stdout(Stdio::piped());
        } else {
            cmd.stdout(Stdio::inherit());
        }

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
                for mut c in children { let _ = c.kill(); }
                return;
            }
        }
    }

    for mut child in children {
        let _ = child.wait().expect("Wait failed");
    }
}

/// Manual PATH search to replace the 'which' crate
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

fn get_builtin_output(name: &str, args: Vec<String>) -> String {
    match name {
        "echo" => format!("{}\n", args.join(" ")),
        "pwd" => format!("{}\n", env::current_dir().unwrap_or_default().display()),
        "type" => {
            if let Some(cmd) = args.first() {
                let builtins = ["echo", "cd", "pwd", "type", "exit"];
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

fn run_builtin(name: &str, args: Vec<String>) {
    match name {
        "echo" | "pwd" | "type" => {
            print!("{}", get_builtin_output(name, args));
            let _ = std::io::stdout().flush();
        }
        "exit" => {
            std::process::exit(0);
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
