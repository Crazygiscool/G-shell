use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

static ALIASES: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn get_alias_names() -> Vec<String> {
    let aliases = ALIASES.lock().unwrap();
    aliases.keys().cloned().collect()
}

pub fn set_alias(name: String, value: String) {
    let mut aliases = ALIASES.lock().unwrap();
    aliases.insert(name, value);
}

pub fn remove_alias(name: &str) -> bool {
    let mut aliases = ALIASES.lock().unwrap();
    aliases.remove(name).is_some()
}

pub fn get_alias(name: &str) -> Option<String> {
    let aliases = ALIASES.lock().unwrap();
    aliases.get(name).cloned()
}
