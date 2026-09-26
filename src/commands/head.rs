use std::io::{BufRead, Write};
use std::os::unix::io::RawFd;

pub fn head(args: &[&str], stdin_fd: Option<RawFd>) -> i32 {
    let mut count: Option<usize> = None;
    let mut numbers = false;
    let mut files: Vec<String> = Vec::new();

    let mut it = args.iter();
    while let Some(a) = it.next() {
        match *a {
            "-n" => {
                if let Some(v) = it.next() {
                    count = v.parse().ok();
                }
            }
            s if s.starts_with("-n")
                && s.len() > 2
                && s[2..].chars().all(|c| c.is_ascii_digit()) =>
            {
                count = s[2..].parse().ok();
            }
            "--lines" => {
                if let Some(v) = it.next() {
                    count = v.parse().ok();
                }
            }
            "-v" => numbers = true,
            s if s.starts_with('-') && s.len() > 1 && s[1..].chars().all(|c| c.is_ascii_digit()) => {
                count = s[1..].parse().ok();
            }
            s if s.starts_with('-') && s.len() > 1 => {
                eprintln!("head: invalid option -- '{}'", &s[1..2]);
                return 2;
            }
            s => files.push(s.to_string()),
        }
    }

    let n = count.unwrap_or(10);
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    if files.is_empty() {
        return head_fd(stdin_fd.unwrap_or(0), n);
    }

    let mut code = 0;
    for (i, f) in files.iter().enumerate() {
        if files.len() > 1 {
            if i > 0 && !numbers {
                let _ = writeln!(out);
            }
            let _ = writeln!(out, "==> {} <==", f);
        }
        match std::fs::File::open(f) {
            Ok(handle) => {
                let reader = std::io::BufReader::new(handle);
                let mut shown = 0usize;
                for line in reader.lines() {
                    match line {
                        Ok(line) => {
                            if shown >= n {
                                break;
                            }
                            let _ = writeln!(out, "{}", line);
                            shown += 1;
                        }
                        Err(_) => break,
                    }
                }
            }
            Err(_) => {
                eprintln!("head: {}: No such file or directory", f);
                code = 1;
            }
        }
    }
    let _ = out.flush();
    code
}

fn head_fd(fd: RawFd, n: usize) -> i32 {
    let mut buf = [0u8; 4096];
    let mut out = String::new();
    loop {
        let r = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if r <= 0 {
            break;
        }
        out.push_str(&String::from_utf8_lossy(&buf[..r as usize]));
        if out.bytes().filter(|&b| b == b'\n').count() >= n {
            break;
        }
    }
    let mut written = 0usize;
    let mut line_count = 0usize;
    for (i, b) in out.bytes().enumerate() {
        if b == b'\n' {
            line_count += 1;
            if line_count > n {
                break;
            }
            written = i + 1;
        }
    }
    if line_count > n {
        let _ = print!("{}", &out[..written]);
    } else {
        let _ = print!("{}", out);
    }
    0
}