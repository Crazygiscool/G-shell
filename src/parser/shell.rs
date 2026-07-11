use rustyline::config::{BellStyle, CompletionType};
use rustyline::{Config, Editor};
use rustyline::history::{FileHistory, History};
use crate::parser::helper::ShellHelper;
use crate::parser::tokenize::tokenize;
use crate::parser::ast::{TokenKind, Program, CompleteCommand, CommandNode};
use crate::parser::{parser, eval};
use crate::commands::history::{history as history_cmd, HistoryAction};
use crate::parser::expand::expand_prompt;
use std::fs::{File, OpenOptions};
use crate::parser::pathcache;
use std::io::{BufWriter, Write};
use std::env;

pub struct Shell {
    rl: Editor<ShellHelper, FileHistory>,
    history_start_index: usize,
    history_file: String,
    pub last_exit_code: i32,
}

extern "C" fn sigint_handler(_sig: i32) {
    // Newline to let the user type again
    use std::io::Write;
    let _ = std::io::stderr().write_all(b"\n");
}

impl Shell {
    pub fn new() -> rustyline::Result<Self> {
        // Install SIGINT handler
        unsafe {
            let mut act: libc::sigaction = std::mem::zeroed();
            act.sa_sigaction = sigint_handler as *const () as usize;
            libc::sigaction(libc::SIGINT, &act, std::ptr::null_mut());
        }

        let config = Config::builder()
            .completion_type(CompletionType::List)
            .bell_style(BellStyle::Audible)
            .build();

        let history_file = env::var("HISTFILE").unwrap_or_else(|_| ".shell_history".to_string());

        let mut rl = Editor::<ShellHelper, FileHistory>::with_config(config)?;
        rl.set_helper(Some(ShellHelper));

        let _ = rl.load_history(&history_file);

        let history_start_index = rl.history().len();

        pathcache::refresh_cache();

        Ok(Shell { rl, history_start_index, history_file, last_exit_code: 0 })
    }

    pub fn run(&mut self) -> rustyline::Result<()> {
        self.source_rcfile();

        let result = self.run_loop();

        if let Err(e) = self.save_history_plain(&self.history_file.clone(), false) {
            eprintln!("Error saving history to {}: {}", self.history_file, e);
        }

        result
    }

    fn source_rcfile(&mut self) {
        let rcfile = env::var("GSHELLRC").unwrap_or_else(|_| {
            let cwd_path = ".gshellrc";
            if std::path::Path::new(cwd_path).exists() {
                return cwd_path.to_string();
            }
            env::var("HOME").map(|h| format!("{}/.gshellrc", h)).unwrap_or_default()
        });
        if rcfile.is_empty() {
            return;
        }
        let contents = match std::fs::read_to_string(&rcfile) {
            Ok(c) => c,
            Err(_) => return,
        };
        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let command = self.process_heredocs(trimmed);
            let tokens = tokenize(&command);
            if tokens.is_empty() { continue; }
            let program = parser::parse(&tokens);
            self.last_exit_code = eval::eval_program(&program, &[], self.last_exit_code);
        }
    }

    fn run_loop(&mut self) -> rustyline::Result<()> {
        loop {
            let prompt = expand_prompt(&env::var("PS1").unwrap_or_else(|_| "$ ".to_string()));
            let readline = self.rl.readline(&prompt);

            match readline {
                Ok(buffer) => {
                    let trimmed = buffer.trim();
                    if trimmed.is_empty() { continue; }

                    if trimmed == "exit" {
                        break;
                    }

                    let history_vec: Vec<String> = self.rl.history()
                        .iter()
                        .map(|s| s.to_string())
                        .collect();

                    let expanded = expand_history(trimmed, &history_vec);

                    // Collect multi-line input for scripting constructs
                    let full_input = self.collect_until_complete(&expanded);
                    let full_trimmed = full_input.trim();
                    if full_trimmed.is_empty() { continue; }

                    let history_line = full_trimmed.replace('\n', "; ");
                    let _ = self.rl.add_history_entry(&history_line);

                    let command = self.process_heredocs(full_trimmed);
                    let tokens = tokenize(&command);
                    if tokens.is_empty() { continue; }

                    // Check for history command with flags
                    if self.check_simple_history(&tokens).is_some() {
                        let args: Vec<String> = tokens.iter().skip(1).map(|t| t.value.clone()).collect();
                        match history_cmd(&history_vec, &args, &self.history_file) {
                            HistoryAction::Load(path) => {
                                if let Err(_) = self.rl.load_history(&path) {
                                    eprintln!("history: {}: No such file or directory", path);
                                }
                                self.history_start_index = self.rl.history().len();
                            }
                            HistoryAction::Write(path) => {
                                let _ = self.save_history_plain(&path, false);
                            }
                            HistoryAction::Append(path) => {
                                let _ = self.save_history_plain(&path, true);
                                self.history_start_index = self.rl.history().len();
                            }
                            HistoryAction::Clear => {
                                let _ = self.rl.clear_history();
                                self.history_start_index = 0;
                            }
                            HistoryAction::None => {}
                        }
                        continue;
                    }

                    let program = parser::parse(&tokens);

                    for complete_cmd in &program.commands {
                        if complete_cmd.background {
                            self.execute_background_ast(complete_cmd);
                        } else {
                            let single = Program { commands: vec![complete_cmd.clone()] };
                            self.last_exit_code = eval::eval_program(&single, &history_vec, self.last_exit_code);
                        }
                    }
                }
                Err(_) => break,
            }
        }
        Ok(())
    }

    fn collect_until_complete(&mut self, first: &str) -> String {
        let mut buffer = first.to_string();
        loop {
            let tokens = tokenize(&buffer);
            if input_is_complete(&tokens) {
                return buffer;
            }
            match self.rl.readline("> ") {
                Ok(line) => {
                    buffer.push('\n');
                    buffer.push_str(&line);
                }
                Err(_) => return buffer,
            }
        }
    }

    fn check_simple_history(&self, tokens: &[crate::parser::ast::Token]) -> Option<bool> {
        if tokens.len() >= 1
            && tokens[0].kind == TokenKind::Word
            && tokens[0].value == "history"
            && tokens.iter().all(|t| t.kind == TokenKind::Word)
        {
            Some(true)
        } else {
            None
        }
    }

    fn execute_background_ast(&mut self, cmd: &CompleteCommand) {
        use std::process::{Command, Stdio};
        use std::os::unix::process::CommandExt;

        if let Some(node) = cmd.and_or.nodes.first() {
            if let CommandNode::Pipeable(pipeline) = &node.command {
                if pipeline.commands.len() == 1 && !pipeline.negated {
                    let simple = &pipeline.commands[0];
                    if simple.words.is_empty() { return; }

                    let program = &simple.words[0];
                    let args: Vec<&str> = simple.words.iter().skip(1).map(|s| s.as_str()).collect();

                    if let Some(path) = pathcache::find_in_path_cache(program) {
                        match Command::new(&path)
                            .arg0(program)
                            .args(&args)
                            .stdin(Stdio::null())
                            .stdout(Stdio::inherit())
                            .stderr(Stdio::inherit())
                            .spawn()
                        {
                            Ok(child) => {
                                let pid = child.id();
                                eprintln!("[{}] {}", pid, program);
                                let _ = child;
                            }
                            Err(e) => {
                                eprintln!("{}: {}", program, e);
                            }
                        }
                    } else {
                        eprintln!("{}: command not found", program);
                    }
                }
            }
        }
    }

    fn process_heredocs(&mut self, command: &str) -> String {
        let tokens: Vec<String> = crate::parser::tokenize::tokenize_strings(command);
        let heredoc_pos = tokens.iter().position(|t| t == "<<" || t.ends_with("<<"));
        match heredoc_pos {
            None => command.to_string(),
            Some(pos) => {
                if pos + 1 >= tokens.len() {
                    return command.to_string();
                }
                let delimiter = tokens[pos + 1].clone();
                let mut lines = Vec::new();
                loop {
                    let readline = self.rl.readline("> ");
                    match readline {
                        Ok(line) => {
                            if line.trim() == delimiter {
                                break;
                            }
                            lines.push(line);
                        }
                        Err(_) => break,
                    }
                }
                let temp_dir = std::env::temp_dir();
                let temp_path = temp_dir.join(format!("gshell_heredoc_{}", std::process::id()));
                if let Ok(mut f) = std::fs::File::create(&temp_path) {
                    use std::io::Write;
                    for line in &lines {
                        let _ = writeln!(f, "{}", line);
                    }
                }
                let mut result = command.to_string();
                if let Some(heredoc_start) = result.find("<<") {
                    let after_op = &result[heredoc_start..];
                    if let Some(delim_end) = after_op.find(&delimiter) {
                        let before = &result[..heredoc_start];
                        let after = &result[heredoc_start + delim_end + delimiter.len()..];
                        result = format!("{}< {}", before.trim_end(), temp_path.to_string_lossy()) + after;
                    }
                }
                result
            }
        }
    }

    fn save_history_plain(&self, path: &str, append: bool) -> std::io::Result<()> {
        let file = if append {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?
        } else {
            File::create(path)?
        };

        let mut writer = BufWriter::new(file);

        let start = if append { self.history_start_index } else { 0 };

        for entry in self.rl.history().iter().skip(start) {
            writeln!(writer, "{}", entry)?;
        }
        writer.flush()?;
        Ok(())
    }
}

fn input_is_complete(tokens: &[crate::parser::ast::Token]) -> bool {
    let mut depth: i32 = 0;
    for token in tokens {
        if token.kind != crate::parser::ast::TokenKind::Word {
            continue;
        }
        match token.value.as_str() {
            "if" | "for" | "while" | "case" => depth += 1,
            "fi" | "done" | "esac" => depth -= 1,
            _ => {}
        }
    }
    depth <= 0
}

fn expand_history(input: &str, history: &[String]) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '!' {
            match chars.peek() {
                Some('!') => {
                    chars.next();
                    if let Some(last) = history.last() {
                        result.push_str(last);
                    }
                }
                Some('$') => {
                    chars.next();
                    if let Some(last) = history.last() {
                        if let Some(word) = last.split_whitespace().last() {
                            result.push_str(word);
                        }
                    }
                }
                Some('?') => {
                    chars.next();
                }
                Some(d) if d.is_ascii_digit() => {
                    let mut num_str = String::new();
                    while let Some(&n) = chars.peek() {
                        if n.is_ascii_digit() {
                            num_str.push(n);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    if let Ok(n) = num_str.parse::<usize>() {
                        if n > 0 && n <= history.len() {
                            result.push_str(&history[n - 1]);
                        }
                    }
                }
                _ => {
                    result.push('!');
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}
