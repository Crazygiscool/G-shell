use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper, Result};
use crate::parser::tab::{complete_command, complete_path};
use crate::parser::tokenize::tokenize;

#[derive(Clone, Copy)]
pub struct ShellHelper;

// In modern Rustyline, Helper is a blanket trait. 
// These empty impls are correct as long as the sub-traits are implemented.
impl Helper for ShellHelper {}

impl Hinter for ShellHelper {
    type Hint = String;
    fn hint(&self, _line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<Self::Hint> {
        None
    }
}

impl Highlighter for ShellHelper {}
impl Validator for ShellHelper {}

impl Completer for ShellHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Pair>)> {
        let buf = &line[..pos];
        let tokens = tokenize(buf);
        
        let last_word = if buf.ends_with(' ') || tokens.is_empty() {
            ""
        } else {
            tokens.last().map(|s| s.as_str()).unwrap_or("")
        };

        let start_pos = if last_word.is_empty() {
            pos
        } else {
            buf.rfind(last_word).unwrap_or(pos)
        };

        // 1. Get all matches
        let is_command = tokens.len() <= 1 && !buf.ends_with(' ');
        let matches = if is_command {
            complete_command(last_word)
        } else {
            complete_path(last_word)
        };

        // 2. Map matches to Pairs and handle trailing spaces
        let candidates: Vec<Pair> = matches
            .into_iter()
            .map(|m| {
                let mut replacement = m.clone();
                
                // UX logic for 2026:
                // - If it's a command, add a space.
                // - If it's a path and NOT a directory (doesn't end in /), add a space.
                // - If it's a directory (ends in /), don't add a space so they can keep typing.
                if is_command || !replacement.ends_with('/') {
                    replacement.push(' ');
                }

                Pair {
                    display: m, // Show the name without the trailing space in the list
                    replacement, // Push the name with the space into the buffer
                }
            })
            .collect();

        Ok((start_pos, candidates))
    }
}
