use std::path::Path;
use crate::parser::pathcache;
use crate::parser::alias;

#[derive(Clone, Debug)]
pub struct CompletionCandidate {
    pub display: String,
    pub replacement: String,
    #[allow(dead_code)]
    pub description: String,
}

fn builtin_descriptions() -> Vec<(&'static str, &'static str)> {
    vec![
        ("alias", "define or display aliases"),
        ("cd", "change the working directory"),
        ("echo", "display a line of text"),
        ("env", "display or set environment variables"),
        ("exit", "exit the shell"),
        ("export", "set export attribute for variables"),
        ("help", "display help information"),
        ("history", "display or manipulate command history"),
        ("pwd", "print name of current working directory"),
        ("set", "set positional parameters or shell attributes"),
        ("source", "read and execute commands from a file"),
        ("test", "evaluate conditional expression"),
        ("type", "describe a command"),
        ("unalias", "remove alias definitions"),
        ("unset", "unset variables or functions"),
    ]
}

pub fn complete_command(prefix: &str) -> Vec<CompletionCandidate> {
    let mut matches: Vec<CompletionCandidate> = Vec::new();
    let prefix_lower = prefix.to_lowercase();

    for (cmd, desc) in builtin_descriptions() {
        if cmd.to_lowercase().starts_with(&prefix_lower) {
            matches.push(CompletionCandidate {
                display: format!("{}  -- {}", cmd, desc),
                replacement: format!("{} ", cmd),
                description: desc.to_string(),
            });
        }
    }

    for cmd in pathcache::get_cached_commands() {
        if cmd.to_lowercase().starts_with(&prefix_lower)
            && !matches.iter().any(|m| m.replacement.trim() == cmd)
        {
            matches.push(CompletionCandidate {
                display: cmd.clone(),
                replacement: format!("{} ", cmd),
                description: String::new(),
            });
        }
    }

    for alias_name in alias::get_alias_names() {
        if alias_name.to_lowercase().starts_with(&prefix_lower)
            && !matches.iter().any(|m| m.replacement.trim() == alias_name)
        {
            matches.push(CompletionCandidate {
                display: format!("{}  (alias)", alias_name),
                replacement: format!("{} ", alias_name),
                description: "alias".to_string(),
            });
        }
    }

    matches.sort_by(|a, b| a.replacement.cmp(&b.replacement));
    matches
}

pub fn complete_variable(prefix: &str) -> Vec<CompletionCandidate> {
    let var_name = if prefix.starts_with('$') { &prefix[1..] } else { prefix };
    let prefix_lower = var_name.to_lowercase();
    let mut matches = Vec::new();

    for (key, _value) in std::env::vars() {
        if key.to_lowercase().starts_with(&prefix_lower) {
            matches.push(CompletionCandidate {
                display: format!("${{{}}}  = {}", key, &_value[.._value.len().min(40)]),
                replacement: format!("${{{}}}", key),
                description: _value,
            });
        }
    }

    matches.sort_by(|a, b| a.replacement.cmp(&b.replacement));
    matches
}

pub fn complete_path(prefix: &str) -> Vec<CompletionCandidate> {
    let mut matches = Vec::new();

    let expanded_prefix = if prefix.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            format!("{}{}", home, &prefix[1..])
        } else {
            prefix.to_string()
        }
    } else if prefix == "~" {
        if let Ok(home) = std::env::var("HOME") {
            home
        } else {
            prefix.to_string()
        }
    } else {
        prefix.to_string()
    };

    let path_str = if expanded_prefix.is_empty() { "." } else { &expanded_prefix };
    let path = Path::new(path_str);

    let (dir, partial_str) = if expanded_prefix.ends_with('/') || path.is_dir() {
        (path, "")
    } else {
        let p = path.parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let f = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        (p, f)
    };

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();

            let partial_lower = partial_str.to_lowercase();
            let name_lower = name.to_lowercase();

            if name_lower.starts_with(&partial_lower) {
                let mut new_path = dir.to_path_buf();
                new_path.push(&name);

                let mut path_string = new_path.to_string_lossy().to_string();

                if new_path.is_dir() {
                    path_string.push('/');
                }

                if dir == Path::new(".") && !expanded_prefix.starts_with("./") {
                    path_string = name.clone();
                    if new_path.is_dir() { path_string.push('/'); }
                }

                let display = if new_path.is_dir() {
                    format!("{}/", path_string.trim_end_matches('/'))
                } else {
                    path_string.clone()
                };

                matches.push(CompletionCandidate {
                    display,
                    replacement: path_string,
                    description: if new_path.is_dir() { "directory".to_string() } else { "file".to_string() },
                });
            }
        }
    }

    matches.sort_by(|a, b| {
        let a_dir = a.replacement.ends_with('/');
        let b_dir = b.replacement.ends_with('/');
        b_dir.cmp(&a_dir).then(a.replacement.cmp(&b.replacement))
    });
    matches.dedup_by(|a, b| a.replacement == b.replacement);
    matches
}
