use pathsearch::find_executable_in_path;
use std::process::Command;
use std::fs::File;
use std::os::unix::process::CommandExt;

pub fn execute(cmd: &str, args: &[&str], redirect: Option<(&str, i32)>) -> i32 {
        if let Some(path) = find_executable_in_path(cmd) {
            let mut child = Command::new(&path);

            child.arg0(cmd);
            child.args(args);

            if let Some((filename, fd)) = redirect {
                match fd {
                    1 => {
                        match File::create(filename) {
                            Ok(file) => { child.stdout(file); }
                            Err(e) => { eprintln!("{}: {}", filename, e); return 1; }
                        }
                    }
                    2 => {
                        match File::create(filename) {
                            Ok(file) => { child.stderr(file); }
                            Err(e) => { eprintln!("{}: {}", filename, e); return 1; }
                        }
                    }
                    0 => {
                        match File::open(filename) {
                            Ok(file) => { child.stdin(file); }
                            Err(e) => { eprintln!("{}: {}", filename, e); return 1; }
                        }
                    }
                    _ => {}
                }
            }

            match child.status() {
                Ok(status) => {
                    status.code().unwrap_or(1)
                }
                Err(_e) => {
                    1
                }
            }
        } else {
            eprintln!("{}: command not found", cmd);
            1
        }
    }
