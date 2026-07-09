use std::env;

pub fn expand_tokens(tokens: &[String], last_exit_code: i32) -> Vec<String> {
    tokens.iter().map(|token| expand_token(token, last_exit_code)).collect()
}

fn expand_token(token: &str, last_exit_code: i32) -> String {
    let s = expand_tilde(token);
    expand_vars(&s, last_exit_code)
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

fn expand_vars(s: &str, last_exit_code: i32) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' {
            match chars.peek() {
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
        } else {
            result.push(c);
        }
    }

    result
}

fn expand_var(name: &str) -> String {
    env::var(name).unwrap_or_default()
}
