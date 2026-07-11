use std::env;
use std::process::{Command, Stdio};


pub fn expand_tokens(tokens: &[String], last_exit_code: i32) -> Vec<String> {
    tokens.iter().map(|token| expand_token(token, last_exit_code)).collect()
}

fn expand_token(token: &str, last_exit_code: i32) -> String {
    let s = expand_tilde(token);
    expand_vars_and_cmd(&s, last_exit_code)
}

fn expand_tilde(token: &str) -> String {
    if !token.starts_with('~') {
        return token.to_string();
    }
    if token == "~" || token.starts_with("~/") {
        if let Ok(home) = env::var("HOME") {
            return home + &token[1..];
        }
    }
    token.to_string()
}

fn expand_vars_and_cmd(s: &str, last_exit_code: i32) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' {
            match chars.peek() {
                Some('(') => {
                    chars.next();
                    let cmd_str = capture_parens(&mut chars, ')');
                    result.push_str(&execute_subshell(&cmd_str));
                }
                Some('?') => {
                    chars.next();
                    result.push_str(&last_exit_code.to_string());
                }
                Some('{') => {
                    chars.next();
                    let mut var_name = String::new();
                    while let Some(&ch) = chars.peek() {
                        if ch == '}' {
                            chars.next();
                            break;
                        }
                        var_name.push(ch);
                        chars.next();
                    }
                    result.push_str(&expand_var(&var_name));
                }
                Some(ch) if ch.is_ascii_alphanumeric() || *ch == '_' => {
                    let mut var_name = String::new();
                    while let Some(&ch) = chars.peek() {
                        if ch.is_ascii_alphanumeric() || ch == '_' {
                            var_name.push(ch);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    result.push_str(&expand_var(&var_name));
                }
                _ => {
                    result.push('$');
                }
            }
        } else if c == '`' {
            let cmd_str = capture_backtick(&mut chars);
            result.push_str(&execute_subshell(&cmd_str));
        } else {
            result.push(c);
        }
    }

    result
}

fn capture_parens(chars: &mut std::iter::Peekable<std::str::Chars>, close: char) -> String {
    let mut depth = 1;
    let mut inner = String::new();
    while let Some(&c) = chars.peek() {
        if c == '(' && close == ')' {
            depth += 1;
            chars.next();
            inner.push(c);
        } else if c == close {
            depth -= 1;
            chars.next();
            if depth == 0 {
                break;
            }
            inner.push(c);
        } else {
            inner.push(c);
            chars.next();
        }
    }
    inner
}

fn capture_backtick(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut inner = String::new();
    while let Some(&c) = chars.peek() {
        if c == '`' {
            chars.next();
            break;
        }
        inner.push(c);
        chars.next();
    }
    inner
}

fn execute_subshell(cmd: &str) -> String {
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output();

    match output {
        Ok(out) => {
            let mut s = String::from_utf8_lossy(&out.stdout).to_string();
            if s.ends_with('\n') {
                s.pop();
            }
            s
        }
        Err(_) => String::new(),
    }
}

fn expand_var(name: &str) -> String {
    env::var(name).unwrap_or_default()
}

pub fn expand_prompt(template: &str) -> String {
    let pwd = env::current_dir().ok();
    let cwd = pwd.as_deref().and_then(|p| p.to_str()).unwrap_or("?");
    let cwd_basename = pwd.as_ref().and_then(|p| p.file_name().and_then(|n| n.to_str())).unwrap_or("?");
    let home = env::var("HOME").ok();
    let w = match home {
        Some(ref home) if cwd.starts_with(home) => "~".to_string() + &cwd[home.len()..],
        _ => cwd.to_string(),
    };
    let user = env::var("USER").unwrap_or_else(|_| "?".to_string());
    let host = env::var("HOSTNAME").ok().or_else(|| {
        std::process::Command::new("hostname").arg("-s").output().ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok().map(|x| x.trim().to_string()))
    }).unwrap_or_else(|| "?".to_string());
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let sec = now.as_secs() % 86400;
    let h = sec / 3600;
    let m = sec / 60 % 60;
    let s = sec % 60;
    let t = format!("{:02}:{:02}:{:02}", h, m, s);

    let mut result = String::new();
    let mut chars = template.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('w') => result.push_str(&w),
                Some('W') => result.push_str(cwd_basename),
                Some('u') => result.push_str(&user),
                Some('h') => result.push_str(&host),
                Some('$') => result.push(if user == "root" { '#' } else { '$' }),
                Some('t') => result.push_str(&t),
                Some('e') => result.push('\x1b'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                },
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }
    result
}
