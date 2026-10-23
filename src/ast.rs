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
    /// `*p` : own / ref の先を読む
    Deref(Box<Expr>),
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
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
    Expr(Expr),
    If { cond: Expr, then: Block, els: Option<Block> },
    While { cond: Expr, body: Block },
    Return { value: Option<Expr>, span: Span },
}

#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub fns: Vec<FnDef>,
}
