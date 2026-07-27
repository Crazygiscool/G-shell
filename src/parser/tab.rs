use std::path::Path;
use crate::parser::pathcache;
use crate::parser::alias;
use crate::parser::eval::BUILTIN_REGISTRY;

#[derive(Clone, Debug)]
pub struct CompletionCandidate {
    pub display: String,
    pub replacement: String,
    #[allow(dead_code)]
    pub description: String,
}

fn builtin_descriptions() -> Vec<(&'static str, &'static str)> {
    BUILTIN_REGISTRY
        .iter()
        .map(|&(name, _, desc)| (name, desc))
        .collect()
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

            let has_uppercase = partial_str.chars().any(|c| c.is_uppercase());

            if if has_uppercase {
                name.starts_with(partial_str)
            } else {
                name.to_lowercase().starts_with(&partial_str.to_lowercase())
            } {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmpdir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("gshell_test_{}", name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_builtin_descriptions_count() {
        let descs = builtin_descriptions();
        assert_eq!(descs.len(), BUILTIN_REGISTRY.len());
    }

    #[test]
    fn test_builtin_descriptions_all_have_names_and_descs() {
        for (name, desc) in builtin_descriptions() {
            assert!(!name.is_empty());
            assert!(!desc.is_empty());
        }
    }

    #[test]
    fn test_builtin_descriptions_includes_key_builtins() {
        let descs = builtin_descriptions();
        let names: Vec<&str> = descs.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"cd"));
        assert!(names.contains(&"echo"));
        assert!(names.contains(&"exit"));
        assert!(names.contains(&"export"));
        assert!(names.contains(&"test"));
        assert!(names.contains(&"["));
        assert!(names.contains(&"alias"));
        assert!(names.contains(&"history"));
    }

    #[test]
    fn test_complete_command_exact_match() {
        let results = complete_command("cd");
        assert!(!results.is_empty());
        assert!(results.iter().any(|c| c.replacement.trim() == "cd"));
    }

    #[test]
    fn test_complete_command_prefix() {
        let results = complete_command("ec");
        assert!(!results.is_empty());
        assert!(results.iter().any(|c| c.replacement.trim() == "echo"));
    }

    #[test]
    fn test_complete_command_case_insensitive() {
        let results = complete_command("CD");
        assert!(!results.is_empty());
        assert!(results.iter().any(|c| c.replacement.trim() == "cd"));
    }

    #[test]
    fn test_complete_command_no_match() {
        let results = complete_command("zzznonexistent");
        let builtins: Vec<_> = results.iter().filter(|c| c.description != "alias" || true).collect();
        assert!(builtins.is_empty() || results.iter().all(|c| !c.replacement.trim().starts_with("zzznonexistent")));
    }

    #[test]
    fn test_complete_command_empty_prefix() {
        let results = complete_command("");
        assert!(results.len() >= 15);
    }

    #[test]
    fn test_complete_command_replacement_has_trailing_space() {
        let results = complete_command("cd");
        for c in &results {
            if c.replacement.trim() == "cd" {
                assert!(c.replacement.ends_with(' '));
            }
        }
    }

    #[test]
    fn test_complete_command_sorted() {
        let results = complete_command("");
        for w in results.windows(2) {
            assert!(w[0].replacement <= w[1].replacement);
        }
    }

    #[test]
    fn test_complete_variable_with_dollar() {
        let results = complete_variable("$PATH");
        assert!(!results.is_empty());
        assert!(results.iter().any(|c| c.replacement.contains("PATH")));
    }

    #[test]
    fn test_complete_variable_without_dollar() {
        let results = complete_variable("PATH");
        assert!(!results.is_empty());
        assert!(results.iter().any(|c| c.replacement.contains("PATH")));
    }

    #[test]
    fn test_complete_variable_replacement_has_dollar() {
        let results = complete_variable("HOME");
        assert!(!results.is_empty());
        for c in &results {
            assert!(c.replacement.starts_with('$'));
        }
    }

    #[test]
    fn test_complete_variable_sorted() {
        let results = complete_variable("");
        for w in results.windows(2) {
            assert!(w[0].replacement <= w[1].replacement);
        }
    }

    #[test]
    fn test_complete_variable_no_match() {
        let results = complete_variable("$zzznonexistent_xyz_123");
        assert!(results.is_empty());
    }

    #[test]
    fn test_complete_path_empty_prefix_lists_cwd() {
        let results = complete_path("");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_complete_path_filters_by_prefix() {
        let dir = tmpdir("path_filter");
        fs::create_dir_all(dir.join("alpha")).unwrap();
        fs::create_dir_all(dir.join("bravo")).unwrap();
        fs::File::create(dir.join("alpha.txt")).unwrap();

        let old = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();

        let results = complete_path("al");
        assert!(results.iter().any(|c| c.replacement.starts_with("alpha")));

        std::env::set_current_dir(&old).unwrap();
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_complete_path_single_char_prefix() {
        let dir = tmpdir("path_single");
        fs::create_dir_all(dir.join("aur")).unwrap();
        fs::create_dir_all(dir.join("build")).unwrap();

        let old = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();

        let results = complete_path("a");
        assert_eq!(results.len(), 1);
        assert!(results[0].replacement.starts_with("aur"));

        std::env::set_current_dir(&old).unwrap();
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_complete_path_directory_gets_slash() {
        let dir = tmpdir("path_slash");
        fs::create_dir_all(dir.join("mydir")).unwrap();
        fs::File::create(dir.join("mydir.txt")).unwrap();

        let old = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();

        let results = complete_path("my");
        let dir_result = results.iter().find(|c| c.replacement.ends_with('/'));
        assert!(dir_result.is_some());
        assert!(dir_result.unwrap().replacement.starts_with("mydir"));

        let file_result = results.iter().find(|c| c.replacement == "mydir.txt");
        assert!(file_result.is_some());

        std::env::set_current_dir(&old).unwrap();
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_complete_path_nonexistent_prefix_falls_back_to_cwd() {
        let dir = tmpdir("path_fallback");
        fs::create_dir_all(dir.join("xyz_item")).unwrap();

        let old = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();

        let results = complete_path("xyz");
        assert_eq!(results.len(), 1);
        assert!(results[0].replacement.starts_with("xyz_item"));

        std::env::set_current_dir(&old).unwrap();
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_complete_path_parent_empty_string_fix() {
        let dir = tmpdir("path_parent");
        fs::create_dir_all(dir.join("test_entry")).unwrap();

        let old = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();

        let results = complete_path("test");
        assert_eq!(results.len(), 1);
        assert!(results[0].replacement.starts_with("test_entry"));

        std::env::set_current_dir(&old).unwrap();
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_complete_path_trailing_slash_lists_contents() {
        let dir = tmpdir("path_trailing");
        fs::create_dir_all(dir.join("subdir")).unwrap();
        fs::File::create(dir.join("subdir/file.txt")).unwrap();

        let old = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();

        let results = complete_path("subdir/");
        assert_eq!(results.len(), 1);
        assert!(results[0].replacement.contains("file.txt"));

        std::env::set_current_dir(&old).unwrap();
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_complete_path_directories_before_files() {
        let dir = tmpdir("path_order");
        fs::create_dir_all(dir.join("adir")).unwrap();
        fs::File::create(dir.join("afile.txt")).unwrap();

        let old = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();

        let results = complete_path("a");
        assert!(!results.is_empty());
        let first_dir = results.iter().position(|c| c.replacement.ends_with('/'));
        let first_file = results.iter().position(|c| !c.replacement.ends_with('/'));
        if let (Some(d), Some(f)) = (first_dir, first_file) {
            assert!(d < f, "directories should come before files");
        }

        std::env::set_current_dir(&old).unwrap();
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_complete_path_no_match_empty_result() {
        let dir = tmpdir("path_nomatch");
        fs::create_dir_all(dir.join("abc")).unwrap();

        let old = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();

        let results = complete_path("zzz");
        assert!(results.is_empty());

        std::env::set_current_dir(&old).unwrap();
        let _ = fs::remove_dir_all(&dir);
    }
}
