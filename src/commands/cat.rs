use std::io::{BufRead, Write};
use std::os::unix::io::RawFd;

pub fn cat(args: &[&str], stdin_fd: Option<RawFd>) -> i32 {
    let mut numbering = false;
    let mut files: Vec<String> = Vec::new();

    for a in args {
        match *a {
            "-n" => numbering = true,
            "-b" => {}
            s if s.starts_with('-') && s.len() > 1 => {
                eprintln!("cat: invalid option -- '{}'", &s[1..2]);
                return 2;
            }
            s => files.push(s.to_string()),
        }
    }

    let stdin = stdin_fd.unwrap_or(0);
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    if files.is_empty() {
        return dump_fd(stdin, numbering, &mut out);
    }

    let mut code = 0;
    for f in &files {
        match std::fs::File::open(f) {
            Ok(handle) => {
                let reader = std::io::BufReader::new(handle);
                let mut n = 1;
                for line in reader.lines() {
                    match line {
                        Ok(line) => {
                            if numbering {
                                let _ = writeln!(out, "{:>6}\t{}", n, line);
                                n += 1;
                            } else {
                                let _ = writeln!(out, "{}", line);
                            }
                        }
                        Err(_) => {
                            // Binary-ish content: fall back to raw copy.
                            return dump_file_raw(f, &mut out);
                        }
                    }
                }
            }
            Err(_) => {
                eprintln!("cat: {}: No such file or directory", f);
                code = 1;
            }
        }
    }
    let _ = out.flush();
    code
}

fn dump_fd(fd: RawFd, numbering: bool, out: &mut impl Write) -> i32 {
    let mut n = 1;
    let mut buf = [0u8; 4096];
    let mut carry: Vec<u8> = Vec::new();
    loop {
        let r = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if r <= 0 {
            break;
        }
        let data = &buf[..r as usize];
        if !carry.is_empty() {
            let mut joined = std::mem::take(&mut carry);
            joined.extend_from_slice(data);
            if numbering {
                let mut start = 0;
                for (i, &b) in joined.iter().enumerate() {
                    if b == b'\n' {
                        let _ = writeln!(out, "{:>6}\t{}", n, String::from_utf8_lossy(&joined[start..i]));
                        n += 1;
                        start = i + 1;
                    }
                }
                if start < joined.len() {
                    carry = joined[start..].to_vec();
                }
            } else {
                let _ = out.write_all(&joined);
            }
            continue;
        }
        if numbering {
            let mut start = 0;
            for (i, &b) in data.iter().enumerate() {
                if b == b'\n' {
                    let _ = writeln!(out, "{:>6}\t{}", n, String::from_utf8_lossy(&data[start..i]));
                    n += 1;
                    start = i + 1;
                }
            }
            if start < data.len() {
                carry = data[start..].to_vec();
            }
        } else {
            let _ = out.write_all(data);
        }
    }
    if !carry.is_empty() {
        let _ = out.write_all(&carry);
    }
    0
}

fn dump_file_raw(path: &str, out: &mut impl Write) -> i32 {
    let mut f = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return 1,
    };
    let mut buf = [0u8; 4096];
    loop {
        match std::io::Read::read(&mut f, &mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                let _ = out.write_all(&buf[..n]);
            }
        }
    }
    0
}