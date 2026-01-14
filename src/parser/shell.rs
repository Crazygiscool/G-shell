use rustyline::config::{BellStyle, CompletionType};
use rustyline::{Config, Editor};
use crate::parser::helper::ShellHelper;
use crate::parser::{pipeline, tokenize::tokenize};
use crate::commands::history::{history, HistoryAction}; // Added HistoryAction
use crate::parser::process::process_command;
use std::fs;

pub struct Shell {
    rl: Editor<ShellHelper>,
}

impl Shell {
    pub fn new() -> rustyline::Result<Self> {
        let config = Config::builder()
            .completion_type(CompletionType::List)
            .bell_style(BellStyle::Audible)
            .build();
        
        let mut rl = Editor::<ShellHelper>::with_config(config)?;
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

                    // 1. Add current command to history
                    let _ = self.rl.add_history_entry(trimmed);

                    // 2. Prepare history vector for modules
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
                        // Capture the action (None or Load)
                        let action = history(&history_vec, &tokens);

                        // Handle 'history -r' request
                        if let HistoryAction::Load(path) = action {
                            match fs::read_to_string(&path) {
                                Ok(content) => {
                                    for line in content.lines() {
                                        let line_trimmed = line.trim();
                                        if !line_trimmed.is_empty() {
                                            // Append file history to current session
                                            let _ = self.rl.add_history_entry(line_trimmed);
                                        }
                                    }
                                }
                                Err(_) => eprintln!("history: {}: No such file or directory", path),
                            }
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