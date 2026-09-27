use std::path::PathBuf;

/// Collect the set of executable names on PATH as a sorted Vec.
///
/// Deliberately not a HashSet: hashbrown's SIMD control-group loads page-fault
/// on the G-Zero kernel at insert time (see functions.rs for details), and the
/// on-disk PATH set is small enough that a linear search is never a problem.
fn build_cache() -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        if !names.iter().any(|n| n == name) {
                            names.push(name.to_string());
                        }
                    }
                }
            }
        }
    }
    names.sort();
    names
}

pub fn find_in_path_cache(cmd: &str) -> Option<PathBuf> {
    if build_cache().iter().any(|n| n == cmd) {
        return pathsearch::find_executable_in_path(cmd);
    }
    None
}

pub fn get_cached_commands() -> Vec<String> {
    build_cache()
}