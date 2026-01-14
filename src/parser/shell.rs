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
}

impl Shell {
    /// Initializes the shell, resolves HISTFILE, and loads existing history.
    pub fn new() -> rustyline::Result<Self> {
        let config = Config::builder()
            .completion_type(CompletionType::List)
            .bell_style(BellStyle::Audible)
            .build();
        
        // 1. Resolve HISTFILE: use env var or default to .shell_history
        let history_file = env::var("HISTFILE").unwrap_or_else(|_| ".shell_history".to_string());

        let mut rl = Editor::<ShellHelper, FileHistory>::with_config(config)?;
        rl.set_helper(Some(ShellHelper));
        
        // 2. STARTUP: Load history from the resolved path into memory
        let _ = rl.load_history(&history_file);
        
        // 3. Mark the baseline for this session to track 'new' commands
        let history_start_index = rl.history().len();
        
        Ok(Shell { rl, history_start_index, history_file })
    }

    /// Entry point for the shell. Handles startup, loop, and shutdown cleanup.
    pub fn run(&mut self) -> rustyline::Result<()> {
        // Execute the main REPL loop
        let result = self.run_loop();
        
        // 4. SHUTDOWN: Save the full session back to HISTFILE on exit
        // We use the custom plain saver to avoid Rustyline's #v2 metadata headers
        if let Err(e) = self.save_history_plain(&self.history_file.clone(), false) {
            eprintln!("Error saving history to {}: {}", self.history_file, e);
        }
        
        result
    }

fn run_loop(&mut self) -> rustyline::Result<()> {
        loop {
            let readline = self.rl.readline("$ ");
            
            match readline {
                Ok(buffer) => {
                    let trimmed = buffer.trim();
                    if trimmed.is_empty() { continue; }

                    // 1. CRITICAL FIX: Handle 'exit' explicitly to allow graceful shutdown
                    // This ensures the loop breaks and hits the save_history_plain call.
                    if trimmed == "exit" {
                        break;
                    }

                    // 2. Add current command to memory
                    let _ = self.rl.add_history_entry(trimmed);

                    // Sync to a Vec for listing
                    let history_vec: Vec<String> = self.rl.history()
                        .iter()
                        .map(|s| s.to_string())
                        .collect();

                    let mut tokens = tokenize(trimmed);
                    if tokens.is_empty() { continue; }
                    let command = tokens.remove(0);

                    // --- Routing ---
                    if command == "history" {
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
                    } else if trimmed.contains('|') {
                        pipeline::execute_pipeline(trimmed, &history_vec);
                    } else {
                        process_command(trimmed);
                    }
                }
                // Ctrl+D also triggers this break, which allows saving.
                Err(_) => break, 
            }
        }
        Ok(())
    }

    /// Writes history as raw text (one command per line) to match standard Bash behavior.
    /// This is safer for shell tests than Rustyline's default formatted save.
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
        
        // If appending, skip everything that was already in the file at session start
        let start = if append { self.history_start_index } else { 0 };

        for entry in self.rl.history().iter().skip(start) {
            writeln!(writer, "{}", entry)?;
        }
        writer.flush()?;
        Ok(())
    }
}