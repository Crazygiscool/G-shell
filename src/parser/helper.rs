use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper, Result};
use crate::parser::tab::{complete_command, complete_path, complete_variable};
use crate::parser::tokenize::tokenize;

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
    ) -> Result<(usize, Vec<Pair>)> {
        let buf = &line[..pos];
        let tokens = tokenize(buf);

        let last_word = if buf.ends_with(' ') || tokens.is_empty() {
            ""
        } else {
            tokens.last().map(|t| t.value.as_str()).unwrap_or("")
        };

        let start_pos = if last_word.is_empty() {
            pos
        } else {
            buf.rfind(last_word).unwrap_or(pos)
        };

        let is_command = tokens.len() <= 1 && !buf.ends_with(' ');

        let matches: Vec<String> = if last_word.starts_with('$') {
            complete_variable(last_word)
        } else if is_command {
            complete_command(last_word)
        } else {
            complete_path(last_word)
        };

        let candidates: Vec<Pair> = matches
            .into_iter()
            .map(|m| {
                let mut replacement = m.clone();
                if is_command || !replacement.ends_with('/') {
                    replacement.push(' ');
                }
                Pair { display: m, replacement }
            })
            .collect();

        Ok((start_pos, candidates))
    }
}
