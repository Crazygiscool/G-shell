use rustyline::config::{BellStyle, CompletionType};
use rustyline::{Config, Editor};
use rustyline::history::FileHistory; // Use FileHistory instead of DefaultHistory
use crate::parser::helper::ShellHelper;
use crate::parser::{pipeline, tokenize::tokenize};
use crate::commands::history::{history, HistoryAction};
use crate::parser::process::process_command;

pub struct Shell {
    // FIX: Specify FileHistory as the second generic
    rl: Editor<ShellHelper, FileHistory>,
}

impl Shell {
    pub fn new() -> rustyline::Result<Self> {
        let config = Config::builder()
            .completion_type(CompletionType::List)
            .bell_style(BellStyle::Audible)
            .build();
        
        // FIX: with_config still takes only the config object.
        // The generics on Editor determine the types.
        let mut rl = Editor::<ShellHelper, FileHistory>::with_config(config)?;
        rl.set_helper(Some(ShellHelper));
        
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

                    // FIX: In 2026, add_history_entry returns Result<bool>.
                    // Use ? to propagate the error or handle it.
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
                                if let Err(e) = self.rl.save_history(&path) {
                                    eprintln!("history: failed to write {}: {}", path, e);
                                }
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
        let _ = self.rl.save_history(".shell_history");
        Ok(())
    }
}
