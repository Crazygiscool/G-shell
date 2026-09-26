use std::os::unix::io::RawFd;

use crate::parser::glob::glob_match;
use crate::syscall;

pub fn grep(args: &[&str], stdin_fd: Option<RawFd>) -> i32 {
    let mut ignore_case = false;
    let mut invert = false;
    let mut numbering = false;
    let mut use_glob = false;
    let mut pattern: Option<String> = None;
    let mut files: Vec<String> = Vec::new();

    let mut it = args.iter();
    while let Some(a) = it.next() {
        match *a {
            "-i" | "--ignore-case" => ignore_case = true,
            "-v" | "--invert-match" => invert = true,
            "-n" | "--line-number" => numbering = true,
            "-e" => {
                if let Some(p) = it.next() {
                    pattern = Some(p.to_string());
                }
            }
            "-g" | "--glob" => use_glob = true,
            "--" => {
                for rest in it {
                    files.push(rest.to_string());
                }
                break;
            }
            s if s.starts_with('-') && s.len() > 1 => {
                // Combined short flags like -in
                for c in s[1..].chars() {
                    match c {
                        'i' => ignore_case = true,
                        'v' => invert = true,
                        'n' => numbering = true,
                        'g' => use_glob = true,
                        _ => {
                            eprintln!("grep: invalid option -- '{}'", c);
                            return 2;
                        }
                    }
                }
            }
            s => files.push(s.to_string()),
        }
    }

    let pat = match pattern {
        Some(p) => p,
        None => {
            if files.is_empty() {
                eprintln!("grep: usage: grep [OPTIONS] PATTERN [FILE...]");
                return 2;
            }
            // The first positional is the pattern; the rest are files.
            files.remove(0)
        }
    };

    let mut any_match = false;
    let mut code = 0;

    if files.is_empty() {
        let text = syscall::read_all(stdin_fd.unwrap_or(0));
        if scan(&text, &pat, ignore_case, invert, numbering, use_glob) {
            any_match = true;
        }
    }

    for f in &files {
        match std::fs::read_to_string(f) {
            Ok(text) => {
                if scan(&text, &pat, ignore_case, invert, numbering, use_glob) {
                    any_match = true;
                }
            }
            Err(_) => {
                eprintln!("grep: {}: No such file or directory", f);
                code = 2;
            }
        }
    }

    if code != 0 {
        code
    } else if any_match {
        0
    } else {
        1
    }
}

fn scan(text: &str, pat: &str, ic: bool, invert: bool, numbering: bool, use_glob: bool) -> bool {
    let pat_cmp = if ic { pat.to_lowercase() } else { pat.to_string() };
    let mut found = false;
    for (i, line) in text.lines().enumerate() {
        if text.is_empty() && i == 0 && line.is_empty() {
            continue;
        }
        let hay = if ic { line.to_lowercase() } else { line.to_string() };
        let matched = if use_glob {
            glob_match(&pat_cmp, &hay)
        } else {
            hay.contains(&pat_cmp)
        };
        let show = if invert { !matched } else { matched };
        if show {
            found = true;
            if numbering {
                println!("{}:{}", i + 1, line);
            } else {
                println!("{}", line);
            }
        }
    }
    found
}