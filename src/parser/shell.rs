use rustyline::config::{BellStyle, CompletionType};
use rustyline::{Config, Editor};
use rustyline::history::FileHistory;
use crate::parser::helper::ShellHelper;
use crate::parser::{pipeline, tokenize::tokenize};
use crate::commands::history::{history, HistoryAction};
use crate::parser::process::process_command;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use rustyline::history::History;

pub struct Shell {
    rl: Editor<ShellHelper, FileHistory>,
    // 2026 update: track the start of the current session
    history_start_index: usize, 
}

impl Shell {
    pub fn new() -> rustyline::Result<Self> {
        let config = Config::builder()
            .completion_type(CompletionType::List)
            .bell_style(BellStyle::Audible)
            .build();
        
        let mut rl = Editor::<ShellHelper, FileHistory>::with_config(config)?;
        rl.set_helper(Some(ShellHelper));
        
        // Load initial history
        let _ = rl.load_history(".shell_history");
        
        // Set the starting point for "new" commands in this session
        let history_start_index = rl.history().len();
        
        Ok(Shell { rl, history_start_index })
    }

    pub fn run(&mut self) -> rustyline::Result<()> {
        loop {
            let readline = self.rl.readline("$ ");
            
            match readline {
                Ok(buffer) => {
                    let trimmed = buffer.trim();
                    if trimmed.is_empty() { continue; }

                    // Add to in-memory history (self-inclusion)
                    self.rl.add_history_entry(trimmed)?;

                    let history_vec: Vec<String> = self.rl.history()
                        .iter()
                        .map(|s| s.to_string())
                        .collect();

                    let mut tokens = tokenize(trimmed);
                    if tokens.is_empty() { continue; }
                    
                    let command = tokens.remove(0);

                    if trimmed.contains('|') {
                        pipeline::execute_pipeline(trimmed, &history_vec);
                    } else if command == "history" {
                        match history(&history_vec, &tokens) {
                            HistoryAction::Load(path) => {
                                if let Err(_) = self.rl.load_history(&path) {
                                    eprintln!("history: {}: No such file or directory", path);
                                }
                                // Standard shell behavior: loading a file 
                                // resets the "new" index to the new end of history
                                self.history_start_index = self.rl.history().len();
                            }
                            HistoryAction::Write(path) => {
                                let _ = self.save_history_plain(&path, false);
                            }
                            HistoryAction::Append(path) => {
                                // -a: Append ONLY the commands entered in this session
                                let _ = self.save_history_plain(&path, true);
                                // Move the index forward so the same commands 
                                // aren't appended twice if -a is called again
                                self.history_start_index = self.rl.history().len();
                            }
                            HistoryAction::None => {}
                        }
                    } else {
                        process_command(trimmed);
                    }
                }
                Err(_) => break, 
            }
        }
        let _ = self.save_history_plain(".shell_history", false);
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
        
        // If appending, skip everything before the current session's start index
        let start = if append { self.history_start_index } else { 0 };

        for entry in self.rl.history().iter().skip(start) {
            writeln!(writer, "{}", entry)?;
        }
        writer.flush()?;
        Ok(())
    }
}
