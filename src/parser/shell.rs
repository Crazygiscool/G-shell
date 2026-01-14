use rustyline::config::{BellStyle, CompletionType};
use rustyline::{Config, Editor};
use rustyline::history::{FileHistory, History}; // Ensure History trait is in scope for .len()
use crate::parser::helper::ShellHelper;
use crate::parser::{pipeline, tokenize::tokenize};
use crate::commands::history::{history, HistoryAction};
use crate::parser::process::process_command;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};

pub struct Shell {
    rl: Editor<ShellHelper, FileHistory>,
    // 2026 update: track the start of the current session or last append/load
    history_start_index: usize, 
}

impl Shell {
    pub fn new() -> rustyline::Result<Self> {
        let config = Config::builder()
            .completion_type(CompletionType::List)
            .bell_style(BellStyle::Audible)
            .build();
        
        // Editor initialization for 2026 (Rustyline v15+)
        let mut rl = Editor::<ShellHelper, FileHistory>::with_config(config)?;
        rl.set_helper(Some(ShellHelper));
        
        // Load default history (if exists)
        let _ = rl.load_history(".shell_history");
        
        // Mark the baseline for this session
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

                    // 1. Self-inclusion: Add command to history immediately
                    self.rl.add_history_entry(trimmed)?;

                    // 2. Prepare history vector for builtins (includes the current command)
                    let history_vec: Vec<String> = self.rl.history()
                        .iter()
                        .map(|s| s.to_string())
                        .collect();

                    let mut tokens = tokenize(trimmed);
                    if tokens.is_empty() { continue; }
                    
                    let command = tokens.remove(0);

                    // 3. Routing
                    if trimmed.contains('|') {
                        pipeline::execute_pipeline(trimmed, &history_vec);
                    } else if command == "history" {
                        match history(&history_vec, &tokens) {
                            HistoryAction::Load(path) => {
                                // Standard shell: -r appends file to memory
                                if let Err(_) = self.rl.load_history(&path) {
                                    eprintln!("history: {}: No such file or directory", path);
                                }
                                // Reset checkpoint so existing lines aren't re-appended later
                                self.history_start_index = self.rl.history().len();
                            }
                            HistoryAction::Write(path) => {
                                // -w: Overwrite file with current full in-memory list
                                let _ = self.save_history_plain(&path, false);
                            }
                            HistoryAction::Append(path) => {
                                // -a: Append ONLY commands added since the last checkpoint
                                let _ = self.save_history_plain(&path, true);
                                // Move checkpoint forward to the current end of history
                                self.history_start_index = self.rl.history().len();
                            }
                            HistoryAction::None => {}
                        }
                    } else {
                        process_command(trimmed);
                    }
                }
                Err(_) => break, // Exit on Ctrl+C or Ctrl+D
            }
        }
        // Final save on exit (overwrite default file)
        let _ = self.save_history_plain(".shell_history", false);
        Ok(())
    }

    /// Custom plain-text saver to avoid #v2 metadata headers
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
        
        // Use history_start_index if appending to avoid duplicates
        let start = if append { self.history_start_index } else { 0 };

        for entry in self.rl.history().iter().skip(start) {
            writeln!(writer, "{}", entry)?;
        }
        writer.flush()?;
        Ok(())
    }
}
