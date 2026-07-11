use crate::parser::ast::*;

pub struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&'a Token> {
        let token = self.tokens.get(self.pos);
        self.pos += 1;
        token
    }

    fn expect(&mut self, kind: TokenKind) -> Option<&'a Token> {
        let token = self.peek()?;
        if token.kind == kind {
            self.advance()
        } else {
            None
        }
    }

    fn skip_semicolons(&mut self) {
        while self.peek().is_some_and(|t| t.kind == TokenKind::Semicolon) {
            self.advance();
        }
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.peek().is_some_and(|t| t.kind == kind)
    }

    // ── Grammar rules ──

    pub fn parse_program(&mut self) -> Program {
        let mut commands = Vec::new();
        loop {
            self.skip_semicolons();
            if self.peek().is_none() {
                break;
            }
            commands.push(self.parse_complete_command());
            self.skip_semicolons();
        }
        Program { commands }
    }

    fn parse_complete_command(&mut self) -> CompleteCommand {
        let and_or = self.parse_and_or();
        let background = self.expect(TokenKind::Background).is_some();
        CompleteCommand { and_or, background }
    }

    fn parse_and_or(&mut self) -> AndOrList {
        let mut nodes = Vec::new();
        nodes.push(AndOrNode {
            command: self.parse_command_node(),
            operator: None,
        });
        loop {
            let op = if self.check(TokenKind::AndIf) {
                self.advance();
                AndOrOp::And
            } else if self.check(TokenKind::OrIf) {
                self.advance();
                AndOrOp::Or
            } else {
                break;
            };
            nodes.push(AndOrNode {
                command: self.parse_command_node(),
                operator: Some(op),
            });
        }
        AndOrList { nodes }
    }

    fn parse_command_node(&mut self) -> CommandNode {
        // Check for subshell: LParen ... RParen
        if self.check(TokenKind::LParen) {
            return CommandNode::Compound(self.parse_subshell());
        }

        // Check for scripting keywords
        if let Some(token) = self.peek() {
            if token.kind == TokenKind::Word {
                match token.value.as_str() {
                    "if" => return CommandNode::Compound(ScriptCommand::If(self.parse_if())),
                    "for" => return CommandNode::Compound(ScriptCommand::For(self.parse_for())),
                    "while" => return CommandNode::Compound(ScriptCommand::While(self.parse_while())),
                    "case" => return CommandNode::Compound(ScriptCommand::Case(self.parse_case())),
                    "function" => return CommandNode::Compound(ScriptCommand::Function(self.parse_function())),
                    _ => {}
                }
            }
        }

        CommandNode::Pipeable(self.parse_pipeline())
    }

    fn parse_pipeline(&mut self) -> Pipeline {
        let negated = self.expect(TokenKind::Bang).is_some();
        let mut commands = Vec::new();
        commands.push(self.parse_simple_command());
        while self.check(TokenKind::Pipe) {
            self.advance();
            commands.push(self.parse_simple_command());
        }
        Pipeline { negated, commands }
    }

    fn parse_simple_command(&mut self) -> SimpleCommand {
        let mut env_overrides = Vec::new();
        let mut words = Vec::new();
        let mut redirects = Vec::new();

        // Collect leading env overrides: FOO=bar
        while let Some(token) = self.peek() {
            if token.kind != TokenKind::Word {
                break;
            }
            if let Some(eq_pos) = token.value.find('=') {
                if eq_pos > 0 && token.value[..eq_pos].chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    let name = token.value[..eq_pos].to_string();
                    let value = token.value[eq_pos + 1..].to_string();
                    env_overrides.push((name, value));
                    self.advance();
                    continue;
                }
            }
            break;
        }

        // Collect command word, args, and redirects
        loop {
            if self.check(TokenKind::Word) {
                let token = self.advance().unwrap();
                words.push(token.value.clone());
            } else if self.check_redirect() {
                let (fd, kind, target) = self.parse_redirect();
                redirects.push(Redirect { fd, kind, target });
            } else {
                break;
            }
        }

        SimpleCommand { env_overrides, words, redirects }
    }

    fn check_redirect(&self) -> bool {
        matches!(
            self.peek().map(|t| t.kind),
            Some(TokenKind::Great | TokenKind::DGreat | TokenKind::Less | TokenKind::DLass)
        )
    }

    fn parse_redirect(&mut self) -> (i32, RedirectKind, String) {
        let token = self.advance().unwrap();
        let fd = parse_fd_from_value(&token.value, token.kind);
        let kind = match token.kind {
            TokenKind::Great => RedirectKind::Output,
            TokenKind::DGreat => RedirectKind::Append,
            TokenKind::Less => RedirectKind::Input,
            TokenKind::DLass => RedirectKind::Heredoc,
            _ => unreachable!(),
        };
        let target = self.expect(TokenKind::Word)
            .map(|t| t.value.clone())
            .unwrap_or_default();
        (fd, kind, target)
    }

    // ── Scripting construct parsers ──

    fn parse_subshell(&mut self) -> ScriptCommand {
        self.advance(); // consume '('
        let program = self.parse_program();
        self.expect(TokenKind::RParen)
            .unwrap_or_else(|| panic!("Expected )"));
        ScriptCommand::Subshell(program)
    }

    fn parse_if(&mut self) -> IfCommand {
        // We've already consumed "if"
        let condition = self.parse_compound_list();
        self.expect_word("then");
        let body = self.parse_compound_list();

        let mut clauses = vec![IfClause { condition, body }];

        loop {
            if self.check_word("elif") {
                self.advance();
                let cond = self.parse_compound_list();
                self.expect_word("then");
                let b = self.parse_compound_list();
                clauses.push(IfClause { condition: cond, body: b });
            } else {
                break;
            }
        }

        let else_body = if self.check_word("else") {
            self.advance();
            Some(self.parse_compound_list())
        } else {
            None
        };

        self.expect_word("fi");
        IfCommand { clauses, else_body }
    }

    fn parse_for(&mut self) -> ForCommand {
        // We've already consumed "for"
        let var = self.expect(TokenKind::Word)
            .map(|t| t.value.clone())
            .unwrap_or_default();

        let words = if self.check_word("in") {
            self.advance();
            let mut w = Vec::new();
            while self.check(TokenKind::Word)
                && !self.check_word("do")
                && !self.check_word(";")
            {
                w.push(self.advance().unwrap().value.clone());
            }
            w
        } else {
            Vec::new()
        };

        // Optional semicolon before "do"
        self.expect(TokenKind::Semicolon);
        self.expect_word("do");
        let body = self.parse_compound_list();
        self.expect_word("done");

        ForCommand { var, words, body }
    }

    fn parse_while(&mut self) -> WhileCommand {
        // We've already consumed "while"
        let condition = self.parse_compound_list();
        self.expect_word("do");
        let body = self.parse_compound_list();
        self.expect_word("done");
        WhileCommand { condition, body }
    }

    fn parse_case(&mut self) -> CaseCommand {
        // We've already consumed "case"
        let word = self.expect(TokenKind::Word)
            .map(|t| t.value.clone())
            .unwrap_or_default();

        self.expect_word("in");

        let mut items = Vec::new();
        loop {
            // Check for "esac" or EOF
            if self.check_word("esac") || self.peek().is_none() {
                break;
            }

            let patterns = self.parse_case_patterns();
            self.expect(TokenKind::RParen);
            let body = self.parse_compound_list();
            // Optional ;; or ;
            self.expect(TokenKind::Semicolon);
            self.expect(TokenKind::Semicolon); // some use ;;

            items.push(CaseItem { patterns, body });
        }

        self.expect_word("esac");
        CaseCommand { word, items }
    }

    fn parse_case_patterns(&mut self) -> Vec<String> {
        let mut patterns = Vec::new();
        if let Some(token) = self.expect(TokenKind::Word) {
            patterns.push(token.value.clone());
        } else if self.check(TokenKind::LParen) {
            self.advance();
            if let Some(token) = self.expect(TokenKind::Word) {
                patterns.push(token.value.clone());
            }
        }
        while self.check(TokenKind::Pipe) {
            self.advance();
            if let Some(token) = self.expect(TokenKind::Word) {
                patterns.push(token.value.clone());
            }
        }
        patterns
    }

    fn parse_function(&mut self) -> FunctionDef {
        // We've already consumed "function"
        let name = self.expect(TokenKind::Word)
            .map(|t| t.value.clone())
            .unwrap_or_default();
        self.expect(TokenKind::LParen);
        self.expect(TokenKind::RParen);
        self.expect_word("{"); // or compound list
        let body = self.parse_compound_list();
        self.expect_word("}");
        FunctionDef { name, body }
    }

    // ── Compound list (for scripting bodies) ──

    pub fn parse_compound_list(&mut self) -> Program {
        let mut commands = Vec::new();
        loop {
            // Stop at keywords that close a block
            if self.check_terminator() {
                break;
            }
            self.skip_semicolons();
            if self.peek().is_none() || self.check_terminator() {
                break;
            }
            commands.push(self.parse_complete_command());
            self.skip_semicolons();
        }
        Program { commands }
    }

    fn check_terminator(&self) -> bool {
        self.peek().is_some_and(|t| {
            t.kind == TokenKind::Word && matches!(
                t.value.as_str(),
                "then" | "else" | "elif" | "fi"
                    | "do" | "done"
                    | "esac"
                    | "}"
            )
        })
    }

    // ── Helpers ──

    fn check_word(&self, word: &str) -> bool {
        self.peek().is_some_and(|t| t.kind == TokenKind::Word && t.value == word)
    }

    fn expect_word(&mut self, word: &str) -> Option<&'a Token> {
        let token = self.peek()?;
        if token.kind == TokenKind::Word && token.value == word {
            self.advance()
        } else {
            None
        }
    }
}

fn parse_fd_from_value(value: &str, kind: TokenKind) -> i32 {
    let num_part = if value.ends_with(">>") || value.ends_with("<<") {
        &value[..value.len() - 2]
    } else if value.ends_with('>') || value.ends_with('<') {
        &value[..value.len() - 1]
    } else {
        return match kind {
            TokenKind::Less | TokenKind::DLass => 0,
            _ => 1,
        };
    };
    if num_part.is_empty() {
        match kind {
            TokenKind::Less | TokenKind::DLass => 0,
            _ => 1,
        }
    } else {
        num_part.parse().unwrap_or_else(|_| {
            match kind {
                TokenKind::Less | TokenKind::DLass => 0,
                _ => 1,
            }
        })
    }
}

pub fn parse(tokens: &[Token]) -> Program {
    Parser::new(tokens).parse_program()
}

pub fn parse_compound_list(tokens: &[Token]) -> Program {
    Parser::new(tokens).parse_compound_list()
}
