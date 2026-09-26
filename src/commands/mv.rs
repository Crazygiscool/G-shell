use crate::commands::cp;
use crate::syscall;

pub fn mv(args: &[&str]) -> i32 {
    let mut paths: Vec<String> = Vec::new();

    for a in args {
        match *a {
            "-f" | "-i" | "-v" => {}
            s if s.starts_with('-') && s.len() > 1 => {
                eprintln!("mv: invalid option -- '{}'", &s[1..2]);
                return 2;
            }
            s => paths.push(s.to_string()),
        }
    }

    if paths.len() < 2 {
        eprintln!("mv: missing file operand");
        return 1;
    }

    let dest = paths.pop().unwrap();
    let sources = paths;
    let dest_is_dir = syscall::is_dir(&dest);

    let mut code = 0;
    for src in &sources {
        let target = if dest_is_dir {
            let base = src.rsplit('/').next().unwrap_or(src);
            syscall::join_dir(&dest, base)
        } else {
            dest.clone()
        };

        if syscall::is_dir(src) {
            // No kernel rename(2): move a directory by copy + remove.
            let ok = move_tree(src, &target);
            if !ok {
                eprintln!("mv: cannot move '{}'", src);
                code = 1;
            }
        } else if cp::move_file(src, &target) {
            // done
        } else {
            eprintln!("mv: cannot move '{}' to '{}'", src, target);
            code = 1;
        }
    }
    code
}

fn move_tree(src: &str, dest: &str) -> bool {
    let _ = syscall::mkdir(dest, 0o777);
    if let Some(entries) = syscall::read_dir_names(src) {
        for (name, _) in entries {
            let s = syscall::join_dir(src, &name);
            let d = syscall::join_dir(dest, &name);
            if syscall::is_dir(&s) {
                if !move_tree(&s, &d) {
                    return false;
                }
            } else if !cp::move_file(&s, &d) {
                return false;
            }
        }
    }
    syscall::rmdir(src) >= 0
}