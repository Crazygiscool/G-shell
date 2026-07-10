use std::path::Path;

pub fn test_builtin(input_args: &[&str]) -> i32 {
    if input_args.is_empty() {
        return 1;
    }

    let args = if input_args.last().map(|s| *s == "]").unwrap_or(false) {
        &input_args[..input_args.len() - 1]
    } else {
        input_args
    };

    if args.is_empty() {
        return 1;
    }

    match args[0] {
        "-f" => {
            if args.len() < 2 { return 1; }
            if Path::new(args[1]).is_file() { 0 } else { 1 }
        }
        "-d" => {
            if args.len() < 2 { return 1; }
            if Path::new(args[1]).is_dir() { 0 } else { 1 }
        }
        "-e" => {
            if args.len() < 2 { return 1; }
            if Path::new(args[1]).exists() { 0 } else { 1 }
        }
        "-n" => {
            if args.len() < 2 { return 1; }
            if !args[1].is_empty() { 0 } else { 1 }
        }
        "-z" => {
            if args.len() < 2 { return 1; }
            if args[1].is_empty() { 0 } else { 1 }
        }
        "-x" => {
            if args.len() < 2 { return 1; }
            if Path::new(args[1]).is_file() {
                use std::os::unix::fs::MetadataExt;
                if let Ok(meta) = std::fs::metadata(args[1]) {
                    if meta.mode() & 0o111 != 0 { return 0; }
                }
            }
            1
        }
        "-r" => {
            if args.len() < 2 { return 1; }
            if Path::new(args[1]).is_file() {
                if std::fs::File::open(args[1]).is_ok() { 0 } else { 1 }
            } else {
                1
            }
        }
        "-w" => {
            if args.len() < 2 { return 1; }
            if let Ok(meta) = std::fs::metadata(args[1]) {
                if !meta.permissions().readonly() { 0 } else { 1 }
            } else {
                1
            }
        }
        "-s" => {
            if args.len() < 2 { return 1; }
            if let Ok(meta) = std::fs::metadata(args[1]) {
                if meta.len() > 0 { 0 } else { 1 }
            } else {
                1
            }
        }
        _ => {
            if args.len() == 1 {
                if !args[0].is_empty() { 0 } else { 1 }
            } else if args.len() >= 3 {
                match args[1] {
                    "=" => {
                        if args[0] == args[2] { 0 } else { 1 }
                    }
                    "!=" => {
                        if args[0] != args[2] { 0 } else { 1 }
                    }
                    "-eq" => {
                        let a: i64 = args[0].parse().unwrap_or(0);
                        let b: i64 = args[2].parse().unwrap_or(0);
                        if a == b { 0 } else { 1 }
                    }
                    "-ne" => {
                        let a: i64 = args[0].parse().unwrap_or(0);
                        let b: i64 = args[2].parse().unwrap_or(0);
                        if a != b { 0 } else { 1 }
                    }
                    "-lt" => {
                        let a: i64 = args[0].parse().unwrap_or(0);
                        let b: i64 = args[2].parse().unwrap_or(0);
                        if a < b { 0 } else { 1 }
                    }
                    "-gt" => {
                        let a: i64 = args[0].parse().unwrap_or(0);
                        let b: i64 = args[2].parse().unwrap_or(0);
                        if a > b { 0 } else { 1 }
                    }
                    "-le" => {
                        let a: i64 = args[0].parse().unwrap_or(0);
                        let b: i64 = args[2].parse().unwrap_or(0);
                        if a <= b { 0 } else { 1 }
                    }
                    "-ge" => {
                        let a: i64 = args[0].parse().unwrap_or(0);
                        let b: i64 = args[2].parse().unwrap_or(0);
                        if a >= b { 0 } else { 1 }
                    }
                    _ => 1,
                }
            } else {
                1
            }
        }
    }
}
