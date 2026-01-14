use rustyline::config::{BellStyle, CompletionType};
use rustyline::{Config, Editor};
use rustyline::history::FileHistory;
use crate::parser::helper::ShellHelper;
use crate::parser::{pipeline, tokenize::tokenize};
use crate::commands::history::{history, HistoryAction};
use crate::parser::process::process_command;
use std::fs::{File, OpenOptions}; // Added OpenOptions
use std::io::{BufWriter, Write};

pub struct Shell {
    rl: Editor<ShellHelper, FileHistory>,
}

impl Shell {
    pub fn new() -> rustyline::Result<Self> {
        let config = Config::builder()
            .completion_type(CompletionType::List)
            .bell_style(BellStyle::Audible)
            .build();
        
        let mut rl = Editor::<ShellHelper, FileHistory>::with_config(config)?;
        rl.set_helper(Some(ShellHelper));
        
        // Load history from file into memory
        let _ = rl.load_history(".shell_history");
        
        Ok(Shell { rl })
    }

    pub fn run(&mut self) -> rustyline::Result<()> {
        loop {
            let readline = self.rl.readline("$ ");
            
            match readline {
                Ok(buffer) => {
                    let trimmed = buffer.trim();
                    if trimmed.is_empty() { continue; }

                    // Add to in-memory history (returns Result<bool> in 2026)
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
                                // -r: Append file contents to in-memory list
                                if let Err(_) = self.rl.load_history(&path) {
                                    eprintln!("history: {}: No such file or directory", path);
                                }
                            }
                            HistoryAction::Write(path) => {
                                // -w: Overwrite file with current in-memory list (plain text)
                                let _ = self.save_history_plain(&path, false);
                            }
                            HistoryAction::Append(path) => {
                                // -a: Append current in-memory list to file (plain text)
                                let _ = self.save_history_plain(&path, true);
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
        // Save default history on exit (overwrite to keep it fresh)
        let _ = self.save_history_plain(".shell_history", false);
        Ok(())
    }

    /// Helper to save history as plain text without #v2 metadata
    /// set 'append' to true for 'history -a', false for 'history -w'
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
        for entry in self.rl.history().iter() {
            writeln!(writer, "{}", entry)?;
        }
        writer.flush()?;
        Ok(())
    }
}
