//! Thin wrappers around syscalls the kernel implements (see kernel/syscall.c),
//! which aren't all surfaced by libc on the host — notably getdents64. The
//! struct layouts mirror the kernel's (kernel/file.c) so the same code runs on
//! the kernel and on the host.

use std::ffi::CString;
use std::os::unix::io::RawFd;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxDirent64 {
    pub d_ino: u64,
    pub d_off: i64,
    pub d_reclen: u16,
    pub d_type: u8,
    pub d_name: [u8; 256],
}

pub fn getdents64(fd: RawFd, buf: &mut [u8]) -> i64 {
    unsafe {
        libc::syscall(
            libc::SYS_getdents64,
            fd,
            buf.as_mut_ptr(),
            buf.len(),
        )
    }
}

pub fn mkdir(path: &str, mode: u32) -> i64 {
    let c = CString::new(path.to_string()).unwrap_or_default();
    unsafe { libc::syscall(libc::SYS_mkdir, c.as_ptr(), mode) }
}

pub fn rmdir(path: &str) -> i64 {
    let c = CString::new(path.to_string()).unwrap_or_default();
    unsafe { libc::syscall(libc::SYS_rmdir, c.as_ptr()) }
}

pub fn unlink(path: &str) -> i64 {
    let c = CString::new(path.to_string()).unwrap_or_default();
    unsafe { libc::syscall(libc::SYS_unlink, c.as_ptr()) }
}

/// Read the names in a directory (sorted), each with its file type byte.
pub fn read_dir_names(path: &str) -> Option<Vec<(String, u8)>> {
    let c = CString::new(path.to_string()).ok()?;
    let fd = unsafe { libc::open(c.as_ptr(), libc::O_RDONLY) };
    if fd < 0 {
        return None;
    }
    let mut out = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = getdents64(fd, &mut buf);
        if n <= 0 {
            break;
        }
        let mut off = 0usize;
        while off < n as usize {
            let ent = unsafe { &*(buf.as_ptr().add(off) as *const LinuxDirent64) };
            let reclen = ent.d_reclen as usize;
            if reclen > 0 {
                let mut name = Vec::new();
                for &b in &ent.d_name {
                    if b == 0 {
                        break;
                    }
                    name.push(b);
                }
                let name = String::from_utf8_lossy(&name).to_string();
                if !name.is_empty() && name != "." && name != ".." {
                    out.push((name, ent.d_type));
                }
            }
            off += reclen;
        }
    }
    unsafe { libc::close(fd); }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Some(out)
}

pub fn is_dir(path: &str) -> bool {
    let c = CString::new(path.to_string()).unwrap_or_default();
    let mut st = std::mem::MaybeUninit::<libc::stat>::uninit();
    let r = unsafe { libc::stat(c.as_ptr(), st.as_mut_ptr()) };
    if r < 0 {
        return false;
    }
    unsafe { ((*st.as_ptr()).st_mode) & libc::S_IFMT == libc::S_IFDIR }
}

pub fn is_file(path: &str) -> bool {
    let c = CString::new(path.to_string()).unwrap_or_default();
    let mut st = std::mem::MaybeUninit::<libc::stat>::uninit();
    let r = unsafe { libc::stat(c.as_ptr(), st.as_mut_ptr()) };
    if r < 0 {
        return false;
    }
    unsafe { ((*st.as_ptr()).st_mode) & libc::S_IFMT == libc::S_IFREG }
}

pub fn exists(path: &str) -> bool {
    is_dir(path) || is_file(path)
}

pub fn file_size(path: &str) -> Option<u64> {
    let c = CString::new(path.to_string()).unwrap_or_default();
    let mut st = std::mem::MaybeUninit::<libc::stat>::uninit();
    if unsafe { libc::stat(c.as_ptr(), st.as_mut_ptr()) } < 0 {
        return None;
    }
    Some(unsafe { (*st.as_ptr()).st_size as u64 })
}

pub fn join_dir(dir: &str, name: &str) -> String {
    if dir.ends_with('/') {
        format!("{}{}", dir, name)
    } else {
        format!("{}/{}", dir, name)
    }
}

/// CLOCK_REALTIME wall clock as (seconds, nanoseconds).
pub fn realtime_clock() -> Option<(i64, u32)> {
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    let r = unsafe { libc::syscall(libc::SYS_clock_gettime, 0, &mut ts as *mut _) };
    if r < 0 {
        None
    } else {
        Some((ts.tv_sec as i64, ts.tv_nsec as u32))
    }
}

/// Read everything from an fd (0 = stdin) as a String.
pub fn read_all(fd: RawFd) -> String {
    let mut out = String::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n <= 0 {
            break;
        }
        out.push_str(&String::from_utf8_lossy(&buf[..n as usize]));
    }
    out
}