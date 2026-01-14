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
        
        // 1. Identify the word fragment being completed
        // If buffer ends in space, we are starting a new argument (empty string)
        let last_word = if buf.ends_with(' ') || tokens.is_empty() {
            ""
        } else {
            tokens.last().map(|s| s.as_str()).unwrap_or("")
        };

        // 2. Determine the buffer index where the replacement should start
        let start_pos = if last_word.is_empty() {
            pos
        } else {
            // Find the last occurrence of the fragment to replace it
            buf.rfind(last_word).unwrap_or(pos)
        };

        // 3. Fetch all matches from your tab logic
        // If it's the first token (and no trailing space), it's a command
        let matches = if tokens.len() <= 1 && !buf.ends_with(' ') {
            complete_command(last_word)
        } else {
            // Otherwise, it's a file path
            complete_path(last_word)
        };

        // 4. Convert Strings to Rustyline Candidate Pairs
        let candidates: Vec<Pair> = matches
            .into_iter()
            .map(|m| Pair {
                display: m.clone(),
                replacement: m,
            })
            .collect();

        Ok((start_pos, candidates))
    }
}
