mod commands;
mod parser;
mod syscall;

use parser::shell;

const VERSION: &str = env!("CARGO_PKG_VERSION");

static USAGE: &str = "\
Usage: gsh [OPTIONS] [FILE]...

G-Shell: a small, self-contained shell for the G-Zero kernel
(with a host build for testing).

Options:
  -c COMMAND     Execute COMMAND instead of starting an interactive session
  -h, --help     Print this help and exit
  --norc         Do not source the rcfile at startup
  --version      Print version information and exit

A FILE argument (a shell script) is read and executed line-by-line, then the
shell exits. Multi-line constructs (heredocs) are only supported interactively.
";

struct Cli {
    command: Option<String>,
    script: Option<String>,
    norc: bool,
    show_version: bool,
    show_help: bool,
}

fn parse_args() -> Cli {
    let mut cli = Cli {
        command: None,
        script: None,
        norc: false,
        show_version: false,
        show_help: false,
    };
    let mut script_lines: Vec<String> = Vec::new();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "-c" => {
                i += 1;
                if let Some(cmd) = args.get(i) {
                    cli.command = Some(cmd.clone());
                }
            }
            "--norc" => cli.norc = true,
            "-h" | "--help" => cli.show_help = true,
            "--version" => cli.show_version = true,
            s if s.starts_with('-') && s.len() > 1 => {
                eprintln!("gsh: unknown option: {}", s);
                eprintln!("Try 'gsh --help' for more information.");
                cli.show_help = true;
            }
            s => script_lines.push(s.to_string()),
        }
        i += 1;
    }
    if !script_lines.is_empty() {
        cli.script = Some(script_lines.join(" "));
    }
    cli
}

fn main() -> rustyline::Result<()> {
    let cli = parse_args();

    if cli.show_help {
        print!("{}", USAGE);
        return Ok(());
    }
    if cli.show_version {
        println!("G-Shell {}", VERSION);
        return Ok(());
    }

    let mut shell = match shell::Shell::new_with_opts(cli.norc) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize shell: {}", e);
            return Err(e);
        }
    };

    if let Some(cmd) = cli.command {
        shell.run_command_string(&cmd);
        std::process::exit(shell.last_exit_code.max(0).min(255));
    }

    if let Some(script) = cli.script {
        let content = match std::fs::read_to_string(&script) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("gsh: cannot open '{}': {}", script, e);
                return Ok(());
            }
        };
        shell.run_command_string(&content);
        std::process::exit(shell.last_exit_code.max(0).min(255));
    }

    // Interactive REPL (the kernel launches gsh with no arguments).
    if let Err(e) = shell.run() {
        eprintln!("Shell error: {}", e);
        return Err(e);
    }

    Ok(())
}