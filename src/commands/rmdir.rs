use crate::syscall;

pub fn rmdir(args: &[&str]) -> i32 {
    if args.is_empty() {
        eprintln!("rmdir: missing operand");
        return 1;
    }
    let mut code = 0;
    for dir in args {
        if syscall::rmdir(dir) < 0 {
            eprintln!("rmdir: failed to remove '{}'", dir);
            code = 1;
        }
    }
    code
}