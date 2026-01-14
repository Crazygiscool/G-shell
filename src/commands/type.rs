use pathsearch::find_executable_in_path;

// Changed signature from [5] to a slice [&[[&str; 2]]] so it can handle any number of builtins
pub fn r#type(command: &str, registry: &[[&str; 2]]) {
    if let Some(entry) = registry.iter().find(|cmd| cmd[0] == command) {
        println!("{} is a shell {}", entry[0], entry[1]);
    } else if let Some(path) = find_executable_in_path(command) {
        println!("{} is {}", command, path.display());
    } else {
        println!("{}: not found", command);
    }
}
