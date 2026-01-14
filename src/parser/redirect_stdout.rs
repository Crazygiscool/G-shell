use std::os::unix::io::{AsRawFd, RawFd};
use std::fs::OpenOptions;

pub fn redirect_stdout_to(filename: &str) -> Option<RawFd> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(filename)
            .ok()?;

        let new_fd = file.as_raw_fd();
        let old_fd = unsafe { libc::dup(1) };

        unsafe {
            libc::dup2(new_fd, 1);
        }

        Some(old_fd)
    }

pub fn restore_stdout(old_fd: RawFd) {
        unsafe {
            libc::dup2(old_fd, 1);
            libc::close(old_fd);
        }
    }