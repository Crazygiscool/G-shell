use std::fs;
use std::path::Path;

pub fn expand_globs(tokens: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    for token in tokens {
        if contains_glob_chars(token) {
            let expanded = expand_glob(token);
            if expanded.is_empty() {
                result.push(token.clone());
            } else {
                result.extend(expanded);
            }
        } else {
            result.push(token.clone());
        }
    }
    result
}

fn contains_glob_chars(s: &str) -> bool {
    s.contains('*') || s.contains('?') || s.contains('[')
}

fn expand_glob(pattern: &str) -> Vec<String> {
    let path = Path::new(pattern);
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_pattern = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    let dir_entries = match fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let matched: Vec<String> = dir_entries
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|name| glob_match(file_pattern, name))
        .map(|name| {
            let full = parent.join(&name);
            if full.is_dir() {
                full.to_string_lossy().to_string() + "/"
            } else {
                full.to_string_lossy().to_string()
            }
        })
        .collect();

    if matched.is_empty() {
        Vec::new()
    } else {
        matched
    }
}

fn glob_match(pattern: &str, text: &str) -> bool {
    let pchars: Vec<char> = pattern.chars().collect();
    let tchars: Vec<char> = text.chars().collect();
    glob_match_recursive(&pchars, &tchars, 0, 0)
}

fn glob_match_recursive(p: &[char], t: &[char], pi: usize, ti: usize) -> bool {
    let plen = p.len();
    let tlen = t.len();

    if pi == plen && ti == tlen {
        return true;
    }

    if pi >= plen {
        return false;
    }

    let pc = p[pi];

    if pc == '*' {
        // Try matching 0, 1, 2... characters from text
        for ti2 in ti..=tlen {
            if glob_match_recursive(p, t, pi + 1, ti2) {
                return true;
            }
        }
        false
    } else if ti >= tlen {
        false
    } else if pc == '?' {
        glob_match_recursive(p, t, pi + 1, ti + 1)
    } else if pc == '[' {
        let (matches, new_pi) = parse_bracket(&p[pi..]);
        if matches.contains(&t[ti]) {
            glob_match_recursive(p, t, pi + new_pi, ti + 1)
        } else {
            false
        }
    } else if pc == '\\' && pi + 1 < plen {
        if p[pi + 1] == t[ti] {
            glob_match_recursive(p, t, pi + 2, ti + 1)
        } else {
            false
        }
    } else if pc == t[ti] {
        glob_match_recursive(p, t, pi + 1, ti + 1)
    } else {
        false
    }
}

fn parse_bracket(p: &[char]) -> (Vec<char>, usize) {
    let mut chars = Vec::new();
    let mut i = 1; // skip '['
    if i < p.len() && p[i] == '!' {
        i += 1;
    }
    while i < p.len() && p[i] != ']' {
        if i + 2 < p.len() && p[i + 1] == '-' && p[i + 2] != ']' {
            // Simple range expansion:
            let start_c = p[i];
            let end_c = p[i + 2];
            let start_u = start_c as u32;
            let end_u = end_c as u32;
            for c in start_u..=end_u {
                if let Some(ch) = char::from_u32(c) {
                    chars.push(ch);
                }
            }
            i += 3;
        } else {
            chars.push(p[i]);
            i += 1;
        }
    }
    if i < p.len() && p[i] == ']' {
        i += 1; // skip ']'
    }
    (chars, i)
}
