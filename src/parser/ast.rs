// ── Token types (output of tokenizer, input to parser) ──

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenKind {
    Word,
    Pipe,
    AndIf,
    OrIf,
    Semicolon,
    Background,
    Bang,
    LParen,
    RParen,
    Great,
    DGreat,
    Less,
    DLass,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub value: String,
}

impl Token {
    pub fn new(kind: TokenKind, value: impl Into<String>) -> Self {
        Token { kind, value: value.into() }
    }
}

// ── AST node types ──

#[derive(Debug, Clone)]
pub struct Program {
    pub commands: Vec<CompleteCommand>,
}

#[derive(Debug, Clone)]
pub struct CompleteCommand {
    pub and_or: AndOrList,
    pub background: bool,
}

#[derive(Debug, Clone)]
pub struct AndOrList {
    pub nodes: Vec<AndOrNode>,
}

#[derive(Debug, Clone)]
pub struct AndOrNode {
    pub command: CommandNode,
    pub operator: Option<AndOrOp>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AndOrOp {
    And,
    Or,
}

#[derive(Debug, Clone)]
pub enum CommandNode {
    Pipeable(Pipeline),
    Compound(ScriptCommand),
}

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub negated: bool,
    pub commands: Vec<SimpleCommand>,
}

#[derive(Debug, Clone)]
pub struct SimpleCommand {
    pub env_overrides: Vec<(String, String)>,
    pub words: Vec<String>,
    pub redirects: Vec<Redirect>,
}

#[derive(Debug, Clone)]
pub enum RedirectKind {
    Output,
    Append,
    Input,
    Heredoc,
}

#[derive(Debug, Clone)]
pub struct Redirect {
    pub fd: i32,
    pub kind: RedirectKind,
    pub target: String,
}

// ── Scripting AST nodes (parsed in Commands 3) ──

#[derive(Debug, Clone)]
pub enum ScriptCommand {
    If(IfCommand),
    For(ForCommand),
    While(WhileCommand),
    Case(CaseCommand),
    Function(FunctionDef),
    Subshell(Program),
}

#[derive(Debug, Clone)]
pub struct IfCommand {
    pub clauses: Vec<IfClause>,
    pub else_body: Option<Program>,
}

#[derive(Debug, Clone)]
pub struct IfClause {
    pub condition: Program,
    pub body: Program,
}

#[derive(Debug, Clone)]
pub struct ForCommand {
    pub var: String,
    pub words: Vec<String>,
    pub body: Program,
}

#[derive(Debug, Clone)]
pub struct WhileCommand {
    pub condition: Program,
    pub body: Program,
}

#[derive(Debug, Clone)]
pub struct CaseCommand {
    pub word: String,
    pub items: Vec<CaseItem>,
}

#[derive(Debug, Clone)]
pub struct CaseItem {
    pub patterns: Vec<String>,
    pub body: Program,
}

#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub name: String,
    pub body: Program,
}
