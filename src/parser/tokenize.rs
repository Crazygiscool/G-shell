pub fn tokenize(input: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut cur = String::new();

        let mut chars = input.chars().peekable();
        let mut in_single = false;
        let mut in_double = false;

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
                            Some('"') => cur.push('"'),
                            Some('\\') => cur.push('\\'),
                            Some('$') => cur.push('$'),
                            Some('`') => cur.push('`'),
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
                    if !cur.is_empty() {
                        tokens.push(cur);
                        cur = String::new();
                    }
                }

                // FD REDIRECTION (e.g., 1>, 1>>, 2>, 2>>)
                c if !in_single && !in_double && c.is_ascii_digit() => {
                    let mut op = String::new();
                    op.push(c);
                    if let Some('>') = chars.peek().copied() {
                        chars.next();
                        op.push('>');
                        // check for >>
                        if let Some('>') = chars.peek().copied() {
                            chars.next();
                            op.push('>');
                        }
                    } else if let Some('<') = chars.peek().copied() {
                        chars.next();
                        op.push('<');
                        if let Some('<') = chars.peek().copied() {
                            chars.next();
                            op.push('<');
                        }
                    } else {
                        cur.push(c);
                        continue;
                    }
                    if !cur.is_empty() {
                        tokens.push(cur.clone());
                        cur.clear();
                    }
                    tokens.push(op);
                }

                // REDIRECTION OPERATORS: ">", ">>", "<"
                '>' if !in_single && !in_double => {
                    if !cur.is_empty() {
                        tokens.push(cur.clone());
                        cur.clear();
                    }
                    if let Some('>') = chars.peek().copied() {
                        chars.next();
                        tokens.push(">>".to_string());
                    } else {
                        tokens.push(">".to_string());
                    }
                }

                '<' if !in_single && !in_double => {
                    if !cur.is_empty() {
                        tokens.push(cur.clone());
                        cur.clear();
                    }
                    if let Some('<') = chars.peek() {
                        chars.next();
                        tokens.push("<<".to_string());
                    } else {
                        tokens.push("<".to_string());
                    }
                }

                // NORMAL CHARACTER
                _ => cur.push(c),
            }
        }

        if !cur.is_empty() {
            tokens.push(cur);
        }

        tokens
    }