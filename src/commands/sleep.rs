use std::time::Duration;

pub fn sleep(args: &[&str]) -> i32 {
    if args.is_empty() {
        eprintln!("sleep: missing operand");
        return 1;
    }
    if args.len() > 1 {
        eprintln!("sleep: too many arguments");
        return 1;
    }

    let raw = args[0];
    let secs_f64: f64 = match raw.parse() {
        Ok(v) if v >= 0.0 => v,
        _ => {
            eprintln!("sleep: invalid time interval '{}'", raw);
            return 1;
        }
    };
    let dur = Duration::from_secs_f64(secs_f64);
    std::thread::sleep(dur);
    0
}