use std::cell::Cell;

use crate::ast::*;
use crate::diag::{Diagnostic, Span};
use crate::lexer::{Tok, Token};

type R<T> = Result<T, Diagnostic>;

pub struct Parser {
    toks: Vec<Token>,
    pos: usize,
    /// `if` / `while` の条件の中では `Name {` を構造体リテラルと読まない（ブロックの `{` と区別するため）
    no_struct_lit: bool,
}

impl Parser {
    pub fn new(toks: Vec<Token>) -> Self {
        Parser { toks, pos: 0, no_struct_lit: false }
    }

    fn peek(&self) -> &Tok {
        &self.toks[self.pos].tok
    }

    fn span(&self) -> Span {
        self.toks[self.pos].span
    }

    fn prev_span(&self) -> Span {
        self.toks[self.pos.saturating_sub(1)].span
    }

    fn bump(&mut self) -> Token {
        let t = self.toks[self.pos].clone();
        if self.pos + 1 < self.toks.len() {
            self.pos += 1;
        }
        t
    }

    fn skip_newlines(&mut self) {
        while *self.peek() == Tok::Newline {
            self.bump();
        }
    }

    fn skip_seps(&mut self) {
        while matches!(self.peek(), Tok::Newline | Tok::Semi) {
            self.bump();
        }
    }

    fn unexpected(&self, expected: &str) -> Diagnostic {
        Diagnostic::new(format!("構文エラー: {}が必要です", expected))
            .primary(self.span(), format!("ここに {} があります", self.peek().describe()))
    }

    fn expect(&mut self, t: Tok, expected: &str) -> R<Span> {
        if *self.peek() == t {
            Ok(self.bump().span)
        } else {
            Err(self.unexpected(expected))
        }
    }

    fn expect_ident(&mut self, what: &str) -> R<String> {
        if let Tok::Ident(name) = self.peek().clone() {
            self.bump();
            Ok(name)
        } else {
            Err(self.unexpected(what))
        }
    }

    pub fn parse_program(&mut self) -> R<Program> {
        let mut fns = Vec::new();
        let mut structs = Vec::new();
        loop {
            self.skip_seps();
            match self.peek() {
                Tok::Eof => break,
                Tok::Struct => structs.push(self.parse_struct()?),
                _ => fns.push(self.parse_fn()?),
            }
        }
        Ok(Program { structs, fns })
    }

    fn parse_struct(&mut self) -> R<StructDef> {
        let start = self.expect(Tok::Struct, "`struct`")?;
        let name = self.expect_ident("構造体名")?;
        let span = start.join(self.prev_span());
        self.expect(Tok::LBrace, "`{`")?;
        let mut fields: Vec<String> = Vec::new();
        self.skip_newlines();
        while *self.peek() != Tok::RBrace {
            let fspan = self.span();
            let f = self.expect_ident("フィールド名")?;
            if fields.contains(&f) {
                return Err(Diagnostic::new(format!("フィールド `{}` が二重に定義されています", f))
                    .primary(fspan, "同じ名前のフィールドがすでにある"));
            }
            fields.push(f);
            self.skip_newlines();
            if *self.peek() == Tok::Comma {
                self.bump();
                self.skip_newlines();
            } else {
                break;
            }
        }
        self.expect(Tok::RBrace, "`}`")?;
        Ok(StructDef { name, fields, span })
    }

    fn parse_fn(&mut self) -> R<FnDef> {
        let start = self.expect(Tok::Fn, "`fn` か `struct`（トップレベルには関数と構造体の定義だけ書けます）")?;
        let name = self.expect_ident("関数名")?;
        self.expect(Tok::LParen, "`(`")?;
        let mut params = Vec::new();
        self.skip_newlines();
        while *self.peek() != Tok::RParen {
            params.push(self.expect_ident("引数名")?);
            self.skip_newlines();
            if *self.peek() == Tok::Comma {
                self.bump();
                self.skip_newlines();
            } else {
                break;
            }
        }
        self.expect(Tok::RParen, "`)`")?;
        let header = start.join(self.prev_span());
        let body = self.parse_block()?;
        Ok(FnDef { name, params, body, span: header })
    }

    fn parse_block(&mut self) -> R<Block> {
        self.expect(Tok::LBrace, "`{`")?;
        let mut stmts = Vec::new();
        loop {
            self.skip_seps();
            match self.peek().clone() {
                Tok::RBrace => {
                    let end = self.bump().span;
                    return Ok(Block { stmts, end });
                }
                Tok::Eof => {
                    return Err(Diagnostic::new("構文エラー: `}` が閉じていません")
                        .primary(self.span(), "ファイルがここで終わっています"));
                }
                _ => {}
            }
            stmts.push(self.parse_stmt()?);
            match self.peek().clone() {
                Tok::Newline | Tok::Semi | Tok::RBrace => {}
                _ => return Err(self.unexpected("改行か `;`（文の終わり）")),
            }
        }
    }

    fn parse_stmt(&mut self) -> R<Stmt> {
        match self.peek().clone() {
            Tok::Let => {
                self.bump();
                let name = self.expect_ident("変数名")?;
                self.expect(Tok::Assign, "`=`")?;
                self.skip_newlines();
                let init = self.parse_expr()?;
                Ok(Stmt::Let { name, init })
            }
            Tok::If => self.parse_if(),
            Tok::While => {
                self.bump();
                let cond = self.parse_cond()?;
                let body = self.parse_block()?;
                Ok(Stmt::While { cond, body })
            }
            Tok::Return => {
                let start = self.bump().span;
                let value = match self.peek().clone() {
                    Tok::Newline | Tok::Semi | Tok::RBrace | Tok::Eof => None,
                    _ => Some(self.parse_expr()?),
                };
                let span = start.join(self.prev_span());
                Ok(Stmt::Return { value, span })
            }
            _ => {
                let e = self.parse_expr()?;
                if *self.peek() == Tok::Assign {
                    if !matches!(e.kind, ExprKind::Var(_) | ExprKind::Deref(..) | ExprKind::Field(..)) {
                        return Err(Diagnostic::new("構文エラー: ここには代入できません")
                            .primary(e.span, "代入先は変数名か `*p` か `p.f` だけです"));
                    }
                    self.bump();
                    self.skip_newlines();
                    let value = self.parse_expr()?;
                    let span = e.span.join(value.span);
                    Ok(Stmt::Assign { target: e, value, span })
                } else {
                    Ok(Stmt::Expr(e))
                }
            }
        }
    }

    fn parse_if(&mut self) -> R<Stmt> {
        self.expect(Tok::If, "`if`")?;
        let cond = self.parse_cond()?;
        let then = self.parse_block()?;

        // `}` の次の行に else がある書き方も許す
        let save = self.pos;
        self.skip_newlines();
        let els = if *self.peek() == Tok::Else {
            self.bump();
            if *self.peek() == Tok::If {
                let nested = self.parse_if()?;
                let end = self.prev_span();
                Some(Block { stmts: vec![nested], end })
            } else {
                Some(self.parse_block()?)
            }
        } else {
            self.pos = save;
            None
        };
        Ok(Stmt::If { cond, then, els })
    }

    pub fn parse_expr(&mut self) -> R<Expr> {
        self.parse_binary(1)
    }

    /// `if` / `while` の条件。ここでは構造体リテラルを書けない（括弧で囲めば書ける）。
    fn parse_cond(&mut self) -> R<Expr> {
        self.with_struct_lit(false, |p| p.parse_expr())
    }

    fn with_struct_lit<T>(&mut self, allowed: bool, f: impl FnOnce(&mut Self) -> R<T>) -> R<T> {
        let saved = std::mem::replace(&mut self.no_struct_lit, !allowed);
        let r = f(self);
        self.no_struct_lit = saved;
        r
    }

    fn parse_binary(&mut self, min_prec: u8) -> R<Expr> {
        let mut lhs = self.parse_unary()?;
        loop {
            let (op, prec) = match binop_info(self.peek()) {
                Some(x) if x.1 >= min_prec => x,
                _ => break,
            };
            self.bump();
            self.skip_newlines();
            let rhs = self.parse_binary(prec + 1)?;
            let span = lhs.span.join(rhs.span);
            lhs = Expr {
                kind: ExprKind::Binary(op, Box::new(lhs), Box::new(rhs)),
                span,
            };
        }
        Ok(lhs)
    }

    /// 前置演算子。
    /// `ref` と `*` は強く結びつく（`ref a`, `*r + 1` は `(*r) + 1`）。
    /// `.f` はさらに強い（`ref p.x` は `ref (p.x)`、`*p.x` は `*(p.x)`）。
    /// `own` は右側の式全体を取る（`own n * 2` は `own (n * 2)`）。
    fn parse_unary(&mut self) -> R<Expr> {
        match self.peek().clone() {
            Tok::Minus => {
                let start = self.bump().span;
                let e = self.parse_unary()?;
                let span = start.join(e.span);
                Ok(Expr { kind: ExprKind::Unary(UnOp::Neg, Box::new(e)), span })
            }
            Tok::Bang => {
                let start = self.bump().span;
                let e = self.parse_unary()?;
                let span = start.join(e.span);
                Ok(Expr { kind: ExprKind::Unary(UnOp::Not, Box::new(e)), span })
            }
            Tok::Star => {
                let start = self.bump().span;
                let e = self.parse_unary()?;
                let span = start.join(e.span);
                Ok(Expr { kind: ExprKind::Deref(Box::new(e), Cell::new(Check::Unvisited)), span })
            }
            Tok::Ref => {
                let start = self.bump().span;
                let e = self.parse_unary()?;
                let span = start.join(e.span);
                Ok(Expr { kind: ExprKind::Ref(Box::new(e)), span })
            }
            Tok::Own => {
                let start = self.bump().span;
                let e = self.parse_expr()?;
                let span = start.join(e.span);
                Ok(Expr { kind: ExprKind::Own(Box::new(e)), span })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> R<Expr> {
        let mut e = self.parse_primary()?;
        while *self.peek() == Tok::Dot {
            self.bump();
            let fspan = self.span();
            let name = self.expect_ident("フィールド名")?;
            let span = e.span.join(fspan);
            e = Expr { kind: ExprKind::Field(Box::new(e), name, Cell::new(Check::Unvisited)), span };
        }
        Ok(e)
    }

    fn parse_primary(&mut self) -> R<Expr> {
        match self.peek().clone() {
            Tok::Int(n) => {
                let span = self.bump().span;
                Ok(Expr { kind: ExprKind::Int(n), span })
            }
            Tok::Str(s) => {
                let span = self.bump().span;
                Ok(Expr { kind: ExprKind::Str(s), span })
            }
            Tok::True => {
                let span = self.bump().span;
                Ok(Expr { kind: ExprKind::Bool(true), span })
            }
            Tok::False => {
                let span = self.bump().span;
                Ok(Expr { kind: ExprKind::Bool(false), span })
            }
            Tok::Ident(name) => {
                let start = self.bump().span;
                if *self.peek() == Tok::LParen {
                    self.bump();
                    self.skip_newlines();
                    let mut args = Vec::new();
                    while *self.peek() != Tok::RParen {
                        args.push(self.with_struct_lit(true, |p| p.parse_expr())?);
                        self.skip_newlines();
                        if *self.peek() == Tok::Comma {
                            self.bump();
                            self.skip_newlines();
                        } else {
                            break;
                        }
                    }
                    let end = self.expect(Tok::RParen, "`)`")?;
                    Ok(Expr {
                        kind: ExprKind::Call(name, args),
                        span: start.join(end),
                    })
                } else if *self.peek() == Tok::LBrace && !self.no_struct_lit {
                    self.parse_struct_lit(name, start)
                } else {
                    Ok(Expr { kind: ExprKind::Var(name), span: start })
                }
            }
            Tok::LParen => {
                let start = self.bump().span;
                self.skip_newlines();
                let inner = self.with_struct_lit(true, |p| p.parse_expr())?;
                self.skip_newlines();
                let end = self.expect(Tok::RParen, "`)`")?;
                Ok(Expr { kind: inner.kind, span: start.join(end) })
            }
            _ => Err(self.unexpected("式")),
        }
    }

    fn parse_struct_lit(&mut self, name: String, start: Span) -> R<Expr> {
        self.expect(Tok::LBrace, "`{`")?;
        self.skip_newlines();
        let mut fields = Vec::new();
        while *self.peek() != Tok::RBrace {
            let span = self.span();
            let fname = self.expect_ident("フィールド名")?;
            self.expect(Tok::Colon, "`:`")?;
            self.skip_newlines();
            let value = self.with_struct_lit(true, |p| p.parse_expr())?;
            fields.push(FieldInit { name: fname, span, value });
            self.skip_newlines();
            if *self.peek() == Tok::Comma {
                self.bump();
                self.skip_newlines();
            } else {
                break;
            }
        }
        let end = self.expect(Tok::RBrace, "`}`")?;
        Ok(Expr { kind: ExprKind::StructLit(name, fields), span: start.join(end) })
    }
}

fn binop_info(t: &Tok) -> Option<(BinOp, u8)> {
    Some(match t {
        Tok::OrOr => (BinOp::Or, 1),
        Tok::AndAnd => (BinOp::And, 2),
        Tok::EqEq => (BinOp::Eq, 3),
        Tok::NotEq => (BinOp::Ne, 3),
        Tok::Lt => (BinOp::Lt, 4),
        Tok::Le => (BinOp::Le, 4),
        Tok::Gt => (BinOp::Gt, 4),
        Tok::Ge => (BinOp::Ge, 4),
        Tok::Plus => (BinOp::Add, 5),
        Tok::Minus => (BinOp::Sub, 5),
        Tok::Star => (BinOp::Mul, 6),
        Tok::Slash => (BinOp::Div, 6),
        Tok::Percent => (BinOp::Rem, 6),
        _ => return None,
    })
}
