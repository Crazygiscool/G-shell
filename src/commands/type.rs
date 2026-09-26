use crate::parser::{alias, functions, pathcache};

pub fn r#type(command: &str, registry: &[(&str, &str, &str)]) {
    if let Some(entry) = registry.iter().find(|cmd| cmd.0 == command) {
        println!("{} is a shell {}", entry.0, entry.1);
    } else if let Some(_) = alias::get_alias(command) {
        println!("{} is aliased to `{}'", command, alias::get_alias(command).unwrap_or_default());
    } else if functions::is_function(command) {
        println!("{} is a function", command);
    } else if let Some(path) = pathcache::find_in_path_cache(command) {
        println!("{} is {}", command, path.display());
    } else {
        println!("{}: not found", command);
    }
}