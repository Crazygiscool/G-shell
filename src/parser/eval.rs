use std::os::unix::io::RawFd;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use os_pipe::pipe;

use crate::parser::ast::*;
use crate::parser::redirect_stdout::{
    redirect_stdout_to, redirect_stdout_append,
    redirect_stderr_to, redirect_stderr_append,
    redirect_stdin_from, restore_fd,
};
use crate::parser::expand::expand_tokens;
use crate::parser::glob::{expand_globs, glob_match};
use crate::parser::pathcache;
use crate::commands::{echo, cd, pwd, r#type, env, test, help};

static ALIASES: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<String, String>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

const BUILTIN_REGISTRY: &[[&str; 2]; 16] = &[
    ["echo", "builtin"],
    ["type", "builtin"],
    ["exit", "builtin"],
    ["pwd", "builtin"],
    ["cd", "builtin"],
    ["history", "builtin"],
    ["export", "builtin"],
    ["unset", "builtin"],
    ["set", "builtin"],
    ["env", "builtin"],
    ["source", "builtin"],
    ["test", "builtin"],
    ["[", "builtin"],
    ["alias", "builtin"],
    ["unalias", "builtin"],
    ["help", "builtin"],
];

fn is_builtin(name: &str) -> bool {
    BUILTIN_REGISTRY.iter().any(|e| e[0] == name)
}

// ── Top-level evaluation ──

pub fn eval_program(program: &Program, history_data: &[String], last_exit_code: i32) -> i32 {
    let mut code = last_exit_code;
    for cmd in &program.commands {
        code = eval_complete_command(cmd, history_data, code);
    }
    code
}

fn eval_complete_command(cmd: &CompleteCommand, history_data: &[String], last_exit_code: i32) -> i32 {
    let code = eval_and_or_list(&cmd.and_or, history_data, last_exit_code);
    if cmd.background {
        // Background is handled at the shell level before calling eval
        // If we reach here, the command was run synchronously
    }
    code
}

fn eval_and_or_list(list: &AndOrList, history_data: &[String], last_exit_code: i32) -> i32 {
    let mut code = last_exit_code;
    for node in &list.nodes {
        let should_run = match node.operator {
            None => true,
            Some(AndOrOp::And) => code == 0,
            Some(AndOrOp::Or) => code != 0,
        };
        if should_run {
            code = eval_command_node(&node.command, history_data, code);
        }
    }
    code
}

fn eval_command_node(node: &CommandNode, history_data: &[String], last_exit_code: i32) -> i32 {
    match node {
        CommandNode::Pipeable(pipeline) => eval_pipeline(pipeline, history_data, last_exit_code),
        CommandNode::Compound(script) => eval_script(script, history_data, last_exit_code),
    }
}

// ── Pipeline evaluation ──

fn eval_pipeline(pipeline: &Pipeline, history_data: &[String], last_exit_code: i32) -> i32 {
    let count = pipeline.commands.len();
    if count == 0 {
        return if pipeline.negated { 1 } else { 0 };
    }

    let code = if count == 1 {
        eval_simple_command(&pipeline.commands[0], history_data, last_exit_code, None)
    } else {
        eval_multi_command_pipeline(&pipeline.commands, history_data, last_exit_code)
    };

    if pipeline.negated {
        if code == 0 { 1 } else { 0 }
    } else {
        code
    }
}

fn eval_multi_command_pipeline(
    commands: &[SimpleCommand],
    history_data: &[String],
    last_exit_code: i32,
) -> i32 {
    let total = commands.len();
    let mut children = Vec::new();
    let mut prev_stdin: Option<Stdio> = None;
    let mut exit_code = 0;

    for (i, cmd) in commands.iter().enumerate() {
        let is_last = i == total - 1;
        let expanded = expand_and_glob(&cmd.words, last_exit_code);

        if expanded.is_empty() {
            exit_code = 0;
            continue;
        }

        let program = &expanded[0];
        let args: Vec<&str> = expanded.iter().skip(1).map(|s| s.as_str()).collect();

        if is_builtin(program) {
            if is_last {
                let code = run_builtin(program, &args, history_data, last_exit_code);
                exit_code = code;
                prev_stdin = None;
            } else {
                let (reader, mut writer) = pipe().expect("pipe failed");
                let output = get_builtin_output(program, &args, history_data);
                let _ = std::io::Write::write_all(&mut writer, output.as_bytes());
                drop(writer);
                prev_stdin = Some(Stdio::from(reader));
            }
            continue;
        }

        // External command
        if let Some(path) = pathcache::find_in_path_cache(program) {
            let mut child_cmd = Command::new(&path);
            child_cmd.arg0(program);
            child_cmd.args(&args);

            if let Some(stdin) = prev_stdin.take() {
                child_cmd.stdin(stdin);
            }
            if !is_last {
                child_cmd.stdout(Stdio::piped());
            } else {
                child_cmd.stdout(Stdio::inherit());
            }
            child_cmd.stderr(Stdio::inherit());

            match child_cmd.spawn() {
                Ok(mut child) => {
                    if !is_last {
                        if let Some(out) = child.stdout.take() {
                            prev_stdin = Some(Stdio::from(out));
                        }
                    }
                    children.push(child);
                }
                Err(e) => {
                    eprintln!("{}: {}", program, e);
                    for c in &mut children { let _ = c.kill(); }
                    exit_code = 127;
                    break;
                }
            }
        } else {
            eprintln!("{}: command not found", program);
            exit_code = 127;
            break;
        }
    }

    for mut child in children {
        match child.wait() {
            Ok(status) => exit_code = status.code().unwrap_or(1),
            Err(_) => exit_code = 1,
        }
    }

    exit_code
}

// ── Simple command evaluation (single command, no pipe) ──

fn eval_simple_command(
    cmd: &SimpleCommand,
    history_data: &[String],
    last_exit_code: i32,
    opt_stdin: Option<Stdio>,
) -> i32 {
    // Apply env overrides
    let originals: Vec<(String, Option<String>)> = cmd.env_overrides.iter().map(|(k, v)| {
        let old = std::env::var(k).ok();
        unsafe { std::env::set_var(k, v); }
        (k.clone(), old)
    }).collect();

    // Apply redirects via dup2 (for builtins)
    // For externals we use Command::stdout/stderr/stdin in the execute function
    let saved_fds: Vec<(i32, RawFd)> = apply_redirects(&cmd.redirects);

    let expanded = expand_and_glob(&cmd.words, last_exit_code);
    let is_pure_assignment = expanded.is_empty() && !cmd.env_overrides.is_empty();
    let code = if is_pure_assignment {
        0
    } else if expanded.is_empty() {
        0
    } else {
        let program = &expanded[0];
        let args: Vec<&str> = expanded.iter().skip(1).map(|s| s.as_str()).collect();
        run_command(program, &args, &cmd.redirects, opt_stdin, history_data, last_exit_code)
    };

    // Restore redirects
    for (target, saved_fd) in saved_fds.into_iter().rev() {
        restore_fd(saved_fd, target);
    }

    // Restore env overrides (skip for pure assignment — persist in shell)
    if !is_pure_assignment {
        for (name, old) in originals.into_iter().rev() {
            match old {
                Some(v) => unsafe { std::env::set_var(&name, v); },
                None => unsafe { std::env::remove_var(&name); },
            }
        }
    }

    code
}

fn run_command(
    program: &str,
    args: &[&str],
    redirects: &[Redirect],
    opt_stdin: Option<Stdio>,
    history_data: &[String],
    last_exit_code: i32,
) -> i32 {
    match program {
        "echo" => {
            if args.first().map(|s| *s == "--help").unwrap_or(false) {
                println!("echo: echo [string ...]");
                println!("    Write arguments to standard output.");
            } else {
                echo::echo(args);
            }
            0
        }
        "type" => {
            if args.first().map(|s| *s == "--help").unwrap_or(false) {
                println!("type: type [name ...]");
                println!("    Display information about command type.");
            } else if let Some(first) = args.first() {
                r#type::r#type(first, BUILTIN_REGISTRY);
            } else {
                eprintln!("type: missing operand");
                return 1;
            }
            0
        }
        "pwd" => {
            if args.first().map(|s| *s == "--help").unwrap_or(false) {
                println!("pwd: pwd");
                println!("    Print the current working directory.");
            } else {
                pwd::pwd();
            }
            0
        }
        "cd" => {
            if args.first().map(|s| *s == "--help").unwrap_or(false) {
                println!("cd: cd [dir]");
                println!("    Change the current working directory.");
            } else {
                cd::cd(args.first().unwrap_or(&""));
            }
            0
        }
        "exit" => {
            let code = args.first().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
            std::process::exit(code);
        }
        "export" => {
            if args.is_empty() {
                env::set_vars();
            } else {
                env::export_var(args);
            }
            0
        }
        "unset" => {
            env::unset_var(args);
            0
        }
        "set" => {
            env::set_vars();
            0
        }
        "env" => {
            env::env_vars();
            0
        }
        "source" => {
            if let Some(path) = args.first() {
                match std::fs::read_to_string(path) {
                    Ok(contents) => {
                        for line in contents.lines() {
                            let line = line.trim();
                            if line.is_empty() || line.starts_with('#') {
                                continue;
                            }
                            // Recursively evaluate each line
                            let tokens = crate::parser::tokenize::tokenize(line);
                            if tokens.is_empty() { continue; }
                            let prog = crate::parser::parser::parse(&tokens);
                            eval_program(&prog, history_data, last_exit_code);
                        }
                        0
                    }
                    Err(e) => {
                        eprintln!("{}: {}: {}", program, path, e);
                        1
                    }
                }
            } else {
                eprintln!("source: missing filename");
                1
            }
        }
        "alias" => {
            let mut alias_table = ALIASES.lock().unwrap();
            for arg in args {
                if let Some(eq_pos) = arg.find('=') {
                    let name = arg[..eq_pos].to_string();
                    let value = arg[eq_pos + 1..].trim_matches('\'').trim_matches('"').to_string();
                    alias_table.insert(name, value);
                } else {
                    if let Some(value) = alias_table.get(*arg) {
                        println!("alias {}='{}'", arg, value);
                    }
                }
            }
            0
        }
        "unalias" => {
            let mut alias_table = ALIASES.lock().unwrap();
            for arg in args {
                alias_table.remove(*arg);
            }
            0
        }
        "help" => {
            help::help_cmd(args);
            0
        }
        "test" | "[" => test::test_builtin(args),
        "history" => {
            // In non-special-cased context (pipe, compound), just display full history
            for (i, entry) in history_data.iter().enumerate() {
                println!("{:>5}  {}", i + 1, entry);
            }
            0
        }
        _ => {
            // External command
            if let Some(path) = pathcache::find_in_path_cache(program) {
                let mut child = Command::new(&path);
                child.arg0(program);
                child.args(args);

                // Apply redirects for external command
                apply_redirects_to_cmd(&mut child, redirects);

                if let Some(stdin) = opt_stdin {
                    child.stdin(stdin);
                }

                match child.status() {
                    Ok(status) => status.code().unwrap_or(1),
                    Err(e) => {
                        eprintln!("{}: {}", program, e);
                        127
                    }
                }
            } else {
                eprintln!("{}: command not found", program);
                127
            }
        }
    }
}

// ── Script command evaluation ──

fn eval_script(script: &ScriptCommand, history_data: &[String], last_exit_code: i32) -> i32 {
    match script {
        ScriptCommand::Subshell(program) => {
            // Fork and eval in child (simplified: eval directly, can't affect parent)
            eval_program(program, history_data, last_exit_code)
        }
        ScriptCommand::If(if_cmd) => eval_if(if_cmd, history_data, last_exit_code),
        ScriptCommand::For(for_cmd) => eval_for(for_cmd, history_data, last_exit_code),
        ScriptCommand::While(while_cmd) => eval_while(while_cmd, history_data, last_exit_code),
        ScriptCommand::Case(case_cmd) => eval_case(case_cmd, history_data, last_exit_code),
        ScriptCommand::Function(func) => {
            // Store function definition for later use
            let mut alias_table = ALIASES.lock().unwrap();
            // Functions are stored as aliases that run the body program
            // For now, store a marker that allows calling the function
            // The function body is serialized as the alias value
            let body_str = format!("__gshell_fn_body__"); // placeholder
            alias_table.insert(format!("__fn_{}", func.name), body_str);
            // TODO: proper function storage and execution
            0
        }
    }
}

fn eval_if(if_cmd: &IfCommand, history_data: &[String], last_exit_code: i32) -> i32 {
    for clause in &if_cmd.clauses {
        let cond_code = eval_program(&clause.condition, history_data, last_exit_code);
        if cond_code == 0 {
            return eval_program(&clause.body, history_data, last_exit_code);
        }
    }
    if let Some(else_body) = &if_cmd.else_body {
        eval_program(else_body, history_data, last_exit_code)
    } else {
        0
    }
}

fn eval_for(for_cmd: &ForCommand, history_data: &[String], _last_exit_code: i32) -> i32 {
    let mut code = 0;
    for word in &for_cmd.words {
        unsafe { std::env::set_var(&for_cmd.var, word); }
        code = eval_program(&for_cmd.body, history_data, code);
    }
    code
}

fn eval_while(while_cmd: &WhileCommand, history_data: &[String], _last_exit_code: i32) -> i32 {
    let mut code = 0;
    loop {
        let cond_code = eval_program(&while_cmd.condition, history_data, code);
        if cond_code != 0 {
            break;
        }
        code = eval_program(&while_cmd.body, history_data, code);
    }
    code
}

fn eval_case(case_cmd: &CaseCommand, history_data: &[String], last_exit_code: i32) -> i32 {
    let word = &case_cmd.word;
    for item in &case_cmd.items {
        for pattern in &item.patterns {
            if glob_match_simple(pattern, word) {
                return eval_program(&item.body, history_data, last_exit_code);
            }
        }
    }
    0
}

fn glob_match_simple(pattern: &str, text: &str) -> bool {
    glob_match(pattern, text)
}

// ── Redirect helpers ──

fn apply_redirects(redirects: &[Redirect]) -> Vec<(i32, RawFd)> {
    let mut saved = Vec::new();
    for r in redirects {
        let result = match r.kind {
            RedirectKind::Output if r.fd == 1 => redirect_stdout_to(&r.target).map(|f| (1, f)),
            RedirectKind::Output if r.fd == 2 => redirect_stderr_to(&r.target).map(|f| (2, f)),
            RedirectKind::Append if r.fd == 1 => redirect_stdout_append(&r.target).map(|f| (1, f)),
            RedirectKind::Append if r.fd == 2 => redirect_stderr_append(&r.target).map(|f| (2, f)),
            RedirectKind::Input => redirect_stdin_from(&r.target).map(|f| (0, f)),
            _ => None,
        };
        if let Some(pair) = result {
            saved.push(pair);
        }
    }
    saved
}

fn apply_redirects_to_cmd(cmd: &mut Command, redirects: &[Redirect]) {
    use std::fs::File;
    for r in redirects {
        match r.kind {
            RedirectKind::Output => {
                if let Ok(f) = File::create(&r.target) {
                    if r.fd == 1 { cmd.stdout(f); }
                    else if r.fd == 2 { cmd.stderr(f); }
                }
            }
            RedirectKind::Append => {
                use std::fs::OpenOptions;
                if let Ok(f) = OpenOptions::new().write(true).create(true).append(true).open(&r.target) {
                    if r.fd == 1 { cmd.stdout(f); }
                    else if r.fd == 2 { cmd.stderr(f); }
                }
            }
            RedirectKind::Input => {
                if let Ok(f) = File::open(&r.target) {
                    cmd.stdin(f);
                }
            }
            RedirectKind::Heredoc => {
                if let Ok(f) = File::open(&r.target) {
                    cmd.stdin(f);
                }
            }
        }
    }
}

// ── Expansion + glob ──

fn expand_and_glob(words: &[String], last_exit_code: i32) -> Vec<String> {
    let expanded = expand_tokens(words, last_exit_code);
    expand_globs(&expanded)
}

// ── Builtin helpers for pipes ──

fn get_builtin_output(name: &str, args: &[&str], history_data: &[String]) -> String {
    match name {
        "echo" => format!("{}\n", args.join(" ")),
        "pwd" => format!("{}\n", std::env::current_dir().unwrap_or_default().display()),
        "history" => {
            history_data.iter().enumerate()
                .map(|(i, s)| format!("  {:>3}  {}\n", i + 1, s))
                .collect::<String>()
        }
        "type" => {
            if let Some(cmd) = args.first() {
                if is_builtin(cmd) {
                    format!("{} is a shell builtin\n", cmd)
                } else if let Some(path) = pathcache::find_in_path_cache(cmd) {
                    format!("{} is {}\n", cmd, path.display())
                } else {
                    format!("{}: not found\n", cmd)
                }
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}

fn run_builtin(name: &str, args: &[&str], history_data: &[String], last_exit_code: i32) -> i32 {
    let code = run_command(name, args, &[], None, history_data, last_exit_code);
    std::io::Write::flush(&mut std::io::stdout()).ok();
    code
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Helpers ───

    fn sc(words: &[&str]) -> SimpleCommand {
        SimpleCommand { env_overrides: vec![], words: words.iter().map(|s| s.to_string()).collect(), redirects: vec![] }
    }

    fn sc_env(env: &[(&str, &str)], words: &[&str]) -> SimpleCommand {
        SimpleCommand {
            env_overrides: env.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            words: words.iter().map(|s| s.to_string()).collect(),
            redirects: vec![],
        }
    }

    fn pipeable(negated: bool, cmds: Vec<SimpleCommand>) -> CommandNode {
        CommandNode::Pipeable(Pipeline { negated, commands: cmds })
    }

    fn compound(sc: ScriptCommand) -> CommandNode {
        CommandNode::Compound(sc)
    }

    fn complete(cmd: CommandNode) -> CompleteCommand {
        CompleteCommand {
            and_or: AndOrList { nodes: vec![AndOrNode { command: cmd, operator: None }] },
            background: false,
        }
    }

    fn program(commands: Vec<CompleteCommand>) -> Program {
        Program { commands }
    }

    // ─── is_builtin ───

    #[test]
    fn test_is_builtin_true() {
        assert!(is_builtin("echo"));
        assert!(is_builtin("cd"));
        assert!(is_builtin("pwd"));
        assert!(is_builtin("type"));
        assert!(is_builtin("exit"));
        assert!(is_builtin("history"));
        assert!(is_builtin("export"));
        assert!(is_builtin("unset"));
        assert!(is_builtin("alias"));
        assert!(is_builtin("unalias"));
        assert!(is_builtin("source"));
        assert!(is_builtin("help"));
        assert!(is_builtin("test"));
        assert!(is_builtin("["));
        assert!(is_builtin("env"));
        assert!(is_builtin("set"));
    }

    #[test]
    fn test_is_builtin_false() {
        assert!(!is_builtin("cat"));
        assert!(!is_builtin("ls"));
        assert!(!is_builtin("grep"));
        assert!(!is_builtin("foobar"));
    }

    // ─── glob_match_simple ───

    #[test]
    fn test_glob_match_exact() {
        assert!(glob_match_simple("hello", "hello"));
    }

    #[test]
    fn test_glob_match_wildcard() {
        assert!(glob_match_simple("h*", "hello"));
        assert!(glob_match_simple("*o", "hello"));
        assert!(glob_match_simple("*", "anything"));
    }

    #[test]
    fn test_glob_match_question() {
        assert!(glob_match_simple("h?llo", "hello"));
        assert!(glob_match_simple("h???o", "hello"));
    }

    #[test]
    fn test_glob_match_no_match() {
        assert!(!glob_match_simple("hello", "world"));
        assert!(!glob_match_simple("h*", "world"));
    }

    // ─── eval simple commands ───

    #[test]
    fn test_eval_true() {
        let prog = program(vec![complete(pipeable(false, vec![sc(&["true"])]))]);
        assert_eq!(eval_program(&prog, &[], 0), 0);
    }

    #[test]
    fn test_eval_false() {
        let prog = program(vec![complete(pipeable(false, vec![sc(&["false"])]))]);
        assert_eq!(eval_program(&prog, &[], 0), 1);
    }

    #[test]
    fn test_eval_echo() {
        let prog = program(vec![complete(pipeable(false, vec![sc(&["echo", "hello"])]))]);
        assert_eq!(eval_program(&prog, &[], 0), 0);
    }

    #[test]
    fn test_eval_unknown_command() {
        let prog = program(vec![complete(pipeable(false, vec![sc(&["nonexistent_cmd_xyz"])]))]);
        assert_eq!(eval_program(&prog, &[], 0), 127);
    }

    // ─── eval AND/OR ───

    #[test]
    fn test_and_short_circuit() {
        // false && echo nope  →  echo should NOT run (exit code = 1 from false)
        // operator is on the node that follows &&
        let prog = program(vec![CompleteCommand {
            and_or: AndOrList {
                nodes: vec![
                    AndOrNode { command: pipeable(false, vec![sc(&["false"])]), operator: None },
                    AndOrNode { command: pipeable(false, vec![sc(&["echo", "nope"])]), operator: Some(AndOrOp::And) },
                ],
            },
            background: false,
        }]);
        assert_eq!(eval_program(&prog, &[], 0), 1);
    }

    #[test]
    fn test_or_short_circuit() {
        // true || echo nope  →  echo should NOT run (exit code = 0)
        let prog = program(vec![CompleteCommand {
            and_or: AndOrList {
                nodes: vec![
                    AndOrNode { command: pipeable(false, vec![sc(&["true"])]), operator: None },
                    AndOrNode { command: pipeable(false, vec![sc(&["echo", "nope"])]), operator: Some(AndOrOp::Or) },
                ],
            },
            background: false,
        }]);
        assert_eq!(eval_program(&prog, &[], 0), 0);
    }

    #[test]
    fn test_and_runs_when_true() {
        // true && echo ok  →  echo runs (exit code = 0)
        let prog = program(vec![CompleteCommand {
            and_or: AndOrList {
                nodes: vec![
                    AndOrNode { command: pipeable(false, vec![sc(&["true"])]), operator: None },
                    AndOrNode { command: pipeable(false, vec![sc(&["echo", "ok"])]), operator: Some(AndOrOp::And) },
                ],
            },
            background: false,
        }]);
        assert_eq!(eval_program(&prog, &[], 0), 0);
    }

    #[test]
    fn test_or_runs_when_false() {
        // false || echo ok  →  echo runs (exit code = 0)
        let prog = program(vec![CompleteCommand {
            and_or: AndOrList {
                nodes: vec![
                    AndOrNode { command: pipeable(false, vec![sc(&["false"])]), operator: None },
                    AndOrNode { command: pipeable(false, vec![sc(&["echo", "ok"])]), operator: Some(AndOrOp::Or) },
                ],
            },
            background: false,
        }]);
        assert_eq!(eval_program(&prog, &[], 0), 0);
    }

    // ─── eval pipelines ───

    #[test]
    fn test_pipeline_exit_code() {
        let prog = program(vec![complete(pipeable(false, vec![sc(&["false"])]))]);
        assert_eq!(eval_program(&prog, &[], 0), 1);
    }

    #[test]
    fn test_pipeline_exit_code_last() {
        // exit code comes from last command in pipeline
        let prog = program(vec![complete(pipeable(false, vec![sc(&["true"]), sc(&["false"])]))]);
        assert_eq!(eval_program(&prog, &[], 0), 1);
    }

    #[test]
    fn test_pipeline_negated() {
        let prog = program(vec![complete(pipeable(true, vec![sc(&["false"])]))]);
        // ! false → exit 0
        assert_eq!(eval_program(&prog, &[], 0), 0);
    }

    // ─── eval multiple commands ───

    #[test]
    fn test_semicolon_separator() {
        let prog = program(vec![
            complete(pipeable(false, vec![sc(&["true"])])),
            complete(pipeable(false, vec![sc(&["echo", "ok"])])),
        ]);
        assert_eq!(eval_program(&prog, &[], 0), 0);
    }

    // ─── eval background ───

    #[test]
    fn test_background() {
        let prog = Program { commands: vec![CompleteCommand {
            and_or: AndOrList { nodes: vec![AndOrNode { command: pipeable(false, vec![sc(&["true"])]), operator: None }] },
            background: true,
        }]};
        // background command should run and we don't wait — just check it doesn't crash
        let code = eval_program(&prog, &[], 0);
        assert!(code >= 0);
    }

    // ─── eval scripting: if ───

    #[test]
    fn test_eval_if_true() {
        let cond = program(vec![complete(pipeable(false, vec![sc(&["true"])]))]);
        let body = program(vec![complete(pipeable(false, vec![sc(&["echo", "yes"])]))]);
        let prog = program(vec![complete(compound(ScriptCommand::If(IfCommand {
            clauses: vec![IfClause { condition: cond, body }],
            else_body: None,
        })))]);
        assert_eq!(eval_program(&prog, &[], 0), 0);
    }

    #[test]
    fn test_eval_if_false() {
        let cond = program(vec![complete(pipeable(false, vec![sc(&["false"])]))]);
        let body = program(vec![complete(pipeable(false, vec![sc(&["echo", "yes"])]))]);
        let else_body = program(vec![complete(pipeable(false, vec![sc(&["echo", "no"])]))]);
        let prog = program(vec![complete(compound(ScriptCommand::If(IfCommand {
            clauses: vec![IfClause { condition: cond, body }],
            else_body: Some(else_body),
        })))]);
        assert_eq!(eval_program(&prog, &[], 0), 0);
    }

    // ─── eval scripting: for ───

    #[test]
    fn test_eval_for() {
        let body = program(vec![complete(pipeable(false, vec![sc(&["echo", "$i"])]))]);
        let prog = program(vec![complete(compound(ScriptCommand::For(ForCommand {
            var: "i".into(),
            words: vec!["a".into(), "b".into()],
            body,
        })))]);
        assert_eq!(eval_program(&prog, &[], 0), 0);
        unsafe { std::env::remove_var("i"); }
    }

    // ─── eval scripting: case ───

    #[test]
    fn test_eval_case_match() {
        let item_body = program(vec![complete(pipeable(false, vec![sc(&["echo", "matched"])]))]);
        let prog = program(vec![complete(compound(ScriptCommand::Case(CaseCommand {
            word: "x".into(),
            items: vec![
                CaseItem { patterns: vec!["x".into()], body: item_body },
            ],
        })))]);
        assert_eq!(eval_program(&prog, &[], 0), 0);
    }

    #[test]
    fn test_eval_case_no_match() {
        let item_body = program(vec![complete(pipeable(false, vec![sc(&["echo", "matched"])]))]);
        let prog = program(vec![complete(compound(ScriptCommand::Case(CaseCommand {
            word: "z".into(),
            items: vec![
                CaseItem { patterns: vec!["x".into()], body: item_body },
                CaseItem { patterns: vec!["y".into()], body: program(vec![]) },
            ],
        })))]);
        assert_eq!(eval_program(&prog, &[], 0), 0);
    }

    // ─── eval variable assignment ───

    #[test]
    fn test_pure_assignment_persists() {
        // v=hello  (pure assignment, no command)
        let prog = program(vec![complete(pipeable(false, vec![sc_env(&[("v", "hello")], &[])]))]);
        let prev = std::env::var("v").ok();
        eval_program(&prog, &[], 0);
        assert_eq!(std::env::var("v"), Ok("hello".to_string()));
        // Clean up
        match prev {
            Some(v) => unsafe { std::env::set_var("v", v); },
            None => unsafe { std::env::remove_var("v"); },
        }
    }

    #[test]
    fn test_temp_override_restored() {
        // v=hello echo x  (temp override, restored after)
        let prog = program(vec![complete(pipeable(false, vec![sc_env(&[("v", "hello")], &["echo", "x"])]))]);
        let prev = std::env::var("v").ok();
        eval_program(&prog, &[], 0);
        // After the command, v should be restored to its previous value
        assert_eq!(std::env::var("v").ok(), prev);
    }
}


