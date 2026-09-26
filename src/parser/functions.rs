use std::sync::{LazyLock, Mutex};

use crate::parser::ast::Program;

/// Function definitions keyed by name. Kept in-process (like alias.rs), so any
/// shell command — builtin, pipeline stage, or command substitution — can call
/// a user-defined function without a fork.
///
/// Stored in a plain Vec (not a HashMap) on purpose: the kernel runtime this
/// shell targets does not reliably handle hashbrown's SIMD control-group
/// loads inside std hash-map insert, so any std HashMap/HashSet use faults
/// there. Shell scale is tiny; a linear scan is plenty.
static FUNCTIONS: LazyLock<Mutex<Vec<(String, Program)>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

pub fn define(name: &str, body: Program) {
    let mut fns = FUNCTIONS.lock().unwrap();
    if let Some(entry) = fns.iter_mut().find(|(n, _)| n == name) {
        entry.1 = body;
    } else {
        fns.push((name.to_string(), body));
    }
}

pub fn get(name: &str) -> Option<Program> {
    FUNCTIONS
        .lock()
        .unwrap()
        .iter()
        .rev()
        .find(|(n, _)| n == name)
        .map(|(_, body)| body.clone())
}

pub fn is_function(name: &str) -> bool {
    FUNCTIONS.lock().unwrap().iter().any(|(n, _)| n == name)
}

pub fn remove(name: &str) {
    let mut fns = FUNCTIONS.lock().unwrap();
    fns.retain(|(n, _)| n != name);
}