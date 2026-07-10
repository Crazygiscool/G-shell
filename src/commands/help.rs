pub fn help_cmd(args: &[&str]) {
    if args.is_empty() {
        println!("G-shell v{} - A shell written in Rust", env!("CARGO_PKG_VERSION"));
        println!("Type 'help <name>' for info about a specific command.");
        println!();
        println!("Built-in commands:");
        println!("  alias, cd, echo, env, exit, export, help, history,");
        println!("  pwd, set, source, test, type, unalias, unset");
        println!();
        println!("Features:");
        println!("  glob (*?[]), variables ($NAME ${{}} $?), command substitution");
        println!("  ($(cmd) `cmd`), heredocs (<<), pipelines (|), sequences (;),");
        println!("  AND/OR lists (&& ||), background (&), history expansion");
        println!("  (!! !$ !N), tab completion, $PS1 prompt");
    } else {
        for topic in args {
            match *topic {
                "alias" => println!("alias: alias [name=value ...]\n    Define or display aliases."),
                "cd" => println!("cd: cd [dir]\n    Change the current working directory."),
                "echo" => println!("echo: echo [string ...]\n    Write arguments to standard output."),
                "env" => println!("env: env\n    Display environment variables."),
                "exit" => println!("exit: exit [n]\n    Exit the shell with status n."),
                "export" => println!("export: export [name[=value] ...]\n    Set environment variables."),
                "help" => println!("help: help [name ...]\n    Display help information."),
                "history" => println!("history: history [-c|-r|-w|-a] [n|file]\n    Display or manipulate history."),
                "pwd" => println!("pwd: pwd\n    Print the current working directory."),
                "set" => println!("set: set\n    Display all environment variables."),
                "source" => println!("source: source <file>\n    Execute commands from a file."),
                "test" | "[" => println!("test: test [expr] or [ [expr] ]\n    Evaluate expression (file tests, string/number compare)."),
                "type" => println!("type: type <name>\n    Display command type (builtin or external)."),
                "unalias" => println!("unalias: unalias <name> ...\n    Remove alias definitions."),
                "unset" => println!("unset: unset <name> ...\n    Unset environment variables."),
                _ => println!("help: no help topics match '{}'.", topic),
            }
        }
    }
}
