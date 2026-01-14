use std::path::Path;

// ------------------------------------------------------------
// COMMAND COMPLETION
// ------------------------------------------------------------

pub fn complete_command(prefix: &str) -> Vec<String> {
    let mut matches = Vec::new();
    let builtins = ["echo", "cd", "pwd", "type", "exit"];

    // 1. Check Builtins
    for &cmd in builtins.iter().filter(|c| c.starts_with(prefix)) {
        matches.push(cmd.to_string());
    }

    // 2. Check PATH executables
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    // Basic check: starts with prefix and is not a hidden file
                    if name.starts_with(prefix) {
                        matches.push(name);
                    }
                }
            }
        }
    }
    
    matches.sort();
    matches.dedup();
    matches
}

// ------------------------------------------------------------
// FILE / PATH COMPLETION
// ------------------------------------------------------------

pub fn complete_path(prefix: &str) -> Vec<String> {
    let mut matches = Vec::new();
    
    // Convert empty prefix to current directory
    let path_str = if prefix.is_empty() { "." } else { prefix };
    let path = Path::new(path_str);

    // Determine the directory to scan and the partial filename
    let (dir, partial_str) = if prefix.ends_with('/') {
        (path, "")
    } else {
        let p = path.parent().unwrap_or_else(|| Path::new("."));
        let f = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        (p, f)
    };

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            
            if name.starts_with(partial_str) {
                let mut new_path = dir.to_path_buf();
                new_path.push(&name);
                
                let mut path_string = new_path.to_string_lossy().to_string();
                
                // standard shell behavior: add trailing slash to directories
                if new_path.is_dir() {
                    path_string.push('/');
                }
                
                // If we were searching in the current directory (.), 
                // remove the "./" prefix for a cleaner UI
                if dir == Path::new(".") && !prefix.starts_with("./") {
                    path_string = name;
                    if new_path.is_dir() { path_string.push('/'); }
                }
                
                matches.push(path_string);
            }
        }
    }

    matches.sort();
    matches.dedup();
    matches
}
