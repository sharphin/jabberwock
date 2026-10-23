//! 構文木から IR への書き出し（lowering）。
//!
//! 今のインタプリタ（`interp.rs`）と同じ動きになるよう、評価の順・ムーブする／しない・解放する時と場所をそのまま写す。
//! - 式の値はいつも一時的な変数に入れる（あとの式が変数を書き換えても、読んだ時点の値が使われるように）。
//! - 一時的な変数はちょうど 1 回 `move` で読む（読み終わると空になり、解放し忘れが起きない）。
//! - スコープを抜ける道（終わり・break・continue・return）ごとに、生きている変数の `drop` を内側から順に出す。
//!
//! まだ書き出せない書き方（enum・match・リスト・クロージャ・メソッドなど）があれば `Err(理由)` を返す。

use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use super::*;
use crate::ast::{self, Expr, ExprKind, FnDef, InterpPart, Stmt, TypeKind};
use crate::interp::{describe, BUILTINS};
use crate::types::{self, TypeInfo};

/// 書き出せなかった理由
type L<T> = Result<T, String>;

/// main から呼ばれる関数だけを書き出す
pub fn lower(prog: &ast::Program, info: &TypeInfo) -> L<Program> {
    let mut structs = HashMap::new();
    for s in &prog.structs {
        if structs.insert(s.name.clone(), Rc::new(s.clone())).is_some() {
            return Err(format!("構造体 `{}` が二重に定義されている", s.name));
        }
    }
    let mut defs: HashMap<&str, &FnDef> = HashMap::new();
    for f in &prog.fns {
        if BUILTINS.contains(&f.name.as_str()) || defs.insert(&f.name, f).is_some() {
            return Err(format!("関数 `{}` の名前が使えない", f.name));
        }
    }
    if !defs.contains_key("main") {
        return Err("main がない".to_string());
    }
    let mut calls = Calls { defs, by_name: HashMap::new(), queue: VecDeque::new() };
    calls.index("main", &HashMap::new())?;
    let mut fns: Vec<Option<Func>> = Vec::new();
    while let Some((name, def, map, i)) = calls.queue.pop_front() {
        let f = FnLower::new(info, &structs, &mut calls, map).lower(def, name)?;
        if fns.len() <= i {
            fns.resize_with(i + 1, || None);
        }
        fns[i] = Some(f);
    }
    let fns = fns.into_iter().map(|f| f.expect("every queued function is lowered")).collect();
    Ok(Program { fns, by_name: calls.by_name, structs })
}

/// 呼ばれた関数に番号をつけ、書き出す順に並べる。ジェネリックな関数は型の引数ごとに作り分ける（`pick<i64>`）
struct Calls<'p> {
    defs: HashMap<&'p str, &'p FnDef>,
    by_name: HashMap<String, usize>,
    /// (作り分けた名前, 定義, 型の引数, 番号)
    queue: VecDeque<(String, &'p FnDef, HashMap<String, Ty>, usize)>,
}

impl<'p> Calls<'p> {
    /// `name` を型の引数 `targs`（具体的な型）で呼ぶときの関数の番号
    fn index(&mut self, name: &str, targs: &HashMap<String, Ty>) -> L<usize> {
        let def = *self.defs.get(name).ok_or_else(|| format!("関数 `{}` がない", name))?;
        let mut args = Vec::new();
        for p in &def.tparams {
            let t = targs.get(p).ok_or_else(|| format!("`{}` の型の引数 `{}` が分からない", name, p))?;
            if contains_param(t) {
                return Err(format!("`{}` の型の引数が決まらない", name));
            }
            args.push(t.clone());
        }
        let inst = if args.is_empty() {
            name.to_string()
        } else {
            format!("{}<{}>", name, args.iter().map(Ty::show).collect::<Vec<_>>().join(", "))
        };
        if let Some(&i) = self.by_name.get(&inst) {
            return Ok(i);
        }
        let i = self.by_name.len();
        self.by_name.insert(inst.clone(), i);
        let map = def.tparams.iter().cloned().zip(args).collect();
        self.queue.push_back((inst, def, map, i));
        Ok(i)
    }
}

fn contains_param(t: &Ty) -> bool {
    match t {
        Ty::Param(_) => true,
        Ty::Struct(_, a) | Ty::Enum(_, a) => a.iter().any(contains_param),
        Ty::Own(x) | Ty::Ref(x, _) | Ty::Array(x, _) | Ty::List(x) => contains_param(x),
        Ty::Map(k, v) => contains_param(k) || contains_param(v),
        Ty::Fn(ps, r) => ps.iter().any(contains_param) || contains_param(r),
        _ => false,
    }
}

struct Loop {
    /// break で飛ぶ先と、そのとき残すスコープの数（それより内側を drop する）
    brk: Bb,
    brk_depth: usize,
    /// continue で飛ぶ先と、残すスコープの数
    cont: Bb,
    cont_depth: usize,
}

struct FnLower<'a, 'p> {
    info: &'a TypeInfo,
    structs: &'a HashMap<String, Rc<ast::StructDef>>,
    calls: &'a mut Calls<'p>,
    /// 作り分けている関数の型の引数（`T` → 具体的な型）
    subst: HashMap<String, Ty>,
    locals: Vec<LocalDecl>,
    blocks: Vec<Block>,
    cur: Bb,
    /// スコープごとの変数（名前, 番号）。名前が空なのは書き出しのために置いた隠れた変数
    scopes: Vec<Vec<(String, Local)>>,
    loops: Vec<Loop>,
}

impl<'a, 'p> FnLower<'a, 'p> {
    fn new(
        info: &'a TypeInfo,
        structs: &'a HashMap<String, Rc<ast::StructDef>>,
        calls: &'a mut Calls<'p>,
        subst: HashMap<String, Ty>,
    ) -> Self {
        FnLower {
            info,
            structs,
            calls,
            subst,
            locals: Vec::new(),
            blocks: vec![Block { insts: Vec::new(), term: Term::Unreachable }],
            cur: 0,
            scopes: Vec::new(),
            loops: Vec::new(),
        }
    }

    fn lower(mut self, f: &FnDef, name: String) -> L<Func> {
        self.scopes.push(Vec::new());
        for p in &f.params {
            let ty = self.var_ty(p.span)?;
            self.declare(&p.name, ty, p.mutable, p.span);
        }
        let ret_ty = types::subst(&self.info.fn_ret.get(&f.name).cloned().unwrap_or(Ty::Unit), &self.subst);
        let ret = if f.ret.is_some() {
            let r = self.temp(ret_ty.clone(), f.body.end);
            self.block(&f.body, Some(r))?;
            Operand::Move(r, f.body.end)
        } else {
            self.block(&f.body, None)?;
            Operand::Const(Const::Unit)
        };
        self.leave(0, f.body.end, "スコープを抜けた");
        self.terminate(Term::Return(ret));
        Ok(Func {
            name,
            params: f.params.len(),
            ret: ret_ty,
            locals: self.locals,
            blocks: reachable(self.blocks),
            span: f.span,
        })
    }

    // ------------------------------------------------------------ 道具

    fn var_ty(&self, decl: Span) -> L<Ty> {
        let t = self.info.var_ty.get(&decl).ok_or_else(|| "変数の型が記録されていない".to_string())?;
        Ok(types::subst(t, &self.subst))
    }

    /// 式の型（作り分けている型の引数は具体的な型に置き換える）
    fn ty(&self, e: &Expr) -> Ty {
        self.info.ty(e).map_or(Ty::Unit, |t| types::subst(t, &self.subst))
    }

    /// 呼び出しの式で型検査が決めた型の引数（具体的な型に置き換えたもの）
    fn targs(&self, e: &Expr) -> HashMap<String, Ty> {
        let m = self.info.targs.get(&(e as *const Expr)).cloned().unwrap_or_default();
        m.into_iter().map(|(k, v)| (k, types::subst(&v, &self.subst))).collect()
    }

    fn new_local(&mut self, name: &str, ty: Ty, mutable: bool, span: Span) -> Local {
        self.locals.push(LocalDecl { name: name.to_string(), ty, mutable, span });
        self.locals.len() - 1
    }

    fn temp(&mut self, ty: Ty, span: Span) -> Local {
        self.new_local("", ty, true, span)
    }

    /// 今のスコープに変数を置く（値は `Assign` で入れる）
    fn declare(&mut self, name: &str, ty: Ty, mutable: bool, span: Span) -> Local {
        let l = self.new_local(name, ty, mutable, span);
        self.scopes.last_mut().expect("scope").push((name.to_string(), l));
        l
    }

    fn lookup(&self, name: &str) -> Option<Local> {
        self.scopes.iter().rev().flat_map(|s| s.iter().rev()).find(|(n, _)| n == name).map(|(_, l)| *l)
    }

    fn emit(&mut self, i: Inst) {
        self.blocks[self.cur].insts.push(i);
    }

    fn assign(&mut self, l: Local, value: Rvalue, span: Span) {
        self.emit(Inst::Assign { place: Place::local(l, span), value, span });
    }

    /// 値を新しい一時的な変数に入れ、それを読む operand を返す
    fn temp_of(&mut self, ty: Ty, value: Rvalue, span: Span) -> Operand {
        if let Rvalue::Use(o @ Operand::Const(_)) = value {
            return o;
        }
        let t = self.temp(ty, span);
        self.assign(t, value, span);
        Operand::Move(t, span)
    }

    fn new_block(&mut self) -> Bb {
        self.blocks.push(Block { insts: Vec::new(), term: Term::Unreachable });
        self.blocks.len() - 1
    }

    /// 今のブロックを終わらせる。続きは、どこからも飛んでこない新しいブロックに書く
    fn terminate(&mut self, t: Term) {
        self.blocks[self.cur].term = t;
        self.cur = self.new_block();
    }

    fn goto_and_switch(&mut self, to: Bb) {
        self.blocks[self.cur].term = Term::Goto(to);
        self.cur = to;
    }

    /// スコープ `depth` 個を残して、それより内側の変数を内側から drop する（スコープは閉じない）
    fn leave(&mut self, depth: usize, span: Span, how: &str) {
        let mut drops = Vec::new();
        for scope in self.scopes[depth..].iter().rev() {
            for (name, l) in scope.iter().rev() {
                drops.push(Inst::Drop { local: *l, span, why: format!("`{}` は{}ので解放された", name, how) });
            }
        }
        for d in drops {
            self.emit(d);
        }
    }

    /// 一番内側のスコープを、普通に抜けたとして閉じる
    fn pop_scope(&mut self, span: Span, how: &str) {
        let depth = self.scopes.len() - 1;
        self.leave(depth, span, how);
        self.scopes.pop();
    }

    // ------------------------------------------------------------ 文

    /// ブロックを書き出す。`dest` があれば、最後の `;` のない式の値をそこに入れる（なければ ()）
    fn block(&mut self, b: &ast::Block, dest: Option<Local>) -> L<()> {
        self.scopes.push(Vec::new());
        let mut wrote = false;
        for (i, s) in b.stmts.iter().enumerate() {
            match (s, dest) {
                (Stmt::Expr(e), Some(d)) if b.tail && i + 1 == b.stmts.len() => {
                    let v = self.expr(e)?;
                    self.assign(d, Rvalue::Use(v), e.span);
                    wrote = true;
                }
                _ => self.stmt(s)?,
            }
        }
        if let (Some(d), false) = (dest, wrote) {
            self.assign(d, Rvalue::Use(Operand::Const(Const::Unit)), b.end);
        }
        self.pop_scope(b.end, "スコープを抜けた");
        Ok(())
    }

    fn stmt(&mut self, s: &Stmt) -> L<()> {
        match s {
            Stmt::Let { name, mutable, span, init, .. } => {
                let v = self.expr(init)?;
                let ty = self.var_ty(*span)?;
                let l = self.declare(name, ty, *mutable, *span);
                self.assign(l, Rvalue::Use(v), *span);
            }
            Stmt::Assign { target, value, span } => {
                // 値を先に計算し、そのあとで場所を求める（interp の assign と同じ順）
                let v = self.expr(value)?;
                let place = self.place(target)?;
                self.emit(Inst::Assign { place, value: Rvalue::Use(v), span: *span });
            }
            Stmt::Expr(e) => match &e.kind {
                ExprKind::If { cond, then, els } => self.if_expr(cond, then, els.as_ref(), None)?,
                _ => {
                    let v = self.expr(e)?;
                    if !matches!(v, Operand::Const(_)) {
                        let why = "どこにも束縛されず、そのまま捨てられた".to_string();
                        self.emit(Inst::Discard { value: v, span: e.span, why });
                    }
                }
            },
            Stmt::While { cond, body } => {
                let head = self.new_block();
                let body_bb = self.new_block();
                let exit = self.new_block();
                self.goto_and_switch(head);
                let c = self.expr(cond)?;
                self.blocks[self.cur].term = Term::Branch(c, body_bb, exit);
                self.cur = body_bb;
                let depth = self.scopes.len();
                self.loops.push(Loop { brk: exit, brk_depth: depth, cont: head, cont_depth: depth });
                self.block(body, None)?;
                self.loops.pop();
                self.blocks[self.cur].term = Term::Goto(head);
                self.cur = exit;
            }
            Stmt::Loop { body } => {
                let head = self.new_block();
                let exit = self.new_block();
                self.goto_and_switch(head);
                let depth = self.scopes.len();
                self.loops.push(Loop { brk: exit, brk_depth: depth, cont: head, cont_depth: depth });
                self.block(body, None)?;
                self.loops.pop();
                self.blocks[self.cur].term = Term::Goto(head);
                self.cur = exit;
            }
            Stmt::For { var, range, range_span, body, els } => self.for_range(var, range, *range_span, body, els.as_ref())?,
            Stmt::ForEach { .. } => return Err("for x in a".to_string()),
            Stmt::Break(sp) => {
                let lp = self.loops.last().expect("parser rejects break outside loops");
                let (to, depth) = (lp.brk, lp.brk_depth);
                self.leave(depth, *sp, "break でループを抜けた");
                self.terminate(Term::Goto(to));
            }
            Stmt::Continue(sp) => {
                let lp = self.loops.last().expect("parser rejects continue outside loops");
                let (to, depth) = (lp.cont, lp.cont_depth);
                self.leave(depth, *sp, "continue で次の周回へ進んだ");
                self.terminate(Term::Goto(to));
            }
            Stmt::Return { value, span } => {
                let v = match value {
                    Some(e) => self.expr(e)?,
                    None => Operand::Const(Const::Unit),
                };
                self.leave(0, *span, "関数から return した");
                self.terminate(Term::Return(v));
            }
        }
        Ok(())
    }

    /// `for i in range(..)`。interp と同じく、範囲の値を先にすべて計算し、刻みが 0 ならエラー、
    /// 型の範囲を超えたら終わる。ループ変数は周回ごとの新しいスコープに置く
    fn for_range(&mut self, var: &str, range: &[Expr], range_span: Span, body: &ast::Block, els: Option<&ast::Block>) -> L<()> {
        // ループ変数の型は、終わりの値（1 つだけなら、その値）の型
        let ty = match self.int_ty(&range[range.len().min(2) - 1]) {
            Some(t) => t,
            None => return Err("range の型が分からない".to_string()),
        };
        let mut vals = Vec::new();
        for a in range {
            vals.push(self.expr(a)?);
        }
        let one = Operand::Const(Const::Int(1, IntTy::I64));
        let (start, end_v, step_v) = match vals.len() {
            1 => (Operand::Const(Const::Int(0, ty)), vals.remove(0), one),
            2 => {
                let s = vals.remove(0);
                (s, vals.remove(0), one)
            }
            _ => {
                let s = vals.remove(0);
                let e = vals.remove(0);
                (s, e, vals.remove(0))
            }
        };
        // 隠れた変数（ループの外へ出るとき、ほかの変数と同じく drop する）
        self.scopes.push(Vec::new());
        let hidden = self.scopes.len();
        let rd = |l: Local| Operand::Copy(Place::local(l, range_span));
        let counter = self.declare("", Ty::Int(ty), true, range_span);
        self.assign(counter, Rvalue::Cast(start, Ty::Int(ty)), range_span);
        let end = self.declare("", Ty::Int(ty), false, range_span);
        self.assign(end, Rvalue::Use(end_v), range_span);
        // 刻みが書いたままの数なら、向きはここで決まる
        let (step, sign) = match step_v {
            Operand::Const(Const::Int(n, _)) => (Operand::Const(Const::Int(n, IntTy::I64)), Some(n.signum())),
            v => {
                let l = self.declare("", Ty::Int(IntTy::I64), false, range_span);
                self.assign(l, Rvalue::Use(v), range_span);
                (rd(l), None)
            }
        };
        // 1 回でも回ったか（else があるときだけ）
        let ran = match els {
            Some(_) => {
                let l = self.declare("", Ty::Bool, true, range_span);
                self.assign(l, Rvalue::Use(Operand::Const(Const::Bool(false))), range_span);
                Some(l)
            }
            None => None,
        };
        let zero = Operand::Const(Const::Int(0, IntTy::I64));
        let fail_term = Term::Fail("range の刻みが 0 です".to_string(), range_span);

        let head = self.new_block();
        let up = self.new_block();
        let down = self.new_block();
        let body_bb = self.new_block();
        let next = self.new_block();
        let done = self.new_block();
        let exit = self.new_block();
        match sign {
            Some(0) => {
                self.blocks[self.cur].term = fail_term;
                self.cur = head;
            }
            Some(_) => self.goto_and_switch(head),
            None => {
                // 刻みが 0 なら終わらない
                let fail = self.new_block();
                let z = self.temp_of(Ty::Bool, Rvalue::Bin(BinOp::Eq, step.clone(), zero.clone(), range_span), range_span);
                self.blocks[self.cur].term = Term::Branch(z, fail, head);
                self.blocks[fail].term = fail_term;
                self.cur = head;
            }
        }
        match sign {
            Some(n) if n > 0 => self.goto_and_switch(up),
            Some(_) => self.goto_and_switch(down),
            None => {
                let pos = self.temp_of(Ty::Bool, Rvalue::Bin(BinOp::Gt, step.clone(), zero, range_span), range_span);
                self.blocks[self.cur].term = Term::Branch(pos, up, down);
                self.cur = up;
            }
        }
        if !matches!(sign, Some(n) if n < 0) {
            self.cur = up;
            let c = self.temp_of(Ty::Bool, Rvalue::Bin(BinOp::Lt, rd(counter), rd(end), range_span), range_span);
            self.blocks[self.cur].term = Term::Branch(c, body_bb, done);
        }
        if !matches!(sign, Some(n) if n > 0) {
            self.cur = down;
            let c = self.temp_of(Ty::Bool, Rvalue::Bin(BinOp::Gt, rd(counter), rd(end), range_span), range_span);
            self.blocks[self.cur].term = Term::Branch(c, body_bb, done);
        }

        self.cur = body_bb;
        if let Some(ran) = ran {
            self.assign(ran, Rvalue::Use(Operand::Const(Const::Bool(true))), range_span);
        }
        self.scopes.push(Vec::new());
        let vty = self.var_ty(range_span)?;
        let v = self.declare(var, vty, false, range_span);
        self.assign(v, Rvalue::Use(rd(counter)), range_span);
        self.loops.push(Loop { brk: exit, brk_depth: hidden, cont: next, cont_depth: hidden + 1 });
        self.block(body, None)?;
        self.loops.pop();
        self.goto_and_switch(next);
        self.pop_scope(range_span, "次の周回へ進んだ");
        self.blocks[self.cur].term =
            Term::Step { counter, step, next: head, done };
        // 普通に終わった: 1 回も回らなければ else
        self.cur = done;
        if let (Some(els), Some(ran)) = (els, ran) {
            let run_else = self.new_block();
            let r = self.temp_of(Ty::Bool, Rvalue::Use(rd(ran)), range_span);
            self.blocks[self.cur].term = Term::Branch(r, exit, run_else);
            self.cur = run_else;
            self.block(els, None)?;
        }
        self.goto_and_switch(exit);
        self.pop_scope(range_span, "ループを抜けた");
        Ok(())
    }

    /// `if`。`dest` があれば値をそこに入れる
    fn if_expr(&mut self, cond: &Expr, then: &ast::Block, els: Option<&ast::Block>, dest: Option<Local>) -> L<()> {
        let c = self.expr(cond)?;
        let t = self.new_block();
        let e = self.new_block();
        let join = self.new_block();
        self.blocks[self.cur].term = Term::Branch(c, t, e);
        self.cur = t;
        self.block(then, dest)?;
        self.blocks[self.cur].term = Term::Goto(join);
        self.cur = e;
        match els {
            Some(b) => self.block(b, dest)?,
            None => {
                if let Some(d) = dest {
                    self.assign(d, Rvalue::Use(Operand::Const(Const::Unit)), cond.span);
                }
            }
        }
        self.goto_and_switch(join);
        Ok(())
    }

    // ------------------------------------------------------------ 式

    /// 型検査が決めた「この整数の式を使う型」
    fn int_ty(&self, e: &Expr) -> Option<IntTy> {
        e.int.get().or(match self.info.ty(e) {
            Some(Ty::Int(t)) => Some(*t),
            _ => None,
        })
    }

    /// 値として評価する（interp の `eval`）。ムーブする値の変数はムーブする
    fn expr(&mut self, e: &Expr) -> L<Operand> {
        if e.borrow.get() {
            // `ref T` の引数に `ref` なしで渡した場所は自動で借りる（変換はしない）
            let place = self.place(e)?;
            let r = Rvalue::Ref { place, mutable: false, what: describe(e), span: e.span };
            return Ok(self.temp_of(self.ty(e), r, e.span));
        }
        let v = self.expr_kind(e)?;
        // 型検査がつけた変換の印（interp の `convert`）
        let target = if e.float.get() {
            Ty::Float
        } else {
            match e.int.get() {
                Some(t) => Ty::Int(t),
                None => return Ok(v),
            }
        };
        Ok(match v {
            Operand::Const(Const::Int(n, _)) => Operand::Const(match target {
                Ty::Float => Const::Float(n as f64),
                Ty::Int(t) => Const::Int(t.wrap(n), t),
                _ => unreachable!(),
            }),
            Operand::Const(c) => Operand::Const(c),
            v => self.temp_of(target.clone(), Rvalue::Cast(v, target), e.span),
        })
    }

    fn expr_kind(&mut self, e: &Expr) -> L<Operand> {
        let ty = self.ty(e);
        let sp = e.span;
        Ok(match &e.kind {
            ExprKind::Int(n) => Operand::Const(Const::Int(*n, IntTy::I64)),
            ExprKind::Float(x) => Operand::Const(Const::Float(*x)),
            ExprKind::Bool(b) => Operand::Const(Const::Bool(*b)),
            ExprKind::Str(s) => Operand::Const(Const::Str(Rc::from(s.as_str()))),
            ExprKind::Var(name) => {
                let l = self.lookup(name).ok_or_else(|| format!("関数を値として使う `{}`", name))?;
                let read = if moves(&self.locals[l].ty)? { Operand::Move(l, sp) } else { Operand::Copy(Place::local(l, sp)) };
                self.temp_of(ty, Rvalue::Use(read), sp)
            }
            ExprKind::Cast(inner, t) => {
                let target = match &t.kind {
                    TypeKind::Name(n) => match IntTy::from_name(n) {
                        Some(it) => Ty::Int(it),
                        None => Ty::Float,
                    },
                    _ => return Err("数でない型への as".to_string()),
                };
                let v = self.expr(inner)?;
                self.temp_of(ty, Rvalue::Cast(v, target), sp)
            }
            ExprKind::Own(inner) => {
                let v = self.expr(inner)?;
                self.temp_of(ty, Rvalue::Own(v), sp)
            }
            ExprKind::Ref(inner, mutable) => {
                let place = self.place(inner)?;
                self.temp_of(ty, Rvalue::Ref { place, mutable: *mutable, what: describe(inner), span: sp }, sp)
            }
            ExprKind::Deref(..) | ExprKind::Field(..) => {
                // ポインタの先・フィールドから取り出せるのはコピーできる値だけ（ムーブする値はまだ扱わない）
                if moves(&ty)? {
                    return Err("ポインタの先やフィールドからムーブする値を読む".to_string());
                }
                let place = self.place(e)?;
                self.temp_of(ty, Rvalue::Use(Operand::Copy(place)), sp)
            }
            ExprKind::StructLit(name, inits) => {
                let def = self.structs.get(name).cloned().ok_or_else(|| format!("構造体 `{}` がない", name))?;
                // 書いた順に評価し、定義の順に並べる
                let mut fields: Vec<Option<Operand>> = vec![None; def.fields.len()];
                for init in inits {
                    let i = def.field_index(&init.name).ok_or("ないフィールド")?;
                    if fields[i].is_some() {
                        return Err("フィールドが二回ある".to_string());
                    }
                    fields[i] = Some(self.expr(&init.value)?);
                }
                let fields = fields.into_iter().collect::<Option<Vec<_>>>().ok_or("フィールドが足りない")?;
                self.temp_of(ty, Rvalue::Struct(name.clone(), fields), sp)
            }
            ExprKind::If { cond, then, els } => {
                let d = self.temp(ty.clone(), sp);
                self.if_expr(cond, then, els.as_ref(), Some(d))?;
                Operand::Move(d, sp)
            }
            ExprKind::Unary(op, inner) => {
                let v = self.expr(inner)?;
                self.temp_of(ty, Rvalue::Un(*op, v, sp), sp)
            }
            ExprKind::Binary(op @ (BinOp::And | BinOp::Or), l, r) => {
                // 左だけで決まれば右は評価しない
                let d = self.temp(Ty::Bool, sp);
                let lv = self.expr(l)?;
                self.assign(d, Rvalue::Use(lv), l.span);
                let right = self.new_block();
                let join = self.new_block();
                let c = Operand::Copy(Place::local(d, sp));
                self.blocks[self.cur].term =
                    if *op == BinOp::And { Term::Branch(c, right, join) } else { Term::Branch(c, join, right) };
                self.cur = right;
                let rv = self.expr(r)?;
                self.assign(d, Rvalue::Use(rv), r.span);
                self.goto_and_switch(join);
                Operand::Move(d, sp)
            }
            ExprKind::Binary(op, l, r) => {
                let lv = self.expr(l)?;
                let rv = self.expr(r)?;
                self.temp_of(ty, Rvalue::Bin(*op, lv, rv, sp), sp)
            }
            ExprKind::Call(name, args) => return self.call(e, name, args, ty),
            ExprKind::MethodCall(recv, name, args) => return self.method(e, recv, name, args, ty),
            ExprKind::Interp(parts) => {
                let mut strs = Vec::new();
                for p in parts {
                    match p {
                        InterpPart::Lit(s) => strs.push(Operand::Const(Const::Str(Rc::from(s.as_str())))),
                        InterpPart::Expr(x) => strs.push(self.show(x)?),
                    }
                }
                self.temp_of(Ty::Str, Rvalue::Builtin("concat".to_string(), strs, sp), sp)
            }
            ExprKind::Try(_) => return Err("?".to_string()),
            ExprKind::Variant { .. } | ExprKind::Match { .. } => return Err("enum / match".to_string()),
            ExprKind::Index(..) | ExprKind::ArrayLit(..) | ExprKind::ArrayRepeat(..) | ExprKind::MapLit(_) => {
                return Err("配列・リスト・辞書".to_string())
            }
            ExprKind::Lambda(_) => return Err("クロージャ".to_string()),
        })
    }

    /// print と埋め込みの 1 つ分: 借りて読み（interp の `eval_borrow`）、文字にする。一時的な値は見たあと捨てる
    fn show(&mut self, e: &Expr) -> L<Operand> {
        let is_place_read = !e.borrow.get()
            && match &e.kind {
                ExprKind::Var(n) => self.lookup(n).is_some(),
                ExprKind::Deref(inner, _) | ExprKind::Field(inner, ..) => is_place(inner),
                _ => false,
            };
        let r = if is_place_read {
            Rvalue::Show(Operand::Copy(self.place(e)?), false, e.span)
        } else {
            match self.expr(e)? {
                // 書いたままの文字列はそのまま
                s @ Operand::Const(Const::Str(_)) => return Ok(s),
                v => Rvalue::Show(v, true, e.span),
            }
        };
        Ok(self.temp_of(Ty::Str, r, e.span))
    }

    fn call(&mut self, e: &Expr, name: &str, args: &[Expr], ty: Ty) -> L<Operand> {
        let sp = e.span;
        match name {
            "print" => {
                let mut strs = Vec::new();
                for a in args {
                    strs.push(self.show(a)?);
                }
                Ok(self.temp_of(Ty::Unit, Rvalue::Builtin("print".to_string(), strs, sp), sp))
            }
            "assert" => {
                let v = self.expr(&args[0])?;
                Ok(self.temp_of(Ty::Unit, Rvalue::Builtin("assert".to_string(), vec![v], args[0].span), sp))
            }
            "drop" => {
                let v = self.expr(&args[0])?;
                let why = match &args[0].kind {
                    ExprKind::Var(n) => format!("`{}` が drop で明示的に解放された", n),
                    _ => "drop で明示的に解放された".to_string(),
                };
                self.emit(Inst::Discard { value: v, span: sp, why });
                Ok(Operand::Const(Const::Unit))
            }
            "read_line" | "args" => Err("入力".to_string()),
            _ if self.lookup(name).is_some() => Err("関数の値の呼び出し".to_string()),
            _ => {
                let mut vals = Vec::new();
                for a in args {
                    vals.push(self.expr(a)?);
                }
                let f = self.calls.index(name, &self.targs(e))?;
                Ok(self.temp_of(ty, Rvalue::Call(f, vals, sp), sp))
            }
        }
    }

    /// `recv.name(args)`（interp の `eval_method`）。一時的な receiver は先に評価し、場所の receiver は引数のあとで借りる
    fn method(&mut self, e: &Expr, recv: &Expr, name: &str, args: &[Expr], ty: Ty) -> L<Operand> {
        let sp = e.span;
        let rt = self.ty(recv);
        let (inner, ptr) = match &rt {
            Ty::Own(x) | Ty::Ref(x, _) => ((**x).clone(), true),
            t => (t.clone(), false),
        };
        let temp = if is_place(recv) { None } else { Some(self.expr(recv)?) };
        let mut vals = Vec::new();
        for a in args {
            vals.push(self.expr(a)?);
        }
        // receiver の場所。own / ref なら先をたどる（interp は種類を調べるため、いつも世代チェックをする）
        let recv_place = |me: &mut Self| -> L<Option<Place>> {
            let mut pl = match &temp {
                None => me.place(recv)?,
                Some(Operand::Move(t, _)) => Place::local(*t, recv.span),
                Some(_) => return Ok(None),
            };
            if ptr {
                pl.proj.push(Proj::Deref(ast::Check::Required, recv.span));
            }
            Ok(Some(pl))
        };
        match inner {
            Ty::Str => {
                let text = match recv_place(self)? {
                    Some(pl) => Operand::Copy(pl),
                    None => temp.clone().expect("a constant receiver"),
                };
                let mut ops = vec![text];
                ops.append(&mut vals);
                let r = self.temp_of(ty, Rvalue::Builtin(format!("str.{}", name), ops, sp), sp);
                self.drop_temp(&temp, sp);
                Ok(r)
            }
            Ty::Struct(sname, sargs) => {
                let full = format!("{}.{}", sname, name);
                let def = *self.calls.defs.get(full.as_str()).ok_or("ないメソッド")?;
                let mutable = matches!(def.params[0].ty.kind, TypeKind::Ref(_, true));
                // 型の引数: 型検査が決めたもの。interface の条件で呼んだ（`T` の）ときは、構造体の型の引数から
                let mut targs = self.targs(e);
                if let Some(sd) = self.structs.get(&sname) {
                    for (p, a) in sd.tparams.iter().zip(&sargs) {
                        targs.entry(p.clone()).or_insert_with(|| a.clone());
                    }
                }
                let pl = recv_place(self)?.ok_or("定数の receiver")?;
                let this_ty = Ty::Ref(Box::new(Ty::Struct(sname.clone(), sargs.clone())), mutable);
                let this = self.temp_of(this_ty, Rvalue::Ref { place: pl, mutable, what: describe(recv), span: recv.span }, sp);
                let f = self.calls.index(&full, &targs)?;
                let mut ops = vec![this];
                ops.append(&mut vals);
                let r = self.temp_of(ty, Rvalue::Call(f, ops, sp), sp);
                // 一時的な receiver は、呼び出しが終わったら捨てる
                self.drop_temp(&temp, sp);
                Ok(r)
            }
            _ => Err(format!("{} のメソッド", inner.show())),
        }
    }

    fn drop_temp(&mut self, temp: &Option<Operand>, span: Span) {
        if let Some(Operand::Move(t, _)) = temp {
            let why = "メソッドの呼び出しに使い終わった一時的な値".to_string();
            self.emit(Inst::Drop { local: *t, span, why });
        }
    }

    /// 場所（代入の左辺・`ref` の中身）。own / ref の先のフィールドは自動でたどる
    fn place(&mut self, e: &Expr) -> L<Place> {
        match &e.kind {
            ExprKind::Var(name) => {
                let l = self.lookup(name).ok_or_else(|| format!("場所でない名前 `{}`", name))?;
                Ok(Place::local(l, e.span))
            }
            ExprKind::Deref(inner, check) if is_place(inner) => {
                let mut p = self.place(inner)?;
                p.proj.push(Proj::Deref(check.get(), e.span));
                Ok(p)
            }
            ExprKind::Field(base, name, check) if is_place(base) => {
                let mut p = self.place(base)?;
                let sname = match self.info.ty(base) {
                    Some(Ty::Struct(n, _)) => n.clone(),
                    Some(Ty::Own(t) | Ty::Ref(t, _)) => match &**t {
                        Ty::Struct(n, _) => {
                            p.proj.push(Proj::Deref(check.get(), e.span));
                            n.clone()
                        }
                        _ => return Err("構造体でない先のフィールド".to_string()),
                    },
                    _ => return Err("構造体でない値のフィールド".to_string()),
                };
                let def = self.structs.get(&sname).ok_or("構造体がない")?;
                let i = def.field_index(name).ok_or("ないフィールド")?;
                p.proj.push(Proj::Field(i, e.span));
                Ok(p)
            }
            _ => Err("一時的な値の中の場所・配列の要素".to_string()),
        }
    }
}

/// どこからも飛んでこないブロック（return や break のあと）を取り除き、番号を詰める
fn reachable(blocks: Vec<Block>) -> Vec<Block> {
    fn succ(t: &Term) -> Vec<Bb> {
        match t {
            Term::Goto(b) => vec![*b],
            Term::Branch(_, a, b) | Term::Step { next: a, done: b, .. } => vec![*a, *b],
            Term::Return(_) | Term::Fail(..) | Term::Unreachable => Vec::new(),
        }
    }
    let mut seen = vec![false; blocks.len()];
    let mut work = vec![0];
    while let Some(b) = work.pop() {
        if !seen[b] {
            seen[b] = true;
            work.extend(succ(&blocks[b].term));
        }
    }
    let mut new_index = vec![usize::MAX; blocks.len()];
    let mut n = 0;
    for (i, s) in seen.iter().enumerate() {
        if *s {
            new_index[i] = n;
            n += 1;
        }
    }
    blocks
        .into_iter()
        .zip(seen)
        .filter(|(_, s)| *s)
        .map(|(mut b, _)| {
            b.term = match b.term {
                Term::Goto(x) => Term::Goto(new_index[x]),
                Term::Branch(c, x, y) => Term::Branch(c, new_index[x], new_index[y]),
                Term::Step { counter, step, next, done } => {
                    Term::Step { counter, step, next: new_index[next], done: new_index[done] }
                }
                t => t,
            };
            b
        })
        .collect()
}

fn is_place(e: &Expr) -> bool {
    matches!(e.kind, ExprKind::Var(_) | ExprKind::Deref(..) | ExprKind::Field(..))
}

/// 値として読むとムーブになる型か（interp の `Value::moves` を型で決めたもの）
fn moves(t: &Ty) -> L<bool> {
    Ok(match t {
        Ty::Int(_) | Ty::Float | Ty::Bool | Ty::Str | Ty::Unit | Ty::Never | Ty::Ref(..) => false,
        Ty::Own(_) | Ty::Struct(..) => true,
        _ => return Err(format!("型 {}", t.show())),
    })
}
