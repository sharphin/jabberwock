//! 型検査。実行する前にプログラム全体を調べる。
//!
//! 型は関数の引数・戻り値・構造体のフィールドに書き、`let` は初期値から決める（前から順に決めるだけ）。
//! 型の食い違いと、`mut` / `ref mut` の違反（書き換えてはいけない場所への書き込み）をここで見つける。
//!
//! ムーブ済みの変数を使う・ダングリングした ref を使う、は型の問題ではないので、ここでは扱わない
//! （ref の安全は世代付き ref と `analyze` が守る。ref の型に寿命は書かない）。

use std::collections::HashMap;

use crate::ast::*;
use crate::diag::{Diagnostic, Span};

type R<T> = Result<T, Diagnostic>;

const BUILTINS: [&str; 3] = ["print", "drop", "assert"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    I64,
    Bool,
    Str,
    /// 値なし（戻り値のない関数の呼び出しなど）
    Unit,
    /// 決して終わらない（return / break / continue で必ず抜ける）。どの型にも合う
    Never,
    Struct(String),
    Own(Box<Ty>),
    /// bool は `mut`
    Ref(Box<Ty>, bool),
}

impl Ty {
    pub fn show(&self) -> String {
        match self {
            Ty::I64 => "i64".to_string(),
            Ty::Bool => "bool".to_string(),
            Ty::Str => "str".to_string(),
            Ty::Unit => "値なし".to_string(),
            Ty::Never => "（終わらない）".to_string(),
            Ty::Struct(n) => n.clone(),
            Ty::Own(t) => format!("own {}", t.show()),
            Ty::Ref(t, false) => format!("ref {}", t.show()),
            Ty::Ref(t, true) => format!("ref mut {}", t.show()),
        }
    }

    /// own / ref の先の型
    fn pointee(&self) -> Option<&Ty> {
        match self {
            Ty::Own(t) | Ty::Ref(t, _) => Some(t),
            _ => None,
        }
    }
}

/// `from` の値を `to` の場所に入れられるか。`ref mut T` は `ref T` としても使える
fn assignable(from: &Ty, to: &Ty) -> bool {
    match (from, to) {
        (Ty::Never, _) => true,
        (Ty::Ref(a, true), Ty::Ref(b, false)) => a == b,
        _ => from == to,
    }
}

fn mismatch(want: &Ty, got: &Ty, span: Span) -> Diagnostic {
    Diagnostic::new("型が合いません")
        .primary(span, format!("ここは `{}` ですが、`{}` が必要です", got.show(), want.show()))
}

struct VarInfo {
    ty: Ty,
    mutable: bool,
    decl: Span,
}

/// 場所に書き込めるか（interp の `Access` と同じ考え方を、型から静的に決める）
enum Access {
    Write,
    ImmutVar { name: String, decl: Span },
    ReadOnlyRef { what: String, origin: Span },
}

impl Access {
    /// ポインタ（型 `t` の式 `e`）の先の場所に書き込めるか。own の先は持ち主と同じ、ref の先は ref 次第
    fn through(self, t: &Ty, origin: Span, what: String) -> Access {
        match t {
            Ty::Ref(_, true) => Access::Write,
            Ty::Ref(_, false) => Access::ReadOnlyRef { what, origin },
            _ => self,
        }
    }

    fn require(&self, span: Span, doing: &str) -> R<()> {
        match self {
            Access::Write => Ok(()),
            Access::ImmutVar { name, decl } => Err(Diagnostic::new(format!(
                "`{}` は mut ではないので書き換えられません",
                name
            ))
            .label(*decl, format!("ここで `{}` を宣言した（mut がない）", name))
            .primary(span, format!("ここで{}", doing))
            .note(format!("書き換えるなら `let mut {}` や `i64 mut {}` のように mut をつけてください", name, name))),
            Access::ReadOnlyRef { what, origin } => Err(Diagnostic::new("読むだけの ref を通して書き換えようとしました")
                .label(*origin, format!("`{}` は読むだけの ref（mut がない）", what))
                .primary(span, format!("ここで{}", doing))
                .note("書き換えるなら `ref mut` で借りてください")),
        }
    }
}

struct FnSig {
    params: Vec<Ty>,
    ret: Ty,
}

struct Checker<'a> {
    structs: HashMap<&'a str, (&'a StructDef, Vec<Ty>)>,
    fns: HashMap<&'a str, FnSig>,
    scopes: Vec<HashMap<String, VarInfo>>,
    /// 今調べている関数の戻り値の型
    ret: Ty,
}

/// プログラム全体の型を調べる
pub fn check(prog: &Program) -> R<()> {
    let mut c = Checker { structs: HashMap::new(), fns: HashMap::new(), scopes: Vec::new(), ret: Ty::Unit };
    for s in &prog.structs {
        if c.structs.contains_key(s.name.as_str()) {
            return Err(Diagnostic::new(format!("構造体 `{}` が二重に定義されています", s.name))
                .primary(s.span, "ここで再定義されている"));
        }
        c.structs.insert(&s.name, (s, Vec::new()));
    }
    for s in &prog.structs {
        let tys = s.field_types.iter().map(|t| c.resolve(t)).collect::<R<Vec<_>>>()?;
        c.structs.get_mut(s.name.as_str()).expect("inserted").1 = tys;
    }
    for s in &prog.structs {
        c.check_finite(s)?;
    }
    for f in &prog.fns {
        if BUILTINS.contains(&f.name.as_str()) {
            return Err(Diagnostic::new(format!("`{}` は組み込み関数の名前です", f.name))
                .primary(f.span, "別の名前にしてください"));
        }
        if c.fns.contains_key(f.name.as_str()) {
            return Err(Diagnostic::new(format!("関数 `{}` が二重に定義されています", f.name))
                .primary(f.span, "ここで再定義されている"));
        }
        if f.name == "main" && (f.ret.is_some() || !f.params.is_empty()) {
            return Err(Diagnostic::new("`main` は引数も戻り値もない関数です")
                .primary(f.span, "ここ")
                .note("`fn main() { ... }` と書いてください"));
        }
        let params = f.params.iter().map(|p| c.resolve(&p.ty)).collect::<R<Vec<_>>>()?;
        let ret = match &f.ret {
            Some(t) => c.resolve(t)?,
            None => Ty::Unit,
        };
        c.fns.insert(&f.name, FnSig { params, ret });
    }
    for f in &prog.fns {
        c.check_fn(f)?;
    }
    Ok(())
}

impl<'a> Checker<'a> {
    fn resolve(&self, t: &TypeExpr) -> R<Ty> {
        Ok(match &t.kind {
            TypeKind::Own(inner) => Ty::Own(Box::new(self.resolve(inner)?)),
            TypeKind::Ref(inner, m) => Ty::Ref(Box::new(self.resolve(inner)?), *m),
            TypeKind::Name(n) => match n.as_str() {
                "i64" => Ty::I64,
                "bool" => Ty::Bool,
                "str" => Ty::Str,
                "i8" | "i16" | "i32" | "u8" | "u16" | "u32" | "u64" | "usize" | "isize" | "f32" | "f64" => {
                    return Err(Diagnostic::new(format!("型 `{}` はまだ使えません", n))
                        .primary(t.span, "今使える数の型は i64 だけです"))
                }
                _ if self.structs.contains_key(n.as_str()) => Ty::Struct(n.clone()),
                _ => {
                    return Err(Diagnostic::new(format!("未定義の型 `{}`", n))
                        .primary(t.span, "この型は定義されていません")
                        .note("使える型: i64、bool、str、構造体の名前、own 型、ref 型、ref mut 型"))
                }
            },
        })
    }

    /// 構造体が自分自身を（own / ref を通さず）じかに含んでいないか。含むと大きさが無限になる
    fn check_finite(&self, s: &StructDef) -> R<()> {
        let mut stack = vec![s.name.as_str()];
        let mut seen: Vec<&str> = Vec::new();
        while let Some(n) = stack.pop() {
            for t in &self.structs[n].1 {
                if let Ty::Struct(m) = t {
                    if m == &s.name {
                        return Err(Diagnostic::new(format!("構造体 `{}` が自分自身をじかに含んでいます", s.name))
                            .primary(s.span, "これだと大きさが無限になる")
                            .note("`own` を通して持ってください（例: `own Node next;`）"));
                    }
                    if !seen.contains(&m.as_str()) {
                        seen.push(m);
                        stack.push(m);
                    }
                }
            }
        }
        Ok(())
    }

    fn check_fn(&mut self, f: &FnDef) -> R<()> {
        let sig = &self.fns[f.name.as_str()];
        self.ret = sig.ret.clone();
        let mut params = HashMap::new();
        for (p, t) in f.params.iter().zip(sig.params.clone()) {
            if params.contains_key(&p.name) {
                return Err(Diagnostic::new(format!("引数 `{}` が二回あります", p.name))
                    .primary(p.span, "同じ名前の引数がすでにある"));
            }
            params.insert(p.name.clone(), VarInfo { ty: t, mutable: p.mutable, decl: p.span });
        }
        self.scopes = vec![params];
        let ret = self.ret.clone();
        let wants = ret != Ty::Unit;
        let got = self.block(&f.body, wants)?;
        if wants && !assignable(&got, &ret) {
            if got == Ty::Unit {
                return Err(Diagnostic::new("値を返さないまま関数が終わる道があります")
                    .primary(f.body.end, format!("ここで終わるが、`{}` の値を返していない", ret.show()))
                    .note("最後に `;` のない式を書くか、`return 値;` で返してください"));
            }
            let span = match f.body.stmts.last() {
                Some(Stmt::Expr(e)) => e.span,
                _ => f.body.end,
            };
            return Err(mismatch(&ret, &got, span));
        }
        Ok(())
    }

    fn lookup(&self, name: &str) -> Option<&VarInfo> {
        self.scopes.iter().rev().find_map(|s| s.get(name))
    }

    fn declare(&mut self, name: &str, info: VarInfo) {
        self.scopes.last_mut().expect("scope").insert(name.to_string(), info);
    }

    /// ブロックの型。`want_value` なら最後の `;` のない式の型（なければ値なし）。
    /// 途中で必ず抜けるなら Never
    fn block(&mut self, b: &Block, want_value: bool) -> R<Ty> {
        self.scopes.push(HashMap::new());
        let r = self.block_inner(b, want_value);
        self.scopes.pop();
        r
    }

    fn block_inner(&mut self, b: &Block, want_value: bool) -> R<Ty> {
        let mut diverges = false;
        for (i, s) in b.stmts.iter().enumerate() {
            match s {
                Stmt::Expr(e) if want_value && b.tail && i + 1 == b.stmts.len() => {
                    let t = self.expr_ctx(e, true)?;
                    return Ok(if diverges { Ty::Never } else { t });
                }
                _ => diverges |= self.stmt(s)?,
            }
        }
        Ok(if diverges { Ty::Never } else { Ty::Unit })
    }

    /// 文を調べる。必ずそこで抜ける（return など）なら true
    fn stmt(&mut self, s: &Stmt) -> R<bool> {
        match s {
            Stmt::Let { name, ty, mutable, span, init } => {
                let got = self.expr(init)?;
                if got == Ty::Unit {
                    return Err(Diagnostic::new("値のない式は変数に入れられません")
                        .primary(init.span, "これは値を返さない"));
                }
                let t = match ty {
                    Some(te) => {
                        let want = self.resolve(te)?;
                        if !assignable(&got, &want) {
                            return Err(mismatch(&want, &got, init.span).label(te.span, "ここで型を決めた"));
                        }
                        want
                    }
                    None => got,
                };
                self.declare(name, VarInfo { ty: t, mutable: *mutable, decl: *span });
                Ok(false)
            }
            Stmt::Assign { target, value, .. } => {
                let want = self.expr(target)?;
                self.access(target)?.require(target.span, "書き換えようとした")?;
                let got = self.expr(value)?;
                if !assignable(&got, &want) {
                    return Err(mismatch(&want, &got, value.span));
                }
                Ok(false)
            }
            Stmt::Expr(e) => Ok(self.expr_ctx(e, false)? == Ty::Never),
            Stmt::While { cond, body } => {
                self.expect(cond, &Ty::Bool)?;
                self.block(body, false)?;
                Ok(false)
            }
            Stmt::Loop { body } => {
                self.block(body, false)?;
                Ok(!has_break(body))
            }
            Stmt::For { var, range, range_span, body, els } => {
                for a in range {
                    self.expect(a, &Ty::I64)?;
                }
                self.scopes.push(HashMap::new());
                self.declare(var, VarInfo { ty: Ty::I64, mutable: false, decl: *range_span });
                let r = self.block(body, false);
                self.scopes.pop();
                r?;
                if let Some(els) = els {
                    self.block(els, false)?;
                }
                Ok(false)
            }
            Stmt::Break(_) | Stmt::Continue(_) => Ok(true),
            Stmt::Return { value, span } => {
                let ret = self.ret.clone();
                match (value, &ret) {
                    (None, Ty::Unit) => {}
                    (None, _) => {
                        return Err(Diagnostic::new("return に値がありません")
                            .primary(*span, format!("この関数は `{}` を返します", ret.show())))
                    }
                    (Some(e), Ty::Unit) => {
                        return Err(Diagnostic::new("値を返さない関数で値を return しています")
                            .primary(e.span, "この値の行き先がない")
                            .note("値を返すなら `fn i64 f()` のように戻り値の型を書いてください"))
                    }
                    (Some(e), _) => {
                        let got = self.expr(e)?;
                        if !assignable(&got, &ret) {
                            return Err(mismatch(&ret, &got, e.span));
                        }
                    }
                }
                Ok(true)
            }
        }
    }

    fn expect(&mut self, e: &Expr, want: &Ty) -> R<()> {
        let got = self.expr(e)?;
        if assignable(&got, want) {
            Ok(())
        } else {
            Err(mismatch(want, &got, e.span))
        }
    }

    fn expr(&mut self, e: &Expr) -> R<Ty> {
        self.expr_ctx(e, true)
    }

    /// 式の型。`want_value` が false なら、if / match は文として使われている（値を捨てる）
    fn expr_ctx(&mut self, e: &Expr, want_value: bool) -> R<Ty> {
        match &e.kind {
            ExprKind::Int(_) => Ok(Ty::I64),
            ExprKind::Bool(_) => Ok(Ty::Bool),
            ExprKind::Str(_) => Ok(Ty::Str),
            ExprKind::Var(name) => match self.lookup(name) {
                Some(v) => Ok(v.ty.clone()),
                None => Err(Diagnostic::new(format!("未定義の変数 `{}`", name))
                    .primary(e.span, "この名前は見つかりません")),
            },
            ExprKind::Own(inner) => {
                let t = self.value(inner)?;
                Ok(Ty::Own(Box::new(t)))
            }
            ExprKind::Ref(inner, mutable) => {
                if !is_place(inner) {
                    return Err(Diagnostic::new("借用できない式です")
                        .primary(inner.span, "`ref` は変数か `*p` か `p.f` にだけ使えます")
                        .note("`let x = own ...;` で変数に入れてから `ref x` としてください"));
                }
                let t = self.expr(inner)?;
                let Some(target) = t.pointee() else {
                    return Err(Diagnostic::new("借用できない値です")
                        .primary(inner.span, format!("`{}` は own でも ref でもありません", t.show()))
                        .note("値を借りたいときは、まず `own` に入れてください"));
                };
                let target = target.clone();
                if *mutable {
                    // own を ref mut で借りるには持ち主が mut、ref から借り直すにはその ref が ref mut
                    let origin = self.origin(inner);
                    self.access(inner)?.through(&t, origin, describe(inner)).require(e.span, "ref mut で借りようとした")?;
                }
                Ok(Ty::Ref(Box::new(target), *mutable))
            }
            ExprKind::Deref(inner, _) => {
                let t = self.expr(inner)?;
                match t.pointee() {
                    Some(x) => Ok(x.clone()),
                    None => Err(Diagnostic::new("`*` でたどれるのは own と ref だけです")
                        .primary(inner.span, format!("ここは `{}` です", t.show()))),
                }
            }
            ExprKind::Field(base, name, _) => {
                let t = self.expr(base)?;
                let sname = match &t {
                    Ty::Struct(n) => n,
                    Ty::Own(x) | Ty::Ref(x, _) => match &**x {
                        Ty::Struct(n) => n,
                        _ => return Err(not_a_struct(&t, base.span)),
                    },
                    _ => return Err(not_a_struct(&t, base.span)),
                };
                let (def, tys) = &self.structs[sname.as_str()];
                match def.field_index(name) {
                    Some(i) => Ok(tys[i].clone()),
                    None => Err(no_field(def, name, e.span)),
                }
            }
            ExprKind::StructLit(name, inits) => {
                let Some((def, tys)) = self.structs.get(name.as_str()) else {
                    return Err(Diagnostic::new(format!("未定義の構造体 `{}`", name))
                        .primary(e.span, "この構造体は定義されていません"));
                };
                let (def, tys) = (*def, tys.clone());
                let mut seen = vec![false; def.fields.len()];
                for init in inits {
                    let Some(i) = def.field_index(&init.name) else {
                        return Err(no_field(def, &init.name, init.span));
                    };
                    if seen[i] {
                        return Err(Diagnostic::new(format!("フィールド `{}` が二回書かれています", init.name))
                            .primary(init.span, "ここが二回目"));
                    }
                    seen[i] = true;
                    self.expect(&init.value, &tys[i])?;
                }
                let missing: Vec<&str> =
                    def.fields.iter().zip(&seen).filter(|(_, s)| !**s).map(|(n, _)| n.as_str()).collect();
                if !missing.is_empty() {
                    return Err(Diagnostic::new(format!("`{}` のフィールドが足りません", name))
                        .primary(e.span, format!("{} がありません", missing.join(", "))));
                }
                Ok(Ty::Struct(name.clone()))
            }
            ExprKind::If { cond, then, els } => {
                self.expect(cond, &Ty::Bool)?;
                let mut tys = vec![(self.block(then, want_value)?, tail_span(then))];
                match els {
                    Some(els) => tys.push((self.block(els, want_value)?, tail_span(els))),
                    None => tys.push((Ty::Unit, e.span)),
                }
                self.join(tys, want_value, "if")
            }
            ExprKind::Match { scrut, arms, default } => {
                let st = self.expr(scrut)?;
                if !matches!(st, Ty::I64 | Ty::Bool | Ty::Str) {
                    return Err(Diagnostic::new("match できるのは、今のところ整数・真偽値・文字列だけです")
                        .primary(scrut.span, format!("ここは `{}` です", st.show())));
                }
                let mut tys = Vec::new();
                for arm in arms {
                    for (pat, span) in &arm.pats {
                        let pt = match pat {
                            Pat::Int(_) => Ty::I64,
                            Pat::Bool(_) => Ty::Bool,
                            Pat::Str(_) => Ty::Str,
                        };
                        if pt != st {
                            return Err(Diagnostic::new("case の値と match する値の型が合いません")
                                .label(scrut.span, format!("これは `{}`", st.show()))
                                .primary(*span, format!("この case は `{}`", pt.show())));
                        }
                    }
                    tys.push((self.block(&arm.body, want_value)?, tail_span(&arm.body)));
                }
                match default {
                    Some(d) => tys.push((self.block(d, want_value)?, tail_span(d))),
                    None if covers_both_bools(arms) => {}
                    // どの case にも当たらなければ何もせず抜ける
                    None => tys.push((Ty::Unit, e.span)),
                }
                self.join(tys, want_value, "match")
            }
            ExprKind::Unary(op, inner) => {
                let want = if *op == UnOp::Neg { Ty::I64 } else { Ty::Bool };
                self.expect(inner, &want)?;
                Ok(want)
            }
            ExprKind::Binary(op, l, r) => self.binary(*op, l, r, e.span),
            ExprKind::Call(name, args) => self.call(name, args, e.span),
        }
    }

    /// 値として使う式。値なしならエラー
    fn value(&mut self, e: &Expr) -> R<Ty> {
        let t = self.expr(e)?;
        if t == Ty::Unit {
            return Err(Diagnostic::new("値のない式です").primary(e.span, "これは値を返さない"));
        }
        Ok(t)
    }

    /// if / match の枝の型をまとめる
    fn join(&self, branches: Vec<(Ty, Span)>, want_value: bool, what: &str) -> R<Ty> {
        if !want_value {
            // 文として使うときは値を捨てる。どの枝も必ず抜けるなら、ここで必ず抜ける
            let all_never = branches.iter().all(|(t, _)| *t == Ty::Never);
            return Ok(if all_never { Ty::Never } else { Ty::Unit });
        }
        let mut acc = Ty::Never;
        let mut first: Option<(Ty, Span)> = None;
        for (t, span) in branches {
            if assignable(&t, &acc) {
                continue;
            }
            if assignable(&acc, &t) {
                acc = t.clone();
            } else {
                let (ft, fs) = first.clone().expect("acc came from an earlier branch");
                return Err(Diagnostic::new(format!("{} の枝で値の型が違います", what))
                    .label(fs, format!("こちらは `{}`", ft.show()))
                    .primary(span, format!("こちらは `{}`", t.show())));
            }
            if first.is_none() || t != Ty::Never {
                first = Some((t, span));
            }
        }
        Ok(acc)
    }

    fn binary(&mut self, op: BinOp, l: &Expr, r: &Expr, span: Span) -> R<Ty> {
        let lt = self.expr(l)?;
        let rt = self.expr(r)?;
        let bad = || {
            Diagnostic::new("型が合わない演算です")
                .primary(span, format!("`{}` と `{}` にはこの演算を使えません", lt.show(), rt.show()))
        };
        match op {
            BinOp::And | BinOp::Or => {
                if lt != Ty::Bool {
                    return Err(mismatch(&Ty::Bool, &lt, l.span));
                }
                if rt != Ty::Bool {
                    return Err(mismatch(&Ty::Bool, &rt, r.span));
                }
                Ok(Ty::Bool)
            }
            BinOp::Add if lt == Ty::Str && rt == Ty::Str => Ok(Ty::Str),
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
                if lt == Ty::I64 && rt == Ty::I64 {
                    Ok(Ty::I64)
                } else {
                    Err(bad())
                }
            }
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                if lt == Ty::I64 && rt == Ty::I64 {
                    Ok(Ty::Bool)
                } else {
                    Err(bad())
                }
            }
            BinOp::Eq | BinOp::Ne => {
                if lt == rt && matches!(lt, Ty::I64 | Ty::Bool | Ty::Str) {
                    Ok(Ty::Bool)
                } else {
                    Err(Diagnostic::new("型が合わない比較です")
                        .primary(span, format!("`{}` と `{}` は比較できません", lt.show(), rt.show())))
                }
            }
        }
    }

    fn call(&mut self, name: &str, args: &[Expr], span: Span) -> R<Ty> {
        match name {
            "print" => {
                for a in args {
                    self.value(a)?;
                }
                return Ok(Ty::Unit);
            }
            "drop" | "assert" => {
                if args.len() != 1 {
                    return Err(arity_error(name, 1, args.len(), span));
                }
                if name == "drop" {
                    self.value(&args[0])?;
                } else {
                    self.expect(&args[0], &Ty::Bool)?;
                }
                return Ok(Ty::Unit);
            }
            _ => {}
        }
        let Some(sig) = self.fns.get(name) else {
            let d = Diagnostic::new(format!("未定義の関数 `{}`", name)).primary(span, "この関数は定義されていません");
            return Err(if name == "range" { d.note("range(...) は `for i in range(...)` の中でだけ使えます") } else { d });
        };
        if args.len() != sig.params.len() {
            return Err(arity_error(name, sig.params.len(), args.len(), span));
        }
        let (params, ret) = (sig.params.clone(), sig.ret.clone());
        for (a, want) in args.iter().zip(&params) {
            self.expect(a, want)?;
        }
        Ok(ret)
    }

    /// 場所 `e` に書き込めるか
    fn access(&mut self, e: &Expr) -> R<Access> {
        Ok(match &e.kind {
            ExprKind::Var(name) => {
                let v = self.lookup(name).expect("typed before");
                if v.mutable {
                    Access::Write
                } else {
                    Access::ImmutVar { name: name.clone(), decl: v.decl }
                }
            }
            ExprKind::Deref(inner, _) => {
                let outer = if is_place(inner) { self.access(inner)? } else { Access::Write };
                let t = self.expr(inner)?;
                outer.through(&t, self.origin(inner), describe(inner))
            }
            ExprKind::Field(base, ..) => {
                let a = self.access(base)?;
                let t = self.expr(base)?;
                a.through(&t, self.origin(base), describe(base))
            }
            _ => Access::Write,
        })
    }

    /// エラーで「この ref はどこから来たか」として指す位置。変数なら宣言
    fn origin(&self, e: &Expr) -> Span {
        match &e.kind {
            ExprKind::Var(n) => self.lookup(n).map_or(e.span, |v| v.decl),
            _ => e.span,
        }
    }
}

/// ループ本体に、このループを抜ける break があるか（内側のループの break は数えない）
fn has_break(b: &Block) -> bool {
    b.stmts.iter().any(|s| match s {
        Stmt::Break(_) => true,
        Stmt::Expr(e) => match &e.kind {
            ExprKind::If { then, els, .. } => has_break(then) || matches!(els, Some(b) if has_break(b)),
            ExprKind::Match { arms, default, .. } => {
                arms.iter().any(|a| has_break(&a.body)) || matches!(default, Some(b) if has_break(b))
            }
            _ => false,
        },
        Stmt::For { els: Some(els), .. } => has_break(els),
        _ => false,
    })
}

fn tail_span(b: &Block) -> Span {
    match b.stmts.last() {
        Some(Stmt::Expr(e)) if b.tail => e.span,
        _ => b.end,
    }
}

fn is_place(e: &Expr) -> bool {
    matches!(e.kind, ExprKind::Var(_) | ExprKind::Deref(..) | ExprKind::Field(..))
}

fn describe(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Var(n) => n.clone(),
        ExprKind::Deref(inner, _) => format!("*{}", describe(inner)),
        ExprKind::Field(base, name, _) => format!("{}.{}", describe(base), name),
        _ => "式".to_string(),
    }
}

fn covers_both_bools(arms: &[MatchArm]) -> bool {
    let has = |b| arms.iter().any(|a| a.pats.iter().any(|(p, _)| *p == Pat::Bool(b)));
    has(true) && has(false)
}

fn not_a_struct(t: &Ty, span: Span) -> Diagnostic {
    Diagnostic::new("フィールドを持たない値です").primary(span, format!("`{}` は構造体ではありません", t.show()))
}

fn no_field(def: &StructDef, name: &str, span: Span) -> Diagnostic {
    Diagnostic::new(format!("`{}` にフィールド `{}` はありません", def.name, name))
        .primary(span, "このフィールドは定義されていません")
        .note(format!("`{}` のフィールド: {}", def.name, def.fields.join(", ")))
}

fn arity_error(name: &str, want: usize, got: usize, span: Span) -> Diagnostic {
    Diagnostic::new(format!("`{}` の引数の数が違います", name))
        .primary(span, format!("{} 個必要ですが {} 個渡されました", want, got))
}
