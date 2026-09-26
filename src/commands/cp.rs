use crate::syscall;

pub fn cp(args: &[&str]) -> i32 {
    let mut recursive = false;
    let mut quiet = false;
    let mut paths: Vec<String> = Vec::new();

    for a in args {
        match *a {
            "-r" | "-R" | "-a" => recursive = true,
            "-f" | "-i" => {}
            "-q" => quiet = true,
            s if s.starts_with('-') && s.len() > 1 => {
                eprintln!("cp: invalid option -- '{}'", &s[1..2]);
                return 2;
            }
            s => paths.push(s.to_string()),
        }
    }

    if paths.len() < 2 {
        eprintln!("cp: missing file operand");
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
        if recursive && syscall::is_dir(src) {
            copy_tree(src, &target);
        } else if !copy_file(src, &target) {
            if !quiet {
                eprintln!("cp: cannot copy '{}' to '{}'", src, target);
            }
            code = 1;
        }
    }
    code
}

fn copy_file(src: &str, dest: &str) -> bool {
    let mut fin = match std::fs::File::open(src) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut fout = match std::fs::File::create(dest) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut buf = [0u8; 8192];
    loop {
        match std::io::Read::read(&mut fin, &mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                if std::io::Write::write_all(&mut fout, &buf[..n]).is_err() {
                    return false;
                }
            }
        }
    }
    true
}

fn copy_tree(src: &str, dest: &str) {
    match syscall::read_dir_names(src) {
        Some(entries) => {
            let _ = syscall::mkdir(dest, 0o777);
            for (name, _) in entries {
                let s = syscall::join_dir(src, &name);
                let d = syscall::join_dir(dest, &name);
                if syscall::is_dir(&s) {
                    copy_tree(&s, &d);
                } else {
                    let _ = copy_file(&s, &d);
                }
            }
        }
        None => {
            let _ = copy_file(src, dest);
        }
    }
}

pub fn move_file(src: &str, dest: &str) -> bool {
    if copy_file(src, dest) && syscall::unlink(src) >= 0 {
        true
    } else {
        false
    }
}