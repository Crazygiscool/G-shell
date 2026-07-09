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

                    let _ = self.rl.add_history_entry(trimmed);

                    if trimmed == "exit" {
                        break;
                    }

                    let history_vec: Vec<String> = self.rl.history()
                        .iter()
                        .map(|s| s.to_string())
                        .collect();

                    let commands: Vec<&str> = trimmed.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

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
