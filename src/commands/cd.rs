pub fn cd(directory: &str) {
        if directory.is_empty() {
            if let Some(home) = std::env::var_os("HOME") {
                std::env::set_current_dir(home).unwrap_or_else(|_e| {
                    eprintln!("cd: {}: No such file or directory", directory);
                });
            }
            return;
        } else if directory == ".." {
            if let Some(parent) = std::env::current_dir().unwrap().parent() {
                std::env::set_current_dir(parent).unwrap_or_else(|_e| {
                    eprintln!("cd: {}: No such file or directory", directory);
                });
            }
            return;
        } else if directory == "." {
            return;
        } else if directory == "~" {
            if let Some(home) = std::env::var_os("HOME") {
                std::env::set_current_dir(home).unwrap_or_else(|_e| {
                    eprintln!("cd: {}: No such file or directory", directory);
                });
            }
            return;
        }
        match std::env::set_current_dir(directory) {
            Ok(_) => (),
            Err(_e) => eprintln!("cd: {}: No such file or directory", directory),
        }
    }