use std::collections::HashSet;
use std::path::PathBuf;

pub fn find_in_path_cache(cmd: &str) -> Option<PathBuf> {
    if build_cache().contains(&cmd.to_string()) {
        return pathsearch::find_executable_in_path(cmd);
    }
    None
}

fn build_cache() -> HashSet<String> {
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
    set
}

pub fn get_cached_commands() -> Vec<String> {
    let mut v: Vec<String> = build_cache().into_iter().collect();
    v.sort();
    v
}
