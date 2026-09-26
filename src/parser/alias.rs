use std::sync::{LazyLock, Mutex};

/// Aliases keyed by name, as a simple Vec. See functions.rs for why std
/// HashMap/HashSet are avoided: hashbrown's SIMD paths crash on the kernel
/// runtime this shell targets, and shell-scale lookups are linear anyway.
static ALIASES: LazyLock<Mutex<Vec<(String, String)>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

pub fn get_alias_names() -> Vec<String> {
    ALIASES.lock().unwrap().iter().map(|(k, _)| k.clone()).collect()
}

pub fn set_alias(name: String, value: String) {
    let mut aliases = ALIASES.lock().unwrap();
    if let Some(entry) = aliases.iter_mut().find(|(n, _)| *n == name) {
        entry.1 = value;
    } else {
        aliases.push((name, value));
    }
}

pub fn remove_alias(name: &str) -> bool {
    let mut aliases = ALIASES.lock().unwrap();
    let before = aliases.len();
    aliases.retain(|(n, _)| n != name);
    aliases.len() != before
}

pub fn get_alias(name: &str) -> Option<String> {
    ALIASES
        .lock()
        .unwrap()
        .iter()
        .rev()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v.clone())
}