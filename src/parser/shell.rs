use rustyline::config::{BellStyle, CompletionType};
use rustyline::{Config, Editor};
use rustyline::history::{FileHistory, History};
use crate::parser::helper::ShellHelper;
use crate::parser::{pipeline, tokenize::tokenize};
use crate::commands::history::{history as history_cmd, HistoryAction};
use crate::parser::process::process_command;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::env;

pub struct Shell {
    rl: Editor<ShellHelper, FileHistory>,
    history_start_index: usize,
    history_file: String,
    last_exit_code: i32,
}

impl Shell {
    pub fn new() -> rustyline::Result<Self> {
        let config = Config::builder()
            .completion_type(CompletionType::List)
            .bell_style(BellStyle::Audible)
            .build();

        let history_file = env::var("HISTFILE").unwrap_or_else(|_| ".shell_history".to_string());

        let mut rl = Editor::<ShellHelper, FileHistory>::with_config(config)?;
        rl.set_helper(Some(ShellHelper));

        let _ = rl.load_history(&history_file);

        let history_start_index = rl.history().len();

        Ok(Shell { rl, history_start_index, history_file, last_exit_code: 0 })
    }

    pub fn run(&mut self) -> rustyline::Result<()> {
        let result = self.run_loop();

        if let Err(e) = self.save_history_plain(&self.history_file.clone(), false) {
            eprintln!("Error saving history to {}: {}", self.history_file, e);
        }

        result
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

                        let mut tokens = tokenize(command);
                        if tokens.is_empty() { continue; }
                        let cmd = tokens.remove(0);

                        if cmd == "history" {
                            match history_cmd(&history_vec, &tokens, &self.history_file) {
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
                        } else if command.contains("&&") || command.contains("||") {
                            self.last_exit_code = execute_and_or_list(command, &history_vec, self.last_exit_code);
                        } else if command.contains('|') {
                            self.last_exit_code = pipeline::execute_pipeline(command, &history_vec, self.last_exit_code);
                        } else {
                            self.last_exit_code = process_command(command, self.last_exit_code);
                        }
                    }
                }
                Err(_) => break,
            }
        }
        Ok(())
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
                    result.push_str(&last_exit_code_string());
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

fn last_exit_code_string() -> String {
    "0".to_string()
}
