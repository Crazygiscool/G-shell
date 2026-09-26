use crate::syscall;

pub fn mkdir(args: &[&str]) -> i32 {
    let mut parents = false;
    let mut dirs: Vec<String> = Vec::new();

    for a in args {
        match *a {
            "-p" => parents = true,
            "-v" => {}
            s if s.starts_with('-') && s.len() > 1 => {
                eprintln!("mkdir: invalid option -- '{}'", &s[1..2]);
                return 2;
            }
            s => dirs.push(s.to_string()),
        }
    }

    if dirs.is_empty() {
        eprintln!("mkdir: missing operand");
        return 1;
    }

    let mut code = 0;
    for dir in &dirs {
        if parents {
            // Make each missing path component.
            let mut acc = String::new();
            for part in dir.split('/') {
                if part.is_empty() {
                    if !acc.is_empty() {
                        acc.push('/');
                    }
                    continue;
                }
                if acc.is_empty() {
                    acc = part.to_string();
                } else {
                    acc.push('/');
                    acc.push_str(part);
                }
                if !syscall::exists(&acc) && syscall::mkdir(&acc, 0o777) < 0 {
                    eprintln!("mkdir: cannot create directory '{}'", acc);
                    code = 1;
                }
            }
        } else {
            if syscall::mkdir(dir, 0o777) < 0 {
                eprintln!("mkdir: cannot create directory '{}'", dir);
                code = 1;
            }
        }
    }
    code
}