use std::env;
use std::os::unix::io::AsRawFd;
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
                    result.push_str(&execute_subshell(&cmd_str, last_exit_code));
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
            result.push_str(&execute_subshell(&cmd_str, last_exit_code));
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

/// Run a command substitution using the shell's own evaluator in a forked
/// child, capturing stdout. `sh -c` remains only as a host-side fallback if
/// fork() is unavailable.
fn execute_subshell(cmd: &str, last_exit_code: i32) -> String {
    let tokens = crate::parser::tokenize::tokenize(cmd);
    if tokens.is_empty() {
        return String::new();
    }
    let program = crate::parser::parser::parse(&tokens);
    if program.commands.is_empty() {
        return String::new();
    }

    let (reader, writer) = match os_pipe::pipe() {
        Ok(p) => p,
        Err(_) => return String::new(),
    };
    let pid = unsafe { libc::fork() };
    if pid == 0 {
        // Child: run the commands with stdout connected to the pipe.
        unsafe {
            libc::dup2(writer.as_raw_fd(), 1);
        }
        drop(writer);
        drop(reader);
        let code = crate::parser::eval::eval_program(&program, &[], last_exit_code);
        std::process::exit(code.min(255).max(0));
    } else if pid > 0 {
        drop(writer);
        let mut out = String::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = unsafe {
                libc::read(reader.as_raw_fd(), buf.as_mut_ptr() as *mut libc::c_void, buf.len())
            };
            if n <= 0 {
                break;
            }
            out.push_str(&String::from_utf8_lossy(&buf[..n as usize]));
        }
        drop(reader);
        let mut status: libc::c_int = 0;
        unsafe { libc::waitpid(pid, &mut status, 0); }
        while out.ends_with('\n') {
            out.pop();
        }
        out
    } else {
        // fork() failed: fall back to the host's sh if present.
        if let Ok(output) = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .output()
        {
            let mut s = String::from_utf8_lossy(&output.stdout).to_string();
            while s.ends_with('\n') {
                s.pop();
            }
            s
        } else {
            String::new()
        }
    }
}

fn expand_var(name: &str) -> String {
    env::var(name).unwrap_or_default()
}

pub fn expand_prompt(template: &str, last_exit_code: i32) -> String {
    // Check for oh-my-posh theme first
    if let Ok(path) = env::var("GS_OH_MY_POSH_THEME") {
        if let Some(format_str) = crate::parser::theme::load_omp_theme(&path) {
            return crate::parser::theme::render_prompt(&format_str, last_exit_code);
        }
    }
    // Check for GS_PROMPT_FORMAT
    if let Ok(format) = env::var("GS_PROMPT_FORMAT") {
        return crate::parser::theme::render_prompt(&format, last_exit_code);
    }
    // Fall back to legacy PS1 with backslash escapes + segment placeholders
    crate::parser::theme::render_prompt(template, last_exit_code)
}
