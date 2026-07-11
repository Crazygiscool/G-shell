use crate::parser::ast::{Token, TokenKind};

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut cur = String::new();

    let mut chars = input.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;

    let flush_word = |cur: &mut String, tokens: &mut Vec<Token>| {
        if !cur.is_empty() {
            tokens.push(Token::new(TokenKind::Word, std::mem::take(cur)));
        }
    };

    while let Some(c) = chars.next() {
        match c {
            // SINGLE QUOTES
            '\'' if !in_double => {
                in_single = !in_single;
            }

            // DOUBLE QUOTES
            '"' if !in_single => {
                in_double = !in_double;
            }

            // BACKSLASH HANDLING
            '\\' => {
                if in_single {
                    cur.push('\\');
                } else if in_double {
                    match chars.next() {
                        Some('"') => { cur.push('"'); }
                        Some('\\') => { cur.push('\\'); }
                        Some('$') => { cur.push('$'); }
                        Some('`') => { cur.push('`'); }
                        Some('\n') => { /* line continuation */ }
                        Some(ch) => {
                            cur.push('\\');
                            cur.push(ch);
                        }
                        None => cur.push('\\'),
                    }
                } else {
                    match chars.next() {
                        Some('\n') => { /* line continuation */ }
                        Some(ch) => cur.push(ch),
                        None => cur.push('\\'),
                    }
                }
            }

            // COMMENTS
            '#' if !in_single && !in_double && cur.is_empty() => {
                break;
            }

            // WHITESPACE SPLITTING
            c if c.is_whitespace() && !in_single && !in_double => {
                flush_word(&mut cur, &mut tokens);
            }

            // FD REDIRECTION (e.g., 1>, 1>>, 2>, 2>>, 0<)
            c if !in_single && !in_double && c.is_ascii_digit() => {
                let mut op = String::new();
                op.push(c);
                if let Some('>') = chars.peek().copied() {
                    chars.next();
                    op.push('>');
                    let kind = if chars.peek() == Some(&'>') {
                        chars.next();
                        op.push('>');
                        TokenKind::DGreat
                    } else if chars.peek() == Some(&'&') {
                        chars.next();
                        op.push('&');
                        TokenKind::Great
                    } else {
                        TokenKind::Great
                    };
                    flush_word(&mut cur, &mut tokens);
                    tokens.push(Token::new(kind, op));
                } else if let Some('<') = chars.peek().copied() {
                    chars.next();
                    op.push('<');
                    let kind = if chars.peek() == Some(&'<') {
                        chars.next();
                        op.push('<');
                        TokenKind::DLass
                    } else if chars.peek() == Some(&'&') {
                        chars.next();
                        op.push('&');
                        TokenKind::Less
                    } else {
                        TokenKind::Less
                    };
                    flush_word(&mut cur, &mut tokens);
                    tokens.push(Token::new(kind, op));
                } else {
                    cur.push(c);
                }
            }

            // REDIRECTION OPERATORS: ">", ">>"
            '>' if !in_single && !in_double => {
                flush_word(&mut cur, &mut tokens);
                if let Some('>') = chars.peek().copied() {
                    chars.next();
                    tokens.push(Token::new(TokenKind::DGreat, ">>"));
                } else if let Some('&') = chars.peek().copied() {
                    chars.next();
                    tokens.push(Token::new(TokenKind::Great, ">&"));
                } else {
                    tokens.push(Token::new(TokenKind::Great, ">"));
                }
            }

            // REDIRECTION OPERATORS: "<", "<<"
            '<' if !in_single && !in_double => {
                flush_word(&mut cur, &mut tokens);
                if let Some('<') = chars.peek() {
                    chars.next();
                    tokens.push(Token::new(TokenKind::DLass, "<<"));
                } else if let Some('&') = chars.peek().copied() {
                    chars.next();
                    tokens.push(Token::new(TokenKind::Less, "<&"));
                } else {
                    tokens.push(Token::new(TokenKind::Less, "<"));
                }
            }

            // CONTROL OPERATORS
            '|' if !in_single && !in_double => {
                flush_word(&mut cur, &mut tokens);
                if let Some('|') = chars.peek() {
                    chars.next();
                    tokens.push(Token::new(TokenKind::OrIf, "||"));
                } else {
                    tokens.push(Token::new(TokenKind::Pipe, "|"));
                }
            }

            '&' if !in_single && !in_double => {
                flush_word(&mut cur, &mut tokens);
                if let Some('&') = chars.peek() {
                    chars.next();
                    tokens.push(Token::new(TokenKind::AndIf, "&&"));
                } else {
                    tokens.push(Token::new(TokenKind::Background, "&"));
                }
            }

            ';' if !in_single && !in_double => {
                flush_word(&mut cur, &mut tokens);
                if let Some(';') = chars.peek() {
                    chars.next();
                    tokens.push(Token::new(TokenKind::DSemicolon, ";;"));
                } else {
                    tokens.push(Token::new(TokenKind::Semicolon, ";"));
                }
            }

            '!' if !in_single && !in_double => {
                flush_word(&mut cur, &mut tokens);
                tokens.push(Token::new(TokenKind::Bang, "!"));
            }

            '(' if !in_single && !in_double => {
                flush_word(&mut cur, &mut tokens);
                tokens.push(Token::new(TokenKind::LParen, "("));
            }

            ')' if !in_single && !in_double => {
                flush_word(&mut cur, &mut tokens);
                tokens.push(Token::new(TokenKind::RParen, ")"));
            }

            // NORMAL CHARACTER
            _ => cur.push(c),
        }
    }

    flush_word(&mut cur, &mut tokens);
    tokens
}

/// Compatibility shim: returns string values for code that hasn't migrated yet
pub fn tokenize_strings(input: &str) -> Vec<String> {
    tokenize(input).into_iter().map(|t| t.value).collect()
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Helper ───

    fn w(s: &str) -> Token { Token::new(TokenKind::Word, s) }

    // ─── Basic words ───

    #[test]
    fn test_empty() {
        assert_eq!(tokenize(""), vec![]);
        assert_eq!(tokenize("   "), vec![]);
    }

    #[test]
    fn test_single_word() {
        assert_eq!(tokenize("hello"), vec![w("hello")]);
    }

    #[test]
    fn test_multiple_words() {
        assert_eq!(tokenize("echo hello world"), vec![w("echo"), w("hello"), w("world")]);
    }

    #[test]
    fn test_leading_trailing_spaces() {
        assert_eq!(tokenize("  ls  -la "), vec![w("ls"), w("-la")]);
    }

    // ─── Quotes ───

    #[test]
    fn test_single_quotes() {
        assert_eq!(tokenize("echo 'hello world'"), vec![w("echo"), w("hello world")]);
    }

    #[test]
    fn test_double_quotes() {
        assert_eq!(tokenize("echo \"hello world\""), vec![w("echo"), w("hello world")]);
    }

    #[test]
    fn test_single_quote_inside_double() {
        assert_eq!(tokenize("echo \"it's ok\""), vec![w("echo"), w("it's ok")]);
    }

    #[test]
    fn test_mixed_quotes() {
        assert_eq!(tokenize("a='hello' b=\"world\""), vec![w("a=hello"), w("b=world")]);
    }

    // ─── Escapes ───

    #[test]
    fn test_backslash_escape() {
        assert_eq!(tokenize("echo\\ "), vec![w("echo ")]);
    }

    #[test]
    fn test_backslash_double_quote() {
        assert_eq!(tokenize("\"a\\\"b\""), vec![w("a\"b")]);
    }

    // ─── Control operators ───

    #[test]
    fn test_pipe() {
        assert_eq!(tokenize("a|b"), vec![w("a"), Token::new(TokenKind::Pipe, "|"), w("b")]);
    }

    #[test]
    fn test_or_if() {
        let toks = tokenize("a||b");
        assert_eq!(toks, vec![w("a"), Token::new(TokenKind::OrIf, "||"), w("b")]);
    }

    #[test]
    fn test_and_if() {
        assert_eq!(tokenize("a&&b"), vec![w("a"), Token::new(TokenKind::AndIf, "&&"), w("b")]);
    }

    #[test]
    fn test_semicolon() {
        assert_eq!(tokenize("a;b"), vec![w("a"), Token::new(TokenKind::Semicolon, ";"), w("b")]);
    }

    #[test]
    fn test_dsemicolon() {
        assert_eq!(tokenize("a;;b"), vec![w("a"), Token::new(TokenKind::DSemicolon, ";;"), w("b")]);
    }

    #[test]
    fn test_background() {
        assert_eq!(tokenize("a&"), vec![w("a"), Token::new(TokenKind::Background, "&")]);
    }

    #[test]
    fn test_bang() {
        assert_eq!(tokenize("! true"), vec![Token::new(TokenKind::Bang, "!"), w("true")]);
    }

    #[test]
    fn test_lparen_rparen() {
        assert_eq!(
            tokenize("(echo hi)"),
            vec![Token::new(TokenKind::LParen, "("), w("echo"), w("hi"), Token::new(TokenKind::RParen, ")")]
        );
    }

    // ─── Redirect operators ───

    #[test]
    fn test_redirect_output() {
        assert_eq!(tokenize("echo>f"), vec![w("echo"), Token::new(TokenKind::Great, ">"), w("f")]);
    }

    #[test]
    fn test_redirect_append() {
        assert_eq!(tokenize("echo>>f"), vec![w("echo"), Token::new(TokenKind::DGreat, ">>"), w("f")]);
    }

    #[test]
    fn test_redirect_input() {
        assert_eq!(tokenize("cat<f"), vec![w("cat"), Token::new(TokenKind::Less, "<"), w("f")]);
    }

    #[test]
    fn test_heredoc() {
        assert_eq!(tokenize("cat<<EOF"), vec![w("cat"), Token::new(TokenKind::DLass, "<<"), w("EOF")]);
    }

    #[test]
    fn test_fd_redirect_output() {
        assert_eq!(tokenize("echo 2>/dev/null"), vec![w("echo"), Token::new(TokenKind::Great, "2>"), w("/dev/null")]);
    }

    #[test]
    fn test_fd_redirect_append() {
        assert_eq!(tokenize("echo 1>>log"), vec![w("echo"), Token::new(TokenKind::DGreat, "1>>"), w("log")]);
    }

    #[test]
    fn test_fd_redirect_input() {
        assert_eq!(tokenize("cat 0<file"), vec![w("cat"), Token::new(TokenKind::Less, "0<"), w("file")]);
    }

    #[test]
    fn test_redirect_stderr() {
        assert_eq!(tokenize("echo 2>&1"), vec![w("echo"), Token::new(TokenKind::Great, "2>&"), w("1")]);
    }

    #[test]
    fn test_redirect_stdin_dupe() {
        assert_eq!(tokenize("cat 0<&1"), vec![w("cat"), Token::new(TokenKind::Less, "0<&"), w("1")]);
    }

    // ─── Comments ───

    #[test]
    fn test_comment() {
        assert_eq!(tokenize("echo hi # this is a comment"), vec![w("echo"), w("hi")]);
    }

    #[test]
    fn test_comment_only() {
        assert_eq!(tokenize("# comment"), vec![]);
    }

    // ─── Quoting control operators ───

    #[test]
    fn test_pipe_in_quotes() {
        assert_eq!(tokenize("\"a|b\""), vec![w("a|b")]);
        assert_eq!(tokenize("'a|b'"), vec![w("a|b")]);
    }

    #[test]
    fn test_redirect_in_quotes() {
        assert_eq!(tokenize("\"a>b\""), vec![w("a>b")]);
        assert_eq!(tokenize("'>'"), vec![w(">")]);
    }

    #[test]
    fn test_semicolon_in_quotes() {
        assert_eq!(tokenize("\"a;b\""), vec![w("a;b")]);
        assert_eq!(tokenize("';'"), vec![w(";")]);
    }

    // ─── Env assignment ───

    #[test]
    fn test_env_override() {
        assert_eq!(tokenize("FOO=bar echo"), vec![w("FOO=bar"), w("echo")]);
    }

    #[test]
    fn test_pure_assignment() {
        assert_eq!(tokenize("FOO=bar"), vec![w("FOO=bar")]);
    }

    // ─── Complex real-world input ───

    #[test]
    fn test_if_statement() {
        assert_eq!(
            tokenize("if true; then echo ok; fi"),
            vec![
                w("if"), w("true"), Token::new(TokenKind::Semicolon, ";"),
                w("then"), w("echo"), w("ok"), Token::new(TokenKind::Semicolon, ";"),
                w("fi"),
            ]
        );
    }

    #[test]
    fn test_case_statement() {
        assert_eq!(
            tokenize("case x in x) echo m;; esac"),
            vec![
                w("case"), w("x"), w("in"),
                w("x"), Token::new(TokenKind::RParen, ")"),
                w("echo"), w("m"), Token::new(TokenKind::DSemicolon, ";;"),
                w("esac"),
            ]
        );
    }

    #[test]
    fn test_for_loop() {
        assert_eq!(
            tokenize("for i in a b; do echo $i; done"),
            vec![
                w("for"), w("i"), w("in"), w("a"), w("b"),
                Token::new(TokenKind::Semicolon, ";"),
                w("do"), w("echo"), w("$i"), Token::new(TokenKind::Semicolon, ";"),
                w("done"),
            ]
        );
    }

    #[test]
    fn test_pipeline_and_or() {
        let toks = tokenize("a | b || c && d");
        assert_eq!(
            toks,
            vec![
                w("a"), Token::new(TokenKind::Pipe, "|"),
                w("b"), Token::new(TokenKind::OrIf, "||"),
                w("c"), Token::new(TokenKind::AndIf, "&&"),
                w("d"),
            ]
        );
    }

    #[test]
    fn test_background_command() {
        assert_eq!(
            tokenize("sleep 1 &"),
            vec![w("sleep"), w("1"), Token::new(TokenKind::Background, "&")]
        );
    }

    #[test]
    fn test_subshell() {
        assert_eq!(
            tokenize("(cd /tmp && pwd)"),
            vec![
                Token::new(TokenKind::LParen, "("),
                w("cd"), w("/tmp"), Token::new(TokenKind::AndIf, "&&"), w("pwd"),
                Token::new(TokenKind::RParen, ")"),
            ]
        );
    }
}
