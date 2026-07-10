use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;

static CACHE: std::sync::LazyLock<Mutex<Option<HashSet<String>>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));

pub fn find_in_path_cache(cmd: &str) -> Option<PathBuf> {
    let cached = CACHE.lock().unwrap();
    if let Some(set) = cached.as_ref() {
        if set.contains(cmd) {
            return pathsearch::find_executable_in_path(cmd);
        }
        return None;
    }

    // First call: populate cache
    drop(cached);
    refresh_cache();
    let cached = CACHE.lock().unwrap();
    if let Some(set) = cached.as_ref() {
        if set.contains(cmd) {
            return pathsearch::find_executable_in_path(cmd);
        }
    }
    None
}

pub fn refresh_cache() {
    let mut cached = CACHE.lock().unwrap();
    let mut set = HashSet::new();
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        set.insert(name.to_string());
                    }
                }
            }
        }
    }
    *cached = Some(set);
}

pub fn get_cached_commands() -> Vec<String> {
    let cached = CACHE.lock().unwrap();
    if let Some(set) = cached.as_ref() {
        let mut v: Vec<String> = set.iter().cloned().collect();
        v.sort();
        v
    } else {
        drop(cached);
        refresh_cache();
        let cached = CACHE.lock().unwrap();
        if let Some(set) = cached.as_ref() {
            let mut v: Vec<String> = set.iter().cloned().collect();
            v.sort();
            v
        } else {
            Vec::new()
        }
    }
}
