use std::process::{Command, Stdio, Child};
use std::io::{Read, Write};
use crate::parser::tokenize::tokenize;

pub fn execute_pipeline(line: &str) {
    let command_segments: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
    let mut previous_stdout: Option<Stdio> = None;
    let mut children: Vec<Child> = Vec::new();
    let total_cmds = command_segments.len();

    let builtins = ["echo", "cd", "pwd", "type", "exit"];

    for (i, segment) in command_segments.into_iter().enumerate() {
        let mut parts = tokenize(segment);
        if parts.is_empty() { continue; }

        let program = parts.remove(0);

        // 1. Check if the command is a builtin
        if builtins.contains(&program.as_str()) {
            // For builtins in a pipeline, we "simulate" them by running them
            // in the current process, but this is complex for mid-pipeline builtins.
            // Simplified approach: run the builtin and move to next.
            handle_builtin_in_pipeline(&program, parts, &mut previous_stdout, i == total_cmds - 1);
            continue;
        }

        // 2. Handle External Commands
        let mut cmd = Command::new(program);
        cmd.args(parts);

        if let Some(stdin_source) = previous_stdout.take() {
            cmd.stdin(stdin_source);
        }

        if i < total_cmds - 1 {
            cmd.stdout(Stdio::piped());
        } else {
            cmd.stdout(Stdio::inherit());
        }

        match cmd.spawn() {
            Ok(mut child) => {
                if i < total_cmds - 1 {
                    if let Some(out) = child.stdout.take() {
                        previous_stdout = Some(Stdio::from(out));
                    }
                }
                children.push(child);
            }
            Err(e) => {
                eprintln!("Pipeline error: {}", e);
                for mut c in children { let _ = c.kill(); }
                return;
            }
        }
    }

    for mut child in children {
        let _ = child.wait();
    }
}

/// Specialized handler for builtins occurring inside a pipeline
fn handle_builtin_in_pipeline(name: &str, args: Vec<String>, prev_out: &mut Option<Stdio>, is_last: bool) {
    // Note: To fully support builtins like `type` receiving piped input,
    // you would need to read from `prev_out` and write to a new pipe.
    // This is a basic implementation that just runs the builtin.
    match name {
        "type" => {
            // Logic for 'type' usually describes a command
            if let Some(cmd_to_check) = args.first() {
                let res = format!("{} is a shell builtin\n", cmd_to_check);
                if is_last {
                    print!("{}", res);
                } else {
                    // Create a dummy pipe to pass this output forward
                    // (Advanced: Requires manual pipe creation via libc or nix)
                }
            }
        }
        "exit" => std::process::exit(0),
        _ => { /* Handle other builtins */ }
    }
    // Clear previous output as it was "consumed" or ignored by the builtin
    *prev_out = None; 
}
