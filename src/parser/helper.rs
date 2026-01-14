use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};

#[derive(Clone, Copy)]
pub struct ShellHelper;

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
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let mut buf = line[..pos].to_string();
        
        // Use the tokenize logic to find where the "last word" starts
        // This prevents the tab completion from overwriting the whole line
        let tokens = crate::parser::tokenize::tokenize(&buf);
        let last_word = tokens.last().map(|s| s.as_str()).unwrap_or("");
        
        // Calculate the starting position of the last word
        let start_pos = buf.rfind(last_word).unwrap_or(pos);

        // Run your tab completion logic
        crate::parser::tab::tab(&mut buf);

        if buf == line[..pos] {
            return Ok((pos, Vec::new()));
        }

        // The 'replacement' should now be the full completed word
        // found by your tab logic.
        let candidate = Pair {
            display: buf[start_pos..].to_string(),
            replacement: buf[start_pos..].to_string(),
        };

        // Return the start_pos so Rustyline only replaces the word, not the whole line
        Ok((start_pos, vec![candidate]))
    }
}
