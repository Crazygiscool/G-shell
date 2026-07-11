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
                    let kind = if chars.peek() == Some(&'>') {
                        chars.next();
                        op.push_str(">>");
                        TokenKind::DGreat
                    } else {
                        op.push('>');
                        TokenKind::Great
                    };
                    flush_word(&mut cur, &mut tokens);
                    tokens.push(Token::new(kind, op));
                } else if let Some('<') = chars.peek().copied() {
                    chars.next();
                    let kind = if chars.peek() == Some(&'<') {
                        chars.next();
                        op.push_str("<<");
                        TokenKind::DLass
                    } else {
                        op.push('<');
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
