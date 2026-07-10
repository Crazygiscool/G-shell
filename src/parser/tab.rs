use std::path::Path;
use crate::parser::pathcache;

pub fn complete_command(prefix: &str) -> Vec<String> {
    let mut matches: Vec<String> = Vec::new();
    let builtins = ["echo", "cd", "pwd", "type", "exit", "history", "export", "unset", "set", "env", "source", "test", "alias", "unalias", "help"];

    for &cmd in builtins.iter().filter(|c| c.starts_with(prefix)) {
        matches.push(cmd.to_string());
    }

    for cmd in pathcache::get_cached_commands() {
        if cmd.starts_with(prefix) && !matches.contains(&cmd) {
            matches.push(cmd);
        }
    }

    matches.sort();
    matches.dedup();
    matches
}

pub fn complete_variable(prefix: &str) -> Vec<String> {
    let var_name = if prefix.starts_with('$') { &prefix[1..] } else { prefix };
    let mut matches = Vec::new();

    for (key, _value) in std::env::vars() {
        if key.starts_with(var_name) {
            matches.push(format!("${{{}}}", key));
        }
    }

    matches.sort();
    matches
}

pub fn complete_path(prefix: &str) -> Vec<String> {
    let mut matches = Vec::new();

    let path_str = if prefix.is_empty() { "." } else { prefix };
    let path = Path::new(path_str);

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

                if new_path.is_dir() {
                    path_string.push('/');
                }

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
