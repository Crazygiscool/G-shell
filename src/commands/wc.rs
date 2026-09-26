use std::os::unix::io::RawFd;

use crate::syscall;

pub fn wc(args: &[&str], stdin_fd: Option<RawFd>) -> i32 {
    let mut want_lines = false;
    let mut want_words = false;
    let mut want_bytes = false;
    let mut files: Vec<String> = Vec::new();

    for a in args {
        match *a {
            "-l" | "--lines" => want_lines = true,
            "-w" | "--words" => want_words = true,
            "-c" | "--bytes" => want_bytes = true,
            "-wl" | "-lw" => {
                want_lines = true;
                want_words = true;
            }
            "-lc" | "-cl" => {
                want_lines = true;
                want_bytes = true;
            }
            "-wc" | "-cw" => {
                want_words = true;
                want_bytes = true;
            }
            "-lwc" | "-lcw" | "-wlc" | "-wcl" | "-clw" | "-cwl" => {
                want_lines = true;
                want_words = true;
                want_bytes = true;
            }
            s if s.starts_with('-') && s.len() > 1 => {
                eprintln!("wc: invalid option -- '{}'", &s[1..2]);
                return 2;
            }
            s => files.push(s.to_string()),
        }
    }

    let show = |lines: usize, words: usize, bytes: usize, name: &str| {
        let mut parts = Vec::new();
        if want_lines {
            parts.push(format!("{:>8}", lines));
        }
        if want_words {
            parts.push(format!("{:>8}", words));
        }
        if want_bytes {
            parts.push(format!("{:>8}", bytes));
        }
        if parts.is_empty() {
            parts = vec![
                format!("{:>8}", lines),
                format!("{:>8}", words),
                format!("{:>8}", bytes),
            ];
        }
        println!("{} {}", parts.join(""), name);
    };

    let mut total_l = 0usize;
    let mut total_w = 0usize;
    let mut total_b = 0usize;

    if files.is_empty() {
        let text = syscall::read_all(stdin_fd.unwrap_or(0));
        let (l, w, b) = count(&text);
        show(l, w, b, "");
        return 0;
    }

    for f in &files {
        match std::fs::read(f) {
            Ok(data) => {
                let text = String::from_utf8_lossy(&data).to_string();
                let (l, w, b) = count(&text);
                total_l += l;
                total_w += w;
                total_b += b;
                show(l, w, b, f);
            }
            Err(_) => {
                eprintln!("wc: {}: No such file or directory", f);
            }
        }
    }
    if files.len() > 1 {
        show(total_l, total_w, total_b, "total");
    }
    0
}

fn count(text: &str) -> (usize, usize, usize) {
    let lines = if text.is_empty() { 0 } else { text.matches('\n').count() };
    let words = text.split_whitespace().count();
    let bytes = text.len();
    (lines, words, bytes)
}