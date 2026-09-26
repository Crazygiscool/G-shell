use crate::syscall;

pub fn ls(args: &[&str]) -> i32 {
    let mut long = false;
    let mut all = false;
    let mut paths: Vec<String> = Vec::new();

    for a in args {
        if let Some(s) = a.strip_prefix('-') {
            if s.is_empty() || s == "-" {
                paths.push((*a).to_string());
                continue;
            }
            let mut seen = false;
            for c in s.chars() {
                match c {
                    'l' => long = true,
                    'a' => all = true,
                    'h' | 'S' => {} // size hints: kernel files are tiny; ignored
                    _ => {
                        eprintln!("ls: invalid option -- '{}'", c);
                        return 2;
                    }
                }
                seen = true;
            }
            if !seen {
                eprintln!("ls: invalid option -- '{}'", &s[..1]);
                return 2;
            }
            continue;
        }
        paths.push((*a).to_string());
    }

    if paths.is_empty() {
        paths.push(".".to_string());
    }

    let mut code = 0;
    for (i, path) in paths.iter().enumerate() {
        if paths.len() > 1 {
            if i > 0 {
                println!();
            }
            println!("{}:", path);
        }

        match syscall::read_dir_names(path) {
            Some(mut entries) => {
                if !all {
                    entries.retain(|(n, _)| !n.starts_with('.'));
                }
                if long {
                    for (name, dtype) in &entries {
                        let is_dir = *dtype == 4 || syscall::is_dir(&syscall::join_dir(path, name));
                        let size = syscall::file_size(&syscall::join_dir(path, name)).unwrap_or(0);
                        let mode = if is_dir { "drwx------" } else { "-rw-r--r--" };
                        println!("{:>10} {:>6} {}", mode, size, name);
                    }
                } else if entries.is_empty() {
                    // nothing
                } else {
                    let line = entries
                        .iter()
                        .map(|(n, _)| n.clone())
                        .collect::<Vec<_>>()
                        .join("  ");
                    println!("{}", line);
                }
            }
            None => {
                eprintln!("ls: cannot access '{}': No such file or directory", path);
                code = 2;
            }
        }
    }
    code
}