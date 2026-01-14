use std::path::Path;
use crate::parser::tokenize::tokenize;

pub fn tab(command: &mut String) {

    // Tokenize the current buffer
    let tokens = tokenize(command);
    let last = tokens.last().map(|s| s.as_str()).unwrap_or("");

    // Decide what to complete
    let completion = if tokens.len() <= 1 {
        complete_command(last)
    } else {
        complete_path(last)
    };

    // Apply the completion
    if let Some(comp) = completion {
        if let Some(pos) = command.rfind(last) {
            command.replace_range(pos.., &comp);
        }
    }
}

// ------------------------------------------------------------
// COMMAND COMPLETION
// ------------------------------------------------------------

fn complete_command(prefix: &str) -> Option<String> {
    // Builtins
    let builtins = ["echo", "cd", "pwd", "type", "exit"];

    if let Some(cmd) = builtins.iter().find(|c| c.starts_with(prefix)) {
        return Some(cmd.to_string());
    }

    // PATH executables
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with(prefix) {
                        return Some(name);
                    }
                }
            }
        }
    }

    None
}

// ------------------------------------------------------------
// FILE / PATH COMPLETION
// ------------------------------------------------------------

fn complete_path(prefix: &str) -> Option<String> {
    let path = Path::new(prefix);

    // Determine directory + partial filename
    let (dir, partial) = if path.is_absolute() {
        (
            path.parent().unwrap_or(Path::new("/")),
            path.file_name().unwrap_or_default(),
        )
    } else {
        (
            path.parent().unwrap_or(Path::new(".")),
            path.file_name().unwrap_or_default(),
        )
    };

    let partial = partial.to_string_lossy();

    // Scan directory for matches
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&*partial) {
                let mut new = dir.to_path_buf();
                new.push(&name);
                return Some(new.to_string_lossy().to_string());
            }
        }
    }

    None
}
