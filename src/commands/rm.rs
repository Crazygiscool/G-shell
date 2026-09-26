use crate::syscall;

pub fn rm(args: &[&str]) -> i32 {
    let mut recursive = false;
    let mut force = false;
    let mut targets: Vec<String> = Vec::new();

    for a in args {
        match *a {
            "-r" | "-R" | "--recursive" => recursive = true,
            "-f" | "--force" => force = true,
            "-rf" | "-fr" | "-Rf" | "-fR" | "-rF" | "-Fr" => {
                recursive = true;
                force = true;
            }
            s if s.starts_with('-') && s.len() > 1 && s != "--" => {
                eprintln!("rm: invalid option -- '{}'", &s[1..2]);
                return 2;
            }
            s => targets.push(s.to_string()),
        }
    }

    if targets.is_empty() {
        eprintln!("rm: missing operand");
        return 1;
    }

    let mut code = 0;
    for t in &targets {
        if !remove_path(t, recursive, &mut 0) && !force {
            eprintln!("rm: cannot remove '{}'", t);
            code = 1;
        }
    }
    code
}

/// Remove a single path; recurses when recursive is set. Returns true on success.
fn remove_path(path: &str, recursive: bool, _depth: &mut usize) -> bool {
    if syscall::is_dir(path) {
        if !recursive {
            eprintln!("rm: cannot remove '{}': Is a directory", path);
            return false;
        }
        if let Some(entries) = syscall::read_dir_names(path) {
            for (name, _) in entries {
                let child = syscall::join_dir(path, &name);
                if !remove_path(&child, recursive, _depth) {
                    return false;
                }
            }
        }
        return syscall::rmdir(path) >= 0;
    }
    syscall::unlink(path) >= 0
}