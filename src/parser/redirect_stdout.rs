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

pub fn redirect_stdout_append(filename: &str) -> Option<RawFd> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(filename)
            .ok()?;

        let new_fd = file.as_raw_fd();
        let old_fd = unsafe { libc::dup(1) };

        unsafe {
            libc::dup2(new_fd, 1);
        }

        Some(old_fd)
    }

pub fn redirect_stderr_to(filename: &str) -> Option<RawFd> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(filename)
            .ok()?;

        let new_fd = file.as_raw_fd();
        let old_fd = unsafe { libc::dup(2) };

        unsafe {
            libc::dup2(new_fd, 2);
        }

        Some(old_fd)
    }

pub fn redirect_stderr_append(filename: &str) -> Option<RawFd> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(filename)
            .ok()?;

        let new_fd = file.as_raw_fd();
        let old_fd = unsafe { libc::dup(2) };

        unsafe {
            libc::dup2(new_fd, 2);
        }

        Some(old_fd)
    }

pub fn redirect_stdin_from(filename: &str) -> Option<RawFd> {
        let file = OpenOptions::new()
            .read(true)
            .open(filename)
            .ok()?;

        let new_fd = file.as_raw_fd();
        let old_fd = unsafe { libc::dup(0) };

        unsafe {
            libc::dup2(new_fd, 0);
        }

        Some(old_fd)
    }

pub fn restore_fd(fd: RawFd, target: i32) {
        unsafe {
            libc::dup2(fd, target);
            libc::close(fd);
        }
    }
