use std::sync::{Arc, Mutex};
use std::io::{self, Write, Read};
use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper, Result, Cmd, Movement, RepeatCount,
    ConditionalEventHandler, Event, EventContext};
use crate::parser::tab::{complete_command, complete_path, complete_variable, CompletionCandidate};
use crate::parser::ast::TokenKind;
use crate::parser::tokenize::tokenize;

const LISTMAX: usize = 100;

pub struct ShellHelper {
    #[allow(dead_code)]
    pub cycling: Arc<Mutex<Option<CyclingState>>>,
}

pub struct CyclingState {
    pub candidates: Vec<CompletionCandidate>,
    pub selected: usize,
    pub start_pos: usize,
    #[allow(dead_code)]
    pub original_line: String,
    #[allow(dead_code)]
    pub original_pos: usize,
}

impl ShellHelper {
    pub fn new(cycling: Arc<Mutex<Option<CyclingState>>>) -> Self {
        ShellHelper {
            cycling,
        }
    }
}

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

        let is_command = is_at_command_position(&tokens, buf.ends_with(' '));

        let candidates: Vec<CompletionCandidate> = if last_word.starts_with('$') {
            complete_variable(last_word)
        } else if is_command {
            complete_command(last_word)
        } else {
            complete_path(last_word)
        };

        let pairs: Vec<Pair> = candidates
            .into_iter()
            .map(|c| Pair {
                display: c.display,
                replacement: c.replacement,
            })
            .collect();

        Ok((start_pos, pairs))
    }
}

pub struct TabHandler {
    pub cycling: Arc<Mutex<Option<CyclingState>>>,
}

impl ConditionalEventHandler for TabHandler {
    fn handle(&self, _evt: &Event, _n: RepeatCount, _positive: bool, ctx: &EventContext) -> Option<Cmd> {
        let line = ctx.line();
        let pos = ctx.pos();

        {
            let mut state = self.cycling.lock().unwrap();

            if let Some(ref mut cycling) = *state {
                if cycling.start_pos <= pos && pos <= line.len() {
                    cycling.selected = (cycling.selected + 1) % cycling.candidates.len();
                    let candidate = &cycling.candidates[cycling.selected];
                    let new_line = format!(
                        "{}{}{}",
                        &line[..cycling.start_pos],
                        candidate.replacement,
                        &line[pos..]
                    );
                    return Some(Cmd::Replace(Movement::WholeLine, Some(new_line)));
                }
                *state = None;
            }
        }

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
        let is_command = is_at_command_position(&tokens, buf.ends_with(' '));

        let candidates: Vec<CompletionCandidate> = if last_word.starts_with('$') {
            complete_variable(last_word)
        } else if is_command {
            complete_command(last_word)
        } else {
            complete_path(last_word)
        };

        let current_len = pos - start_pos;

        if candidates.len() > LISTMAX {
            eprint!("zsh: do you wish to see all {} possibilities? (y or n) ", candidates.len());
            io::stderr().flush().ok();
            let mut byte = [0u8; 1];
            if io::stdin().read(&mut byte).is_ok() && byte[0] == b'y' {
                eprintln!();
                return None;
            }
            eprintln!();
            return Some(Cmd::Noop);
        }

        if candidates.len() == 1 {
            let replacement = &candidates[0].replacement;
            if replacement.len() > current_len {
                let new_line = format!("{}{}{}", &line[..start_pos], replacement, &line[pos..]);
                return Some(Cmd::Replace(Movement::WholeLine, Some(new_line)));
            }
            return None;
        }

        if candidates.len() > 1 {
            let replacements: Vec<&str> = candidates.iter().map(|c| c.replacement.as_str()).collect();
            let lcp = longest_common_prefix(&replacements);
            if lcp.len() > current_len {
                let mut state = self.cycling.lock().unwrap();
                *state = Some(CyclingState {
                    candidates,
                    selected: 0,
                    start_pos,
                    original_line: line.to_string(),
                    original_pos: pos,
                });
                let new_line = format!("{}{}{}", &line[..start_pos], lcp, &line[pos..]);
                return Some(Cmd::Replace(Movement::WholeLine, Some(new_line)));
            }
        }

        None
    }
}

fn is_at_command_position(tokens: &[crate::parser::ast::Token], buf_ends_with_space: bool) -> bool {
    if tokens.is_empty() {
        return true;
    }
    let current_idx = if buf_ends_with_space {
        tokens.len()
    } else {
        tokens.len() - 1
    };
    if current_idx == 0 {
        return true;
    }
    matches!(
        tokens[current_idx - 1].kind,
        TokenKind::Pipe | TokenKind::AndIf | TokenKind::OrIf
            | TokenKind::Semicolon | TokenKind::DSemicolon
            | TokenKind::Background | TokenKind::Bang | TokenKind::LParen
    )
}

fn longest_common_prefix(strs: &[&str]) -> String {
    if strs.is_empty() {
        return String::new();
    }
    let mut prefix = strs[0].to_string();
    for s in &strs[1..] {
        while !s.starts_with(&prefix) {
            prefix.pop();
            if prefix.is_empty() {
                return String::new();
            }
        }
    }
    prefix
}
