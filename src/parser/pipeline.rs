use std::process::{Command, Stdio};
use std::io::Write;
use os_pipe::pipe; // Ensure 'os_pipe' is in Cargo.toml
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

        // 1. Handle Builtins (e.g., echo)
        if ["echo", "cd", "pwd", "type", "exit"].contains(&program.as_str()) {
            if is_last {
                run_builtin(&program, parts);
                prev_stdin = None;
            } else {
                // Create a manual pipe to pass builtin output to the next command
                let (reader, mut writer) = pipe().expect("Pipe failed");
                
                // Get builtin output (e.g., "mango-grape\n")
                let output = get_builtin_output(&program, parts);
                let _ = writer.write_all(output.as_bytes());
                
                // Explicitly drop writer to send EOF so 'wc' knows to stop reading
                drop(writer); 

                prev_stdin = Some(Stdio::from(reader));
            }
            continue;
        }

        // 2. Handle External Commands (e.g., wc)
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
        let _ = child.wait();
    }
}

fn get_builtin_output(name: &str, args: Vec<String>) -> String {
    match name {
        "echo" => format!("{}\n", args.join(" ")), // mango-grape\n
        "pwd" => format!("{}\n", std::env::current_dir().unwrap().display()),
        _ => String::new(),
    }
}

fn run_builtin(name: &str, args: Vec<String>) {
    match name {
        "echo" => print!("{}", get_builtin_output(name, args)),
        "exit" => std::process::exit(0),
        "cd" => if let Some(path) = args.first() { let _ = std::env::set_current_dir(path); },
        _ => (),
    }
}
