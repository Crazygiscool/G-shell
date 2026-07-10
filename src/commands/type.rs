use crate::parser::pathcache;

pub fn r#type(command: &str, registry: &[[&str; 2]]) {
        if let Some(entry) = registry.iter().find(|cmd| cmd[0] == command) {
            println!("{} is a shell {}", entry[0], entry[1]);
        } else if let Some(path) = pathcache::find_in_path_cache(command) {
            println!("{} is {}", command, path.display());
        } else {
            println!("{}: not found", command);
        }
    }
