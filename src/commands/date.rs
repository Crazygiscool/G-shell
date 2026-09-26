use crate::syscall;

pub fn date(args: &[&str]) -> i32 {
    let mut utc = false;
    for a in args {
        match *a {
            "-u" | "--utc" | "--universal" => utc = true,
            "--help" => {
                println!("date: date [-u]");
                println!("    Print the current date and time.");
                return 0;
            }
            _ => {
                eprintln!("date: invalid option -- '{}'", a);
                return 2;
            }
        }
    }

    let Some((secs, nsecs)) = syscall::realtime_clock() else {
        eprintln!("date: cannot read the clock");
        return 1;
    };
    let _ = (nsecs, utc);
    print_civil(secs);
    0
}

/// Convert a UNIX epoch to a formatted civil date without chrono/libc tm.
/// Based on the classic Howard Hinnant / civil_from_days algorithm.
fn print_civil(epoch_secs: i64) {
    let days = epoch_secs.div_euclid(86400);
    let rem = epoch_secs - days * 86400;
    let hour = rem / 3600;
    let min = (rem % 3600) / 60;
    let sec = rem % 60;

    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    println!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        y, m, d, hour, min, sec
    );
}