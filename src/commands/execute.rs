use pathsearch::find_executable_in_path;
use std::process::Command;
use std::fs::File;
use std::os::unix::process::CommandExt;

pub fn execute(cmd: &str, args: &[&str], redirect: Option<(&str, i32)>) {
        if let Some(path) = find_executable_in_path(cmd) {
            let mut child = Command::new(&path);

            child.arg0(cmd);
            child.args(args);

            if let Some((filename, fd)) = redirect {
                if fd == 1 {
                    match File::create(filename) {
                        Ok(file) => {
                            child.stdout(file);
                        }
                        Err(e) => {
                            eprintln!("{}: {}", filename, e);
                            return;
                        }
                    }
                }
            }

            match child.status() {
                Ok(status) => {
                    if !status.success() {
                        return;
                    }
                }
                Err(_e) => {
                    return;
                }
            }
        } else {
            eprintln!("{}: command not found", cmd);
        }
    }