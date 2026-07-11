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
                    "if" => { self.advance(); return CommandNode::Compound(ScriptCommand::If(self.parse_if())); }
                    "for" => { self.advance(); return CommandNode::Compound(ScriptCommand::For(self.parse_for())); }
                    "while" => { self.advance(); return CommandNode::Compound(ScriptCommand::While(self.parse_while())); }
                    "case" => { self.advance(); return CommandNode::Compound(ScriptCommand::Case(self.parse_case())); }
                    "function" => { self.advance(); return CommandNode::Compound(ScriptCommand::Function(self.parse_function())); }
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
        let program = self.parse_compound_list();
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
            // ;; or ;
            if !self.expect(TokenKind::DSemicolon).is_some() {
                self.expect(TokenKind::Semicolon);
            }

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
            t.kind == TokenKind::DSemicolon
                || t.kind == TokenKind::RParen
                || (t.kind == TokenKind::Word && matches!(
                    t.value.as_str(),
                    "then" | "else" | "elif" | "fi"
                        | "do" | "done"
                        | "esac"
                        | "}"
                ))
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
    } else if value.ends_with(">&") || value.ends_with("<&") {
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

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::tokenize::tokenize;

    // ─── Helpers ───

    fn w(s: &str) -> String { s.to_string() }

    fn sc(words: &[&str]) -> SimpleCommand {
        SimpleCommand { env_overrides: vec![], words: words.iter().map(|s| s.to_string()).collect(), redirects: vec![] }
    }

    fn sc_env(env: &[(&str, &str)], words: &[&str]) -> SimpleCommand {
        SimpleCommand {
            env_overrides: env.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            words: words.iter().map(|s| s.to_string()).collect(),
            redirects: vec![],
        }
    }

    fn pipeable(negated: bool, cmds: Vec<SimpleCommand>) -> CommandNode {
        CommandNode::Pipeable(Pipeline { negated, commands: cmds })
    }

    fn compound(sc: ScriptCommand) -> CommandNode {
        CommandNode::Compound(sc)
    }

    fn complete(cmd: CommandNode) -> CompleteCommand {
        CompleteCommand {
            and_or: AndOrList { nodes: vec![AndOrNode { command: cmd, operator: None }] },
            background: false,
        }
    }

    fn bg(cmd: CommandNode) -> CompleteCommand {
        CompleteCommand {
            and_or: AndOrList { nodes: vec![AndOrNode { command: cmd, operator: None }] },
            background: true,
        }
    }

    fn program(commands: Vec<CompleteCommand>) -> Program {
        Program { commands }
    }

    fn parse_input(input: &str) -> Program {
        parse(&tokenize(input))
    }

    // ─── Simple commands ───

    #[test]
    fn test_parse_empty() {
        assert_eq!(parse_input(""), program(vec![]));
    }

    #[test]
    fn test_parse_single_word() {
        assert_eq!(parse_input("hello"), program(vec![complete(pipeable(false, vec![sc(&["hello"])]))]));
    }

    #[test]
    fn test_parse_multiple_words() {
        assert_eq!(parse_input("echo hello world"), program(vec![complete(pipeable(false, vec![sc(&["echo", "hello", "world"])]))]));
    }

    // ─── Semicolon separator ───

    #[test]
    fn test_semicolon() {
        let prog = parse_input("echo a; echo b");
        assert_eq!(prog.commands.len(), 2);
        assert_eq!(prog.commands[0], complete(pipeable(false, vec![sc(&["echo", "a"])])));
        assert_eq!(prog.commands[1], complete(pipeable(false, vec![sc(&["echo", "b"])])));
    }

    // ─── Pipeline ───

    #[test]
    fn test_pipeline() {
        let prog = parse_input("echo a | cat");
        assert_eq!(prog, program(vec![complete(
            pipeable(false, vec![sc(&["echo", "a"]), sc(&["cat"])])
        )]));
    }

    #[test]
    fn test_pipeline_negated() {
        let prog = parse_input("! echo a | cat");
        assert_eq!(prog, program(vec![complete(
            pipeable(true, vec![sc(&["echo", "a"]), sc(&["cat"])])
        )]));
    }

    // ─── AND / OR ───

    #[test]
    fn test_and_or() {
        let prog = parse_input("true && echo y || echo n");
        assert_eq!(prog.commands.len(), 1);
        let nodes = &prog.commands[0].and_or.nodes;
        assert_eq!(nodes.len(), 3);
        // operators are on the node that follows && / ||
        assert_eq!(nodes[0].command, pipeable(false, vec![sc(&["true"])]));
        assert_eq!(nodes[0].operator, None);
        assert_eq!(nodes[1].command, pipeable(false, vec![sc(&["echo", "y"])]));
        assert_eq!(nodes[1].operator, Some(AndOrOp::And));
        assert_eq!(nodes[2].command, pipeable(false, vec![sc(&["echo", "n"])]));
        assert_eq!(nodes[2].operator, Some(AndOrOp::Or));
    }

    // ─── Background ───

    #[test]
    fn test_background() {
        assert_eq!(parse_input("sleep 1 &"), program(vec![bg(pipeable(false, vec![sc(&["sleep", "1"])]))]));
    }

    // ─── Env overrides ───

    #[test]
    fn test_env_override() {
        let prog = parse_input("FOO=bar echo");
        assert_eq!(prog, program(vec![complete(pipeable(
            false, vec![sc_env(&[("FOO", "bar")], &["echo"])]
        ))]));
    }

    #[test]
    fn test_pure_assignment() {
        let prog = parse_input("FOO=bar");
        assert_eq!(prog, program(vec![complete(pipeable(
            false, vec![sc_env(&[("FOO", "bar")], &[])]
        ))]));
    }

    #[test]
    fn test_multiple_env_overrides() {
        let prog = parse_input("A=1 B=2 cmd");
        assert_eq!(prog, program(vec![complete(pipeable(
            false, vec![sc_env(&[("A", "1"), ("B", "2")], &["cmd"])]
        ))]));
    }

    // ─── Redirects ───

    #[test]
    fn test_redirect_output() {
        let prog = parse_input("echo > file");
        let Pipeline { commands, .. } = match &prog.commands[0].and_or.nodes[0].command {
            CommandNode::Pipeable(p) => p,
            _ => panic!("expected pipeable"),
        };
        assert_eq!(commands[0].words, vec![w("echo")]);
        assert_eq!(commands[0].redirects.len(), 1);
        assert_eq!(commands[0].redirects[0].kind, RedirectKind::Output);
        assert_eq!(commands[0].redirects[0].target, "file");
    }

    #[test]
    fn test_redirect_input() {
        let prog = parse_input("cat < file");
        let Pipeline { commands, .. } = match &prog.commands[0].and_or.nodes[0].command {
            CommandNode::Pipeable(p) => p,
            _ => panic!("expected pipeable"),
        };
        assert_eq!(commands[0].redirects[0].kind, RedirectKind::Input);
        assert_eq!(commands[0].redirects[0].target, "file");
    }

    #[test]
    fn test_redirect_append() {
        let prog = parse_input("echo >> file");
        let Pipeline { commands, .. } = match &prog.commands[0].and_or.nodes[0].command {
            CommandNode::Pipeable(p) => p,
            _ => panic!("expected pipeable"),
        };
        assert_eq!(commands[0].redirects[0].kind, RedirectKind::Append);
    }

    #[test]
    fn test_redirect_stderr() {
        let prog = parse_input("echo 2>&1");
        let Pipeline { commands, .. } = match &prog.commands[0].and_or.nodes[0].command {
            CommandNode::Pipeable(p) => p,
            _ => panic!("expected pipeable"),
        };
        assert_eq!(commands[0].redirects[0].kind, RedirectKind::Output);
        assert_eq!(commands[0].redirects[0].fd, 2);
        assert_eq!(commands[0].redirects[0].target, "1");
    }

    #[test]
    fn test_heredoc() {
        let prog = parse_input("cat << EOF");
        let Pipeline { commands, .. } = match &prog.commands[0].and_or.nodes[0].command {
            CommandNode::Pipeable(p) => p,
            _ => panic!("expected pipeable"),
        };
        assert_eq!(commands[0].redirects[0].kind, RedirectKind::Heredoc);
        assert_eq!(commands[0].redirects[0].target, "EOF");
    }

    // ─── Subshell ───

    #[test]
    fn test_subshell() {
        let prog = parse_input("(echo hi)");
        let body = Program { commands: vec![complete(pipeable(false, vec![sc(&["echo", "hi"])]))] };
        assert_eq!(prog, program(vec![complete(compound(ScriptCommand::Subshell(body)))]));
    }

    // ─── Scripting: if ───

    #[test]
    fn test_if() {
        let prog = parse_input("if true; then echo ok; fi");
        let condition = Program { commands: vec![complete(pipeable(false, vec![sc(&["true"])]))] };
        let body = Program { commands: vec![complete(pipeable(false, vec![sc(&["echo", "ok"])]))] };
        let if_cmd = ScriptCommand::If(IfCommand {
            clauses: vec![IfClause { condition, body }],
            else_body: None,
        });
        assert_eq!(prog, program(vec![complete(compound(if_cmd))]));
    }

    #[test]
    fn test_if_else() {
        let prog = parse_input("if false; then echo a; else echo b; fi");
        let cond = Program { commands: vec![complete(pipeable(false, vec![sc(&["false"])]))] };
        let body_a = Program { commands: vec![complete(pipeable(false, vec![sc(&["echo", "a"])]))] };
        let body_b = Program { commands: vec![complete(pipeable(false, vec![sc(&["echo", "b"])]))] };
        let if_cmd = ScriptCommand::If(IfCommand {
            clauses: vec![IfClause { condition: cond, body: body_a }],
            else_body: Some(body_b),
        });
        assert_eq!(prog, program(vec![complete(compound(if_cmd))]));
    }

    #[test]
    fn test_if_elif() {
        let prog = parse_input("if false; then echo a; elif true; then echo b; fi");
        let c1 = Program { commands: vec![complete(pipeable(false, vec![sc(&["false"])]))] };
        let b1 = Program { commands: vec![complete(pipeable(false, vec![sc(&["echo", "a"])]))] };
        let c2 = Program { commands: vec![complete(pipeable(false, vec![sc(&["true"])]))] };
        let b2 = Program { commands: vec![complete(pipeable(false, vec![sc(&["echo", "b"])]))] };
        let if_cmd = ScriptCommand::If(IfCommand {
            clauses: vec![IfClause { condition: c1, body: b1 }, IfClause { condition: c2, body: b2 }],
            else_body: None,
        });
        assert_eq!(prog, program(vec![complete(compound(if_cmd))]));
    }

    // ─── Scripting: for ───

    #[test]
    fn test_for() {
        let prog = parse_input("for i in a b c; do echo $i; done");
        let body = Program { commands: vec![complete(pipeable(false, vec![sc(&["echo", "$i"])]))] };
        let for_cmd = ScriptCommand::For(ForCommand {
            var: "i".into(),
            words: vec![w("a"), w("b"), w("c")],
            body,
        });
        assert_eq!(prog, program(vec![complete(compound(for_cmd))]));
    }

    // ─── Scripting: while ───

    #[test]
    fn test_while() {
        let prog = parse_input("while true; do echo loop; done");
        let cond = Program { commands: vec![complete(pipeable(false, vec![sc(&["true"])]))] };
        let body = Program { commands: vec![complete(pipeable(false, vec![sc(&["echo", "loop"])]))] };
        let while_cmd = ScriptCommand::While(WhileCommand { condition: cond, body });
        assert_eq!(prog, program(vec![complete(compound(while_cmd))]));
    }

    // ─── Scripting: case ───

    #[test]
    fn test_case() {
        let prog = parse_input("case x in x) echo m;; esac");
        let body = Program { commands: vec![complete(pipeable(false, vec![sc(&["echo", "m"])]))] };
        let case_cmd = ScriptCommand::Case(CaseCommand {
            word: "x".into(),
            items: vec![CaseItem { patterns: vec![w("x")], body }],
        });
        assert_eq!(prog, program(vec![complete(compound(case_cmd))]));
    }

    #[test]
    fn test_case_multiple_patterns() {
        let prog = parse_input("case x in x|y) echo m;; esac");
        let body = Program { commands: vec![complete(pipeable(false, vec![sc(&["echo", "m"])]))] };
        let case_cmd = ScriptCommand::Case(CaseCommand {
            word: "x".into(),
            items: vec![CaseItem { patterns: vec![w("x"), w("y")], body }],
        });
        assert_eq!(prog, program(vec![complete(compound(case_cmd))]));
    }

    #[test]
    fn test_case_multiple_items() {
        let prog = parse_input("case x in x) echo a;; y) echo b;; esac");
        let body_a = Program { commands: vec![complete(pipeable(false, vec![sc(&["echo", "a"])]))] };
        let body_b = Program { commands: vec![complete(pipeable(false, vec![sc(&["echo", "b"])]))] };
        let case_cmd = ScriptCommand::Case(CaseCommand {
            word: "x".into(),
            items: vec![
                CaseItem { patterns: vec![w("x")], body: body_a },
                CaseItem { patterns: vec![w("y")], body: body_b },
            ],
        });
        assert_eq!(prog, program(vec![complete(compound(case_cmd))]));
    }

    // ─── Combined / complex ───

    #[test]
    fn test_pipeline_with_redirect() {
        let prog = parse_input("echo hello > file | cat");
        let cmds = match &prog.commands[0].and_or.nodes[0].command {
            CommandNode::Pipeable(p) => &p.commands,
            _ => panic!("expected pipeable"),
        };
        // First command in pipeline has the redirect
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].words, vec![w("echo"), w("hello")]);
        assert_eq!(cmds[0].redirects.len(), 1);
        assert_eq!(cmds[1].words, vec![w("cat")]);
    }
}
