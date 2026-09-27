fn valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn export_var(args: &[&str]) {
    for arg in args {
        if let Some(eq_pos) = arg.find('=') {
            let name = &arg[..eq_pos];
            let value = &arg[eq_pos + 1..];
            if !valid_identifier(name) {
                eprintln!("bash: export: `{}': not a valid identifier", name);
                continue;
            }
            unsafe { std::env::set_var(name, value); }
        } else {
            if !valid_identifier(arg) {
                eprintln!("bash: export: `{}': not a valid identifier", arg);
                continue;
            }
            if let Ok(value) = std::env::var(arg) {
                println!("declare -x {}={}", arg, value);
            }
        }
    }
}

pub fn unset_var(args: &[&str]) {
    for arg in args {
        if !valid_identifier(arg) {
            eprintln!("bash: unset: `{}': not a valid identifier", arg);
            continue;
        }
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
