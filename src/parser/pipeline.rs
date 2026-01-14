use std::process::{Command, Stdio, Child};
use crate::parser::tokenize::tokenize;

pub fn execute_pipeline(line: &str) {
    // 1. Split the line into individual commands by '|'
    let command_segments: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
    
    let mut previous_stdout: Option<Stdio> = None;
    let mut children: Vec<Child> = Vec::new();

    let total_cmds = command_segments.len();

    for (i, segment) in command_segments.into_iter().enumerate() {
        let mut parts = tokenize(segment);
        if parts.is_empty() { continue; }

        let program = parts.remove(0);
        let mut cmd = Command::new(program);
        cmd.args(parts);

        // Link Stdin: Use take() to move the Stdio out of the Option
        // This avoids "moved value" errors in the loop
        if let Some(stdin_source) = previous_stdout.take() {
            cmd.stdin(stdin_source);
        }

        // Link Stdout
        if i < total_cmds - 1 {
            // Not the last command: pipe it
            cmd.stdout(Stdio::piped());
        } else {
            // Last command: inherit shell's terminal output
            cmd.stdout(Stdio::inherit());
        }

        match cmd.spawn() {
            Ok(mut child) => {
                // If there's a next command, capture this child's stdout
                if i < total_cmds - 1 {
                    if let Some(out) = child.stdout.take() {
                        previous_stdout = Some(Stdio::from(out));
                    }
                }
                children.push(child);
            }
            Err(e) => {
                eprintln!("Pipeline error: {}", e);
                // Kill previously spawned children in the pipe to prevent hanging
                for mut c in children { let _ = c.kill(); }
                return;
            }
        }
    }

    // 2. Wait for all child processes to finish
    for mut child in children {
        let _ = child.wait();
    }
}
