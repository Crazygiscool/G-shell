pub fn export_var(args: &[&str]) {
    for arg in args {
        if let Some(eq_pos) = arg.find('=') {
            let name = &arg[..eq_pos];
            let value = &arg[eq_pos + 1..];
            unsafe { std::env::set_var(name, value); }
        } else {
            if let Ok(value) = std::env::var(arg) {
                println!("declare -x {}={}", arg, value);
            }
        }
    }
}

pub fn unset_var(args: &[&str]) {
    for arg in args {
        unsafe { std::env::remove_var(arg); }
    }
}

pub fn set_vars() {
    let mut vars: Vec<(String, String)> = std::env::vars().collect();
    vars.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, value) in &vars {
        println!("{}={}", name, value);
    }
}

pub fn env_vars() {
    let mut vars: Vec<(String, String)> = std::env::vars().collect();
    vars.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, value) in &vars {
        println!("{}={}", name, value);
    }
}
