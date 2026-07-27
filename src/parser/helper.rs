use std::sync::{Arc, Mutex};
use std::io::{self, Write, Read};
use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper, Result, Cmd, RepeatCount,
    ConditionalEventHandler, Event, EventContext};
use crate::parser::tab::{complete_command, complete_path, complete_variable, CompletionCandidate};
use crate::parser::ast::TokenKind;
use crate::parser::tokenize::tokenize;

const LISTMAX: usize = 100;

pub struct ShellHelper {
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

        {
            let mut state = self.cycling.lock().unwrap();
            if let Some(ref cycling) = *state {
                if cycling.selected > 0 && cycling.start_pos == start_pos {
                    let candidate = &cycling.candidates[cycling.selected];
                    return Ok((cycling.start_pos, vec![Pair {
                        display: candidate.display.clone(),
                        replacement: candidate.replacement.clone(),
                    }]));
                }
                *state = None;
            }
        }

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

        {
            let mut state = self.cycling.lock().unwrap();

            if let Some(ref mut cycling) = *state {
                if cycling.start_pos == start_pos {
                    cycling.selected = (cycling.selected + 1) % cycling.candidates.len();
                    if cycling.selected == 0 && cycling.candidates.len() > 1 {
                        cycling.selected = 1;
                    }
                    return None;
                }
                *state = None;
            }

            let is_command = is_at_command_position(&tokens, buf.ends_with(' '));

            let candidates: Vec<CompletionCandidate> = if last_word.starts_with('$') {
                complete_variable(last_word)
            } else if is_command {
                complete_command(last_word)
            } else {
                complete_path(last_word)
            };

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

            if candidates.len() > 1 {
                *state = Some(CyclingState {
                    candidates,
                    selected: 0,
                    start_pos,
                    original_line: line.to_string(),
                    original_pos: pos,
                });
            } else {
                *state = None;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::TokenKind;
    use crate::parser::tokenize::tokenize;

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

    // ── is_at_command_position tests ──

    #[test]
    fn test_is_command_empty_tokens() {
        assert!(is_at_command_position(&[], false));
    }

    #[test]
    fn test_is_command_first_word() {
        let tokens = tokenize("ls");
        assert!(is_at_command_position(&tokens, false));
    }

    #[test]
    fn test_is_command_after_word_with_space() {
        let tokens = tokenize("cd");
        assert!(!is_at_command_position(&tokens, true));
    }

    #[test]
    fn test_is_command_after_pipe() {
        let tokens = tokenize("echo |");
        assert!(is_at_command_position(&tokens, true));
    }

    #[test]
    fn test_is_command_after_pipe_word() {
        let tokens = tokenize("echo | c");
        assert!(is_at_command_position(&tokens, false));
    }

    #[test]
    fn test_is_command_after_and_if() {
        let tokens = tokenize("echo &&");
        assert!(is_at_command_position(&tokens, true));
    }

    #[test]
    fn test_is_command_after_and_if_word() {
        let tokens = tokenize("echo && ls");
        assert!(is_at_command_position(&tokens, false));
    }

    #[test]
    fn test_is_command_after_or_if() {
        let tokens = tokenize("echo ||");
        assert!(is_at_command_position(&tokens, true));
    }

    #[test]
    fn test_is_command_after_or_if_word() {
        let tokens = tokenize("echo || cat");
        assert!(is_at_command_position(&tokens, false));
    }

    #[test]
    fn test_is_command_after_semicolon() {
        let tokens = tokenize("echo;");
        assert!(is_at_command_position(&tokens, true));
    }

    #[test]
    fn test_is_command_after_semicolon_word() {
        let tokens = tokenize("echo; ls");
        assert!(is_at_command_position(&tokens, false));
    }

    #[test]
    fn test_is_command_after_background() {
        let tokens = tokenize("echo &");
        assert!(is_at_command_position(&tokens, true));
    }

    #[test]
    fn test_is_command_after_background_word() {
        let tokens = tokenize("echo & sleep");
        assert!(is_at_command_position(&tokens, false));
    }

    #[test]
    fn test_is_command_after_bang() {
        let tokens = tokenize("!");
        assert!(is_at_command_position(&tokens, true));
    }

    #[test]
    fn test_is_command_after_lparen() {
        let tokens = tokenize("(");
        assert!(is_at_command_position(&tokens, true));
    }

    #[test]
    fn test_is_command_not_after_word() {
        let tokens = tokenize("echo hello");
        assert!(!is_at_command_position(&tokens, false));
    }

    #[test]
    fn test_is_command_not_after_redirect() {
        let tokens = tokenize("echo >");
        assert!(!is_at_command_position(&tokens, false));
    }

    #[test]
    fn test_is_command_chain() {
        let tokens = tokenize("a | b && c || d ; e");
        assert!(is_at_command_position(&tokens, false));
    }

    // ── longest_common_prefix tests ──

    #[test]
    fn test_lcp_empty() {
        assert_eq!(longest_common_prefix(&[]), "");
    }

    #[test]
    fn test_lcp_single() {
        assert_eq!(longest_common_prefix(&["hello"]), "hello");
    }

    #[test]
    fn test_lcp_common() {
        assert_eq!(longest_common_prefix(&["hello", "help", "heist"]), "he");
    }

    #[test]
    fn test_lcp_all_same() {
        assert_eq!(longest_common_prefix(&["abc", "abc", "abc"]), "abc");
    }

    #[test]
    fn test_lcp_no_common() {
        assert_eq!(longest_common_prefix(&["abc", "def", "ghi"]), "");
    }

    #[test]
    fn test_lcp_one_prefix_of_other() {
        assert_eq!(longest_common_prefix(&["abc", "abcdef"]), "abc");
    }

    #[test]
    fn test_lcp_paths() {
        assert_eq!(
            longest_common_prefix(&["src/commands/", "src/parser/", "src/main.rs"]),
            "src/"
        );
    }

    #[test]
    fn test_lcp_empty_strings() {
        assert_eq!(longest_common_prefix(&["", "", ""]), "");
    }

    #[test]
    fn test_lcp_mixed_empty() {
        assert_eq!(longest_common_prefix(&["abc", "", "def"]), "");
    }

    // ── tokenize integration tests ──

    #[test]
    fn test_tokenize_cd_a() {
        let tokens = tokenize("cd a");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::Word);
        assert_eq!(tokens[0].value, "cd");
        assert_eq!(tokens[1].kind, TokenKind::Word);
        assert_eq!(tokens[1].value, "a");
    }

    #[test]
    fn test_tokenize_pipe_command() {
        let tokens = tokenize("echo | cat");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[1].kind, TokenKind::Pipe);
    }

    #[test]
    fn test_tokenize_semicolon_command() {
        let tokens = tokenize("echo; ls");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[1].kind, TokenKind::Semicolon);
    }

    #[test]
    fn test_tokenize_and_if_command() {
        let tokens = tokenize("echo && ls");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[1].kind, TokenKind::AndIf);
    }

    #[test]
    fn test_tokenize_or_if_command() {
        let tokens = tokenize("echo || ls");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[1].kind, TokenKind::OrIf);
    }
}
