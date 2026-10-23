use std::cell::Cell;

use crate::diag::Span;

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

/// `*p` で ref の世代チェックが要るかどうか。静的解析（`analyze`）が書き込む。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Check {
    /// 解析がまだ見ていない（実行時に検査する）
    Unvisited,
    /// 「借りた先は必ず生きている」と証明できた。実行時の検査を消す
    Elided,
    /// 証明できなかった。実行時に検査する
    Required,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Int(i64),
    Bool(bool),
    Str(String),
    Var(String),
    /// `own e` : ヒープに値を置き、唯一の所有者を返す
    Own(Box<Expr>),
    /// `ref p` : 所有者（変数か `*p`）を世代付き参照として借りる
    Ref(Box<Expr>),
    /// `*p` : own / ref の先を読む。2つ目は静的解析の結論（世代チェックを消せるか）。
    Deref(Box<Expr>, Cell<Check>),
    /// `e.f` : フィールド。`e` が own / ref なら自動で先をたどる（そのときの検査が `Check`）。
    Field(Box<Expr>, String, Cell<Check>),
    /// `Name { f: e, ... }` : 構造体リテラル。フィールドはソースに書いた順
    StructLit(String, Vec<FieldInit>),
    /// `if c { ... } else { ... }`。文としても値としても使える。値はブロックの最後の行。
    /// 値として使う位置では else が必須（パーサが確かめる）。
    If { cond: Box<Expr>, then: Block, els: Option<Block> },
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}

#[derive(Debug, Clone)]
pub struct FieldInit {
    pub name: String,
    pub span: Span,
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    /// 閉じ括弧 `}` の位置。スコープ終了での解放を指し示すのに使う。
    pub end: Span,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let { name: String, init: Expr },
    Assign { target: Expr, value: Expr, span: Span },
    /// 式文。`if` もここに入る（文として使う if）
    Expr(Expr),
    While { cond: Expr, body: Block },
    Loop { body: Block },
    /// `for i in range(a, b, step) { ... } else { ... }`。else は 1 回も回らなかったときに実行
    For { var: String, range: Vec<Expr>, range_span: Span, body: Block, els: Option<Block> },
    Break(Span),
    Continue(Span),
    Return { value: Option<Expr>, span: Span },
}

#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Block,
    pub span: Span,
}

/// `struct Name { a, b }`。フィールドに型はまだない。
#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<String>,
    pub span: Span,
}

impl StructDef {
    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|f| f == name)
    }
}

#[derive(Debug, Clone)]
pub struct Program {
    pub structs: Vec<StructDef>,
    pub fns: Vec<FnDef>,
}
