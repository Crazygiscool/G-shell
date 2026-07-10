use rustyline::config::{BellStyle, CompletionType};
use rustyline::{Config, Editor};
use rustyline::history::{FileHistory, History};
use crate::parser::helper::ShellHelper;
use crate::parser::{pipeline, tokenize::tokenize};
use crate::commands::history::{history as history_cmd, HistoryAction};
use crate::parser::process::process_command;
use std::fs::{File, OpenOptions};
use crate::parser::pathcache;
use std::os::unix::process::CommandExt;
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
            // Check CWD first, then $HOME
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
            let mut tokens = tokenize(&command);
            if tokens.is_empty() { continue; }
            let _cmd = tokens.remove(0);
            if command.contains("&&") || command.contains("||") {
                self.last_exit_code = execute_and_or_list(&command, &[], self.last_exit_code);
            } else if command.contains('|') {
                self.last_exit_code = pipeline::execute_pipeline(&command, &[], self.last_exit_code);
            } else {
                self.last_exit_code = process_command(&command, self.last_exit_code);
            }
        }
    }

    fn run_loop(&mut self) -> rustyline::Result<()> {
        loop {
            let prompt = env::var("PS1").unwrap_or_else(|_| "$ ".to_string());
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

                    let _ = self.rl.add_history_entry(&expanded);

                    let commands: Vec<&str> = expanded.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

                    for command in commands {
                        if command == "exit" {
                            return Ok(());
                        }

                        // Check for & (background)
                        let (cmd, bg) = if command.ends_with('&') {
                            let c = command[..command.len()-1].trim();
                            (c, true)
                        } else {
                            (command, false)
                        };

                        if bg {
                            self.execute_background(cmd, &history_vec);
                        } else {
                            self.execute_foreground(cmd, &history_vec);
                        }
                    }
                }
                Err(_) => break,
            }
        }
        Ok(())
    }

    fn execute_foreground(&mut self, command: &str, history_data: &[String]) {
        let command = self.process_heredocs(command);
        let mut tokens = tokenize(&command);
        if tokens.is_empty() { return; }
        let cmd = tokens.remove(0);

        match cmd.as_str() {
            "history" => {
                match history_cmd(history_data, &tokens, &self.history_file) {
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
            }
            _ => {
                if command.contains("&&") || command.contains("||") {
                    self.last_exit_code = execute_and_or_list(&command, history_data, self.last_exit_code);
                } else if command.contains('|') {
                    self.last_exit_code = pipeline::execute_pipeline(&command, history_data, self.last_exit_code);
                } else {
                    self.last_exit_code = process_command(&command, self.last_exit_code);
                }
            }
        }
    }

    fn execute_background(&mut self, command: &str, _history_data: &[String]) {
        use std::process::{Command, Stdio};

        let tokens = tokenize(command);
        if tokens.is_empty() { return; }
        let program = tokens[0].clone();
        let args: Vec<&str> = tokens.iter().skip(1).map(|s| s.as_str()).collect();

        if let Some(path) = pathcache::find_in_path_cache(&program) {
            match Command::new(&path)
                .arg0(&program)
                .args(&args)
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
            {
                Ok(child) => {
                    let pid = child.id();
                    println!("[{}] {}", pid, command);
                    let _ = child; // don't wait — let init reap it
                }
                Err(e) => {
                    eprintln!("{}: {}", program, e);
                }
            }
        } else {
            eprintln!("{}: command not found", program);
        }
    }

    fn process_heredocs(&mut self, command: &str) -> String {
        let tokens: Vec<String> = crate::parser::tokenize::tokenize(command);
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

fn execute_and_or_list(command: &str, history_data: &[String], last_exit_code: i32) -> i32 {
    let mut code = 0;
    let mut remaining = command;
    let mut expect_success = true;

    loop {
        let rest = remaining.trim();
        if rest.is_empty() { break; }

        if expect_success {
            if let Some(pos) = rest.find("&&") {
                let cmd = &rest[..pos].trim();
                code = execute_single(cmd, history_data, last_exit_code);
                remaining = &rest[pos + 2..];
                expect_success = code == 0;
            } else if let Some(pos) = rest.find("||") {
                let cmd = &rest[..pos].trim();
                code = execute_single(cmd, history_data, last_exit_code);
                remaining = &rest[pos + 2..];
                expect_success = code == 0;
            } else {
                code = execute_single(rest, history_data, last_exit_code);
                break;
            }
        } else {
            if let Some(pos) = rest.find("||") {
                let cmd = &rest[..pos].trim();
                code = execute_single(cmd, history_data, last_exit_code);
                remaining = &rest[pos + 2..];
                expect_success = code == 0;
            } else if let Some(pos) = rest.find("&&") {
                let cmd = &rest[..pos].trim();
                code = execute_single(cmd, history_data, last_exit_code);
                remaining = &rest[pos + 2..];
                expect_success = code == 0;
            } else {
                break;
            }
        }
    }

    code
}

fn execute_single(cmd: &str, history_data: &[String], last_exit_code: i32) -> i32 {
    if cmd.contains('|') {
        pipeline::execute_pipeline(cmd, history_data, last_exit_code)
    } else {
        process_command(cmd, last_exit_code)
    }
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
