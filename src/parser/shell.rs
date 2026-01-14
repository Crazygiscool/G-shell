use rustyline::config::{BellStyle, CompletionType};
use rustyline::{Config, Editor};
use rustyline::history::FileHistory;
use crate::parser::helper::ShellHelper;
use crate::parser::{pipeline, tokenize::tokenize};
use crate::commands::history::{history, HistoryAction};
use crate::parser::process::process_command;
use std::fs::File;
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
        
        // Rustyline can still load plain text files with load_history
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

                    // Add to in-memory history
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
                            }
                            HistoryAction::Write(path) => {
                                // Manual write for "history -w" to avoid #v2
                                let _ = self.save_history_plain(&path);
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
        // Save the default history file as plain text on exit
        let _ = self.save_history_plain(".shell_history");
        Ok(())
    }

    /// Helper to save history as plain text without the #v2 metadata header
    fn save_history_plain(&self, path: &str) -> std::io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        for entry in self.rl.history().iter() {
            writeln!(writer, "{}", entry)?;
        }
        writer.flush()?;
        Ok(())
    }
}
