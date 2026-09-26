pub fn help_cmd(args: &[&str]) {
    if args.is_empty() {
        println!("G-shell v{} - A shell written in Rust", env!("CARGO_PKG_VERSION"));
        println!("Type 'help <name>' for info about a specific command.");
        println!();
        println!("Built-in commands:");
        println!("  alias, cat, cd, clear, cp, date, echo, env, exit, export,");
        println!("  grep, head, help, history, ls, mkdir, mv, pwd, rm, rmdir,");
        println!("  set, sleep, source, test, [, type, unalias, unset, wc");
        println!();
        println!("Features:");
        println!("  glob (*?[]), variables ($NAME ${{}} $? $1..$9 $# $@),");
        println!("  functions (name() {{ ... }} / function name {{ ... }}),");
        println!("  command substitution ($(cmd) `cmd`) via the internal evaluator,");
        println!("  redirections (> >> < << 2> &> 2>&1), heredocs (<<) interactively,");
        println!("  pipelines (|), sequences (;), AND/OR lists (&& ||),");
        println!("  history expansion (!! !$ !N), tab completion, $PS1 prompt");
        println!("  Init file: $GSHELLRC or ~/.gshellrc sourced on startup");
    } else {
        for topic in args {
            match *topic {
                "alias" => println!("alias: alias [name=value ...]\n    Define or display aliases. Aliases expand at execution."),
                "cat" => println!("cat: cat [-n] [file ...]\n    Concatenate files (or stdin) to stdout. -n numbers lines."),
                "cd" => println!("cd: cd [dir]\n    Change the current working directory."),
                "clear" => println!("clear: clear\n    Clear the terminal screen."),
                "cp" => println!("cp: cp [-r] source ... dest\n    Copy files. -r copies directories recursively."),
                "date" => println!("date: date [-u]\n    Print the current date and time from the kernel RTC."),
                "echo" => println!("echo: echo [string ...]\n    Write arguments to standard output."),
                "env" => println!("env: env\n    Display environment variables."),
                "exit" => println!("exit: exit [n]\n    Exit the shell with status n."),
                "export" => println!("export: export [name[=value] ...]\n    Set environment variables."),
                "grep" => println!("grep: grep [-i] [-v] [-n] [-g] [PATTERN] [file ...]\n    Print lines matching a literal pattern (-g: glob pattern)."),
                "head" => println!("head: head [-n N] [file ...]\n    Output the first N lines (default 10) of files or stdin."),
                "help" => println!("help: help [name ...]\n    Display help information."),
                "history" => println!("history: history [-c|-r|-w|-a] [n|file]\n    Display or manipulate history."),
                "ls" => println!("ls: ls [-l] [-a] [dir ...]\n    List directory contents. -l long format, -a include hidden."),
                "mkdir" => println!("mkdir: mkdir [-p] dir ...\n    Create directories. -p creates parents."),
                "mv" => println!("mv: mv source ... dest\n    Move files. Implemented as copy+remove (kernel has no rename(2))."),
                "pwd" => println!("pwd: pwd\n    Print the current working directory."),
                "rm" => println!("rm: rm [-r] [-f] path ...\n    Remove files. -r recurses into directories."),
                "rmdir" => println!("rmdir: rmdir dir ...\n    Remove empty directories."),
                "set" => println!("set: set\n    Display all environment variables."),
                "sleep" => println!("sleep: sleep SECONDS\n    Pause for the given time (fractions allowed, e.g. 0.5)."),
                "source" => println!("source: source <file>\n    Execute commands from a file."),
                "test" | "[" => println!("test: test [expr] or [ [expr] ]\n    Evaluate expression (file tests, string/number compare)."),
                "type" => println!("type: type <name>\n    Display command type (builtin, alias, function, or external)."),
                "unalias" => println!("unalias: unalias <name> ...\n    Remove alias definitions."),
                "unset" => println!("unset: unset <name> ...\n    Unset variables and functions."),
                "wc" => println!("wc: wc [-l] [-w] [-c] [file ...]\n    Count lines, words, and bytes."),
                _ => println!("help: no help topics match '{}'.", topic),
            }
        }
    }
}