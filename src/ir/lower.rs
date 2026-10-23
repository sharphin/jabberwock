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
    while let Some((name, body, map, i)) = calls.queue.pop_front() {
        let fl = FnLower::new(info, &structs, &mut calls, map, name);
        let f = match body {
            Body::Fn(def) => fl.lower(def)?,
            Body::Lambda { lambda, caps, ret, span } => fl.lower_lambda(&lambda, caps, ret, span)?,
        };
        if fns.len() <= i {
            fns.resize_with(i + 1, || None);
        }
        fns[i] = Some(f);
    }
    let fns = fns.into_iter().map(|f| f.expect("every queued function is lowered")).collect();
    Ok(Program { fns, by_name: calls.by_name, structs })
}

/// 書き出す関数の本体
enum Body<'p> {
    Fn(&'p FnDef),
    /// クロージャ。取り込む変数（名前, 外での型）と戻り値の型
    Lambda { lambda: Rc<ast::Lambda>, caps: Vec<(String, Ty)>, ret: Ty, span: Span },
}

/// 呼ばれた関数に番号をつけ、書き出す順に並べる。ジェネリックな関数は型の引数ごとに作り分ける（`pick<i64>`）
struct Calls<'p> {
    defs: HashMap<&'p str, &'p FnDef>,
    by_name: HashMap<String, usize>,
    /// (作り分けた名前, 本体, 型の引数, 番号)
    queue: VecDeque<(String, Body<'p>, HashMap<String, Ty>, usize)>,
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
        let map = def.tparams.iter().cloned().zip(args).collect();
        Ok(self.push(inst, Body::Fn(def), map))
    }

    fn push(&mut self, inst: String, body: Body<'p>, map: HashMap<String, Ty>) -> usize {
        if let Some(&i) = self.by_name.get(&inst) {
            return i;
        }
        let i = self.by_name.len();
        self.by_name.insert(inst.clone(), i);
        self.queue.push_back((inst, body, map, i));
        i
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
    /// 書き出している関数の（作り分けた）名前
    name: String,
    /// クロージャが取り込んだ値の変数
    captures: Vec<Local>,
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
        name: String,
    ) -> Self {
        FnLower {
            info,
            structs,
            calls,
            subst,
            name,
            captures: Vec::new(),
            locals: Vec::new(),
            blocks: vec![Block { insts: Vec::new(), term: Term::Unreachable }],
            cur: 0,
            scopes: Vec::new(),
            loops: Vec::new(),
        }
    }

    fn lower(mut self, f: &FnDef) -> L<Func> {
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
            name: self.name,
            params: f.params.len(),
            captures: 0,
            ret: ret_ty,
            locals: self.locals,
            blocks: reachable(self.blocks),
            span: f.span,
        })
    }

    /// クロージャの本体。引数のあとに、取り込んだ値（読むだけの写し）を受け取る
    fn lower_lambda(mut self, l: &ast::Lambda, caps: Vec<(String, Ty)>, ret_ty: Ty, span: Span) -> L<Func> {
        self.scopes.push(Vec::new());
        for p in &l.params {
            let ty = self.var_ty(p.span)?;
            self.declare(&p.name, ty, false, p.span);
        }
        let ncaps = caps.len();
        for (n, t) in caps {
            let t = match t {
                Ty::Own(x) => Ty::Ref(x, false),
                t => t,
            };
            let v = self.declare(&n, t, false, span);
            self.captures.push(v);
        }
        let ret = if l.returns.get() {
            let r = self.temp(ret_ty.clone(), l.body.end);
            self.block(&l.body, Some(r))?;
            Operand::Move(r, l.body.end)
        } else {
            self.block(&l.body, None)?;
            Operand::Const(Const::Unit)
        };
        self.leave(0, l.body.end, "スコープを抜けた");
        self.terminate(Term::Return(ret));
        Ok(Func {
            name: self.name,
            params: l.params.len() + ncaps,
            captures: ncaps,
            ret: ret_ty,
            locals: self.locals,
            blocks: reachable(self.blocks),
            span,
        })
    }

    /// 変数を値として読む（interp の `read_var`）。関数の値と取り込んだ変数は、値しだいで決める
    fn read_var(&self, l: Local, span: Span) -> L<Operand> {
        let t = &self.locals[l].ty;
        Ok(if self.captures.contains(&l) || matches!(t, Ty::Fn(..)) {
            Operand::Read(l, span)
        } else if self.moves(t)? {
            Operand::Move(l, span)
        } else {
            Operand::Copy(Place::local(l, span))
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
                ExprKind::Match { scrut, arms, default } => self.match_expr(scrut, arms, default.as_ref(), None)?,
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
            Stmt::ForEach { key, var, var_span, iter, body, els } => {
                self.for_each(key.as_ref(), var, *var_span, iter, body, els.as_ref())?
            }
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
        let fail_term = Term::Fail("range の刻みが 0 です".to_string(), range_span, "これだと永遠に終わりません".to_string());

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

    /// `for x in a` / `for (k, v) in m`（interp の `exec_for_each`）。a は借りて読み、始めた時点の要素の写しを回す
    fn for_each(
        &mut self,
        key: Option<&(String, Span)>,
        var: &str,
        var_span: Span,
        iter: &Expr,
        body: &ast::Block,
        els: Option<&ast::Block>,
    ) -> L<()> {
        let it = self.ty(iter);
        let (cty, ptr) = match &it {
            Ty::Own(x) | Ty::Ref(x, _) => ((**x).clone(), true),
            t => (t.clone(), false),
        };
        let elem = match &cty {
            Ty::Array(x, _) | Ty::List(x) => (**x).clone(),
            Ty::Map(_, v) => (**v).clone(),
            _ => return Err("配列でないものの for".to_string()),
        };
        let key_ty = match &cty {
            Ty::Map(k, _) => (**k).clone(),
            _ => Ty::Int(IntTy::I64),
        };
        let i64t = Ty::Int(IntTy::I64);
        // 隠れた変数（ループの外へ出るとき drop する）
        self.scopes.push(Vec::new());
        let hidden = self.scopes.len();
        let mut src = if is_place(iter) && !matches!(&iter.kind, ExprKind::Var(n) if self.lookup(n).is_none()) {
            self.place(iter)?
        } else {
            // 一時的な値は回し終わったら捨てる
            let t = match self.expr(iter)? {
                Operand::Move(t, _) => t,
                _ => return Err("定数の for".to_string()),
            };
            self.scopes.last_mut().expect("scope").push((String::new(), t));
            Place::local(t, iter.span)
        };
        if ptr {
            src.proj.push(Proj::Deref(ast::Check::Required, iter.span));
        }
        if key.is_some() && src.proj.iter().any(|p| matches!(p, Proj::Index(..))) {
            return Err("要素の中の辞書の for (k, v)".to_string());
        }
        let what = format!("{} の要素", describe(iter));
        let items = self.declare("", Ty::List(Box::new(elem.clone())), false, var_span);
        self.assign(items, Rvalue::Items { place: src.clone(), what: what.clone(), span: var_span }, var_span);
        let keys = match key {
            Some(_) => {
                let k = self.declare("", Ty::List(Box::new(key_ty)), false, var_span);
                self.assign(k, Rvalue::Keys(src), var_span);
                Some(k)
            }
            None => None,
        };
        let rd = |l: Local| Place::local(l, var_span);
        let n = self.declare("", i64t.clone(), false, var_span);
        let len = Rvalue::SeqOp { name: "len".to_string(), place: rd(items), args: Vec::new(), span: var_span, arg_span: var_span };
        self.assign(n, len, var_span);
        let i = self.declare("", i64t.clone(), true, var_span);
        self.assign(i, Rvalue::Use(Operand::Const(Const::Int(0, IntTy::I64))), var_span);

        let head = self.new_block();
        let body_bb = self.new_block();
        let next = self.new_block();
        let done = self.new_block();
        let exit = self.new_block();
        self.goto_and_switch(head);
        let c = self.temp_of(Ty::Bool, Rvalue::Bin(BinOp::Lt, Operand::Copy(rd(i)), Operand::Copy(rd(n)), var_span), var_span);
        self.blocks[self.cur].term = Term::Branch(c, body_bb, done);
        self.cur = body_bb;
        self.scopes.push(Vec::new());
        let at = |l: Local| {
            let mut p = rd(l);
            p.proj.push(Proj::Index(Operand::Copy(rd(i)), var_span, iter.span));
            p
        };
        // ムーブする要素（構造体など）は写しを置いて、読むだけの ref で渡す
        let view = if self.moves(&elem)? {
            let w = self.declare("", elem.clone(), false, var_span);
            self.assign(w, Rvalue::Use(Operand::Copy(at(items))), var_span);
            Some(w)
        } else {
            None
        };
        if let (Some((kn, ks)), Some(keys)) = (key, keys) {
            let kt = self.var_ty(*ks)?;
            let k = self.declare(kn, kt, false, *ks);
            self.assign(k, Rvalue::Use(Operand::Copy(at(keys))), *ks);
        }
        let vt = self.var_ty(var_span)?;
        let v = self.declare(var, vt, false, var_span);
        let value = match view {
            Some(w) => Rvalue::Ref { place: rd(w), mutable: false, what, span: var_span },
            None => Rvalue::Use(Operand::Copy(at(items))),
        };
        self.assign(v, value, var_span);
        self.loops.push(Loop { brk: exit, brk_depth: hidden, cont: next, cont_depth: hidden + 1 });
        self.block(body, None)?;
        self.loops.pop();
        self.goto_and_switch(next);
        self.pop_scope(var_span, "次の周回へ進んだ");
        let inc = Rvalue::Bin(BinOp::Add, Operand::Copy(rd(i)), Operand::Const(Const::Int(1, IntTy::I64)), var_span);
        self.assign(i, inc, var_span);
        self.blocks[self.cur].term = Term::Goto(head);
        self.cur = done;
        if let Some(els) = els {
            let run_else = self.new_block();
            let z = Rvalue::Bin(BinOp::Eq, Operand::Copy(rd(n)), Operand::Const(Const::Int(0, IntTy::I64)), var_span);
            let z = self.temp_of(Ty::Bool, z, var_span);
            self.blocks[self.cur].term = Term::Branch(z, run_else, exit);
            self.cur = run_else;
            self.block(els, None)?;
        }
        self.goto_and_switch(exit);
        self.pop_scope(var_span, "for で回し終わった");
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

    /// `match`。`dest` があれば値をそこに入れる
    fn match_expr(&mut self, scrut: &Expr, arms: &[ast::MatchArm], default: Option<&ast::Block>, dest: Option<Local>) -> L<()> {
        let st = self.ty(scrut);
        let s = match self.expr(scrut)? {
            Operand::Move(t, _) => t,
            v => {
                let t = self.temp(st.clone(), scrut.span);
                self.assign(t, Rvalue::Use(v), scrut.span);
                t
            }
        };
        let join = self.new_block();
        let set_unit = |me: &mut Self, span: Span| {
            if let Some(d) = dest {
                me.assign(d, Rvalue::Use(Operand::Const(Const::Unit)), span);
            }
        };
        let drop_s = |me: &mut Self, why: &str| {
            me.emit(Inst::Drop { local: s, span: scrut.span, why: why.to_string() });
        };
        // enum（own / ref の先の enum も）
        let (ety, by_ref, place) = match &st {
            Ty::Enum(..) => (Some(st.clone()), None, Place::local(s, scrut.span)),
            Ty::Own(x) | Ty::Ref(x, _) if matches!(**x, Ty::Enum(..)) => {
                let m = match &st {
                    Ty::Ref(_, m) => Some(*m),
                    _ => None,
                };
                let mut p = Place::local(s, scrut.span);
                p.proj.push(Proj::Deref(ast::Check::Required, scrut.span));
                (Some((**x).clone()), m, p)
            }
            _ => (None, None, Place::local(s, scrut.span)),
        };
        if let Some(ety) = ety {
            // 種類ごとに、最初に書いた case へ
            let mut table: Vec<(Rc<str>, Bb)> = Vec::new();
            let mut todo = Vec::new();
            for arm in arms {
                for (p, _) in &arm.pats {
                    let ast::Pat::Variant(v, binds) = p else { return Err("enum の match に値の case".to_string()) };
                    if table.iter().any(|(n, _)| **n == **v) {
                        continue;
                    }
                    let bb = self.new_block();
                    table.push((Rc::from(v.as_str()), bb));
                    todo.push((bb, v, binds, &arm.body));
                }
            }
            let other = self.new_block();
            self.blocks[self.cur].term = Term::Switch(place.clone(), table, other);
            for (bb, v, binds, body) in todo {
                self.cur = bb;
                let ftys = self.variant_fields(&ety, v)?;
                self.scopes.push(Vec::new());
                for (i, (b, bspan)) in binds.iter().enumerate() {
                    let mut fp = place.clone();
                    fp.proj.push(Proj::Variant(Rc::from(v.as_str()), i));
                    let value = match by_ref {
                        // 値の match: 中身を受け取る名前へ移す
                        None => Rvalue::Take(fp),
                        Some(_) if b == "_" => continue,
                        // 借りた match: 中身の own は借りた ref として受け取る
                        Some(m) if matches!(ftys.get(i), Some(Ty::Own(_))) => {
                            Rvalue::Ref { place: fp, mutable: m, what: format!("{} の中身 {}", describe(scrut), b), span: *bspan }
                        }
                        Some(_) => Rvalue::Use(Operand::Copy(fp)),
                    };
                    if b == "_" {
                        let t = self.temp_of(Ty::Unit, value, *bspan);
                        self.emit(Inst::Discard { value: t, span: *bspan, why: "match で受け取られずに捨てられた".to_string() });
                        continue;
                    }
                    let ty = self.var_ty(*bspan)?;
                    let l = self.declare(b, ty, false, *bspan);
                    self.assign(l, value, *bspan);
                }
                drop_s(self, "match で中身を取り出したので、入れ物が解放された");
                self.block(body, dest)?;
                self.pop_scope(body.end, "スコープを抜けた");
                self.blocks[self.cur].term = Term::Goto(join);
            }
            self.cur = other;
            if by_ref.is_none() {
                let why = "match の default で使われずに捨てられた".to_string();
                self.emit(Inst::Discard { value: Operand::Move(s, scrut.span), span: scrut.span, why });
            } else {
                drop_s(self, "match を終えた");
            }
            match default {
                Some(b) => self.block(b, dest)?,
                None => set_unit(self, scrut.span),
            }
            self.goto_and_switch(join);
            return Ok(());
        }
        // 整数・真偽値・文字列: 上から順に比べる
        for arm in arms {
            let hit = self.new_block();
            for (p, _) in &arm.pats {
                let c = match p {
                    ast::Pat::Int(n) => Const::Int(*n, IntTy::I64),
                    ast::Pat::Bool(b) => Const::Bool(*b),
                    ast::Pat::Str(t) => Const::Str(Rc::from(t.as_str())),
                    ast::Pat::Variant(..) => return Err("値の match に enum の case".to_string()),
                };
                let eq = Rvalue::Bin(BinOp::Eq, Operand::Copy(place.clone()), Operand::Const(c), scrut.span);
                let c = self.temp_of(Ty::Bool, eq, scrut.span);
                let next = self.new_block();
                self.blocks[self.cur].term = Term::Branch(c, hit, next);
                self.cur = next;
            }
            let miss = self.cur;
            self.cur = hit;
            drop_s(self, "match を終えた");
            self.block(&arm.body, dest)?;
            self.blocks[self.cur].term = Term::Goto(join);
            self.cur = miss;
        }
        drop_s(self, "match を終えた");
        match default {
            Some(b) => self.block(b, dest)?,
            None => set_unit(self, scrut.span),
        }
        self.goto_and_switch(join);
        Ok(())
    }

    /// `e?`: Ok / Some なら中身。Err / None なら、評価の途中の値と全部の変数を捨てて、それを関数から返す
    fn try_expr(&mut self, inner: &Expr, sp: Span) -> L<Operand> {
        let it = self.ty(inner);
        let ok = match &it {
            Ty::Enum(n, _) if n == "Result" => "Ok",
            Ty::Enum(n, _) if n == "Option" => "Some",
            _ => return Err("? の型".to_string()),
        };
        let t = match self.expr(inner)? {
            Operand::Move(t, _) => t,
            _ => return Err("? に定数".to_string()),
        };
        let ok_bb = self.new_block();
        let err_bb = self.new_block();
        self.blocks[self.cur].term = Term::Switch(Place::local(t, sp), vec![(Rc::from(ok), ok_bb)], err_bb);
        self.cur = err_bb;
        // まだ使われていない一時的な値（`f(own 1, g()?)` の `own 1`）。使い終わったものは空なので、drop しても何もしない
        let in_scope: Vec<Local> = self.scopes.iter().flatten().map(|(_, l)| *l).collect();
        for l in (0..self.locals.len()).rev() {
            if l != t && self.locals[l].name.is_empty() && !in_scope.contains(&l) {
                let why = "`?` で関数を抜けたので、使われずに捨てられた".to_string();
                self.emit(Inst::Drop { local: l, span: sp, why });
            }
        }
        self.leave(0, sp, "`?` で関数を抜けた");
        self.terminate(Term::Return(Operand::Move(t, sp)));
        self.cur = ok_bb;
        let fty = self.variant_fields(&it, ok)?.pop().unwrap_or(Ty::Unit);
        let mut fp = Place::local(t, sp);
        fp.proj.push(Proj::Variant(Rc::from(ok), 0));
        let r = self.temp_of(fty, Rvalue::Take(fp), sp);
        self.emit(Inst::Drop { local: t, span: sp, why: "`?` で中身を取り出した".to_string() });
        Ok(r)
    }

    /// enum の種類の中身の型（型の引数は置き換え済み）
    fn variant_fields(&self, t: &Ty, variant: &str) -> L<Vec<Ty>> {
        let Ty::Enum(n, args) = t else { return Err("enum でない".to_string()) };
        let (tparams, vs) = self.info.enums.get(n).ok_or("enum の記録がない")?;
        let map: HashMap<String, Ty> = tparams.iter().cloned().zip(args.iter().cloned()).collect();
        let (_, fs) = vs.iter().find(|(v, _)| v == variant).ok_or("ない種類")?;
        Ok(fs.iter().map(|f| types::subst(f, &map)).collect())
    }

    /// 値として読むとムーブになる型か（interp の `Value::moves` を型で決めたもの）
    fn moves(&self, t: &Ty) -> L<bool> {
        Ok(match t {
            Ty::Int(_) | Ty::Float | Ty::Bool | Ty::Str | Ty::Unit | Ty::Never | Ty::Ref(..) => false,
            // 関数の値は取り込んだものしだい（変数を読むときは `Operand::Read` で値を見て決める）
            Ty::Own(_) | Ty::Struct(..) | Ty::List(_) | Ty::Map(..) | Ty::Fn(..) => true,
            Ty::Array(x, _) => self.moves(x)?,
            Ty::Enum(n, args) => {
                let (tparams, vs) = self.info.enums.get(n).ok_or("enum の記録がない")?;
                let map: HashMap<String, Ty> = tparams.iter().cloned().zip(args.iter().cloned()).collect();
                for (_, fs) in vs {
                    for f in fs {
                        if self.moves(&types::subst(f, &map))? {
                            return Ok(true);
                        }
                    }
                }
                false
            }
            _ => return Err(format!("型 {}", t.show())),
        })
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
            ExprKind::Var(name) => match self.lookup(name) {
                Some(l) => {
                    let read = self.read_var(l, sp)?;
                    self.temp_of(ty, Rvalue::Use(read), sp)
                }
                // 名前のある関数を値にする
                None => {
                    let f = self.calls.index(name, &HashMap::new())?;
                    self.temp_of(ty, Rvalue::Closure(f, Vec::new()), sp)
                }
            },
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
            ExprKind::Deref(..) | ExprKind::Field(..) | ExprKind::Index(..) => {
                // ポインタの先・フィールド・要素から取り出せるのはコピーできる値だけ（ムーブする値なら実行時エラー）
                let (place, t) = self.read_place(e)?;
                let what = match &e.kind {
                    ExprKind::Deref(..) => "ポインタの先",
                    ExprKind::Field(..) => "フィールド",
                    _ => "配列の中",
                };
                let r = self.temp_of(ty, Rvalue::CopyOut(place, what.to_string(), sp), sp);
                self.drop_temp(&t.map(|t| Operand::Move(t, sp)), sp);
                r
            }
            ExprKind::ArrayLit(_, seq) if seq.get() == ast::Seq::Map => self.temp_of(ty, Rvalue::Map(Vec::new(), sp), sp),
            ExprKind::ArrayLit(items, seq) => {
                let mut ops = Vec::new();
                for x in items {
                    ops.push(self.expr(x)?);
                }
                self.temp_of(ty, Rvalue::Array(ops, seq.get() == ast::Seq::List), sp)
            }
            ExprKind::ArrayRepeat(x, n, list) => {
                if self.moves(&self.ty(x))? {
                    return Err("ムーブする値を並べる".to_string());
                }
                let v = self.expr(x)?;
                self.temp_of(ty, Rvalue::Repeat(v, *n, list.get()), sp)
            }
            ExprKind::MapLit(pairs) => {
                let mut ops = Vec::new();
                for (k, v) in pairs {
                    let k = self.expr(k)?;
                    ops.push((k, self.expr(v)?));
                }
                self.temp_of(ty, Rvalue::Map(ops, sp), sp)
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
            ExprKind::Try(inner) => self.try_expr(inner, sp)?,
            ExprKind::Variant { variant, args, copy, .. } => {
                let mut ops = Vec::new();
                let mut moving = false;
                for a in args {
                    moving |= self.moves(&self.ty(a))?;
                    ops.push(self.expr(a)?);
                }
                // 型の引数を含む enum（ジェネリックな関数の中）は、interp と同じく中身で決める
                let copy = copy.get().unwrap_or(!moving);
                self.temp_of(ty, Rvalue::Enum(Rc::from(variant.as_str()), ops, copy), sp)
            }
            ExprKind::Match { scrut, arms, default } => {
                let d = self.temp(ty.clone(), sp);
                self.match_expr(scrut, arms, default.as_ref(), Some(d))?;
                Operand::Move(d, sp)
            }
            ExprKind::Lambda(l) => {
                // 外の変数を、名前で読むのと同じように取り込む（own はムーブ、コピーできる値はコピー）
                let names = l.captures.borrow().clone();
                let mut ops = Vec::new();
                let mut caps = Vec::new();
                for n in &names {
                    let lc = self.lookup(n).ok_or("取り込む変数がない")?;
                    caps.push((n.clone(), self.locals[lc].ty.clone()));
                    ops.push(self.read_var(lc, sp)?);
                }
                let ret = match &ty {
                    Ty::Fn(_, r) => (**r).clone(),
                    _ => Ty::Unit,
                };
                let inst = format!("{}::λ{}:{}", self.name, sp.line, sp.col);
                let body = Body::Lambda { lambda: l.clone(), caps, ret, span: sp };
                let f = self.calls.push(inst, body, self.subst.clone());
                self.temp_of(ty, Rvalue::Closure(f, ops), sp)
            }
        })
    }

    /// print と埋め込みの 1 つ分: 借りて読み（interp の `eval_borrow`）、文字にする。一時的な値は見たあと捨てる
    fn show(&mut self, e: &Expr) -> L<Operand> {
        let is_place_read = !e.borrow.get()
            && match &e.kind {
                ExprKind::Var(n) => self.lookup(n).is_some(),
                ExprKind::Deref(..) | ExprKind::Field(..) | ExprKind::Index(..) => true,
                _ => false,
            };
        let r = if is_place_read {
            let (pl, t) = self.read_place(e)?;
            let r = self.temp_of(Ty::Str, Rvalue::Show(Operand::Copy(pl), false, e.span), e.span);
            self.drop_temp(&t.map(|t| Operand::Move(t, e.span)), e.span);
            return Ok(r);
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
            _ if self.lookup(name).is_some() => {
                // 引数を先に評価してから、変数を借りて読む（interp の `call_var`）
                let mut vals = Vec::new();
                for a in args {
                    vals.push(self.expr(a)?);
                }
                let l = self.lookup(name).expect("checked above");
                let mut f = Place::local(l, sp);
                if matches!(self.locals[l].ty, Ty::Own(_) | Ty::Ref(..)) {
                    f.proj.push(Proj::Deref(ast::Check::Required, sp));
                }
                Ok(self.temp_of(ty, Rvalue::CallValue(Operand::Copy(f), vals, sp), sp))
            }
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
                if temp.is_none() && self.through_list(recv) {
                    return Err("リストの中の値のメソッド".to_string());
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
            Ty::Array(..) | Ty::List(_) | Ty::Map(..) => {
                let pl = recv_place(self)?.ok_or("定数の receiver")?;
                let arg_span = args.first().map_or(sp, |a| a.span);
                let op = Rvalue::SeqOp { name: name.to_string(), place: pl, args: vals, span: sp, arg_span };
                let r = self.temp_of(ty, op, sp);
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
            ExprKind::Deref(base, _) | ExprKind::Field(base, ..) | ExprKind::Index(base, ..) if is_place(base) => {
                // 添字を先に評価してから、配列の場所を求める（interp の `place` と同じ順）
                let key = match &e.kind {
                    ExprKind::Index(_, idx, _) => Some(self.expr(idx)?),
                    _ => None,
                };
                let mut p = self.place(base)?;
                self.project(&mut p, e, base, key)?;
                Ok(p)
            }
            _ => Err("一時的な値の中の場所".to_string()),
        }
    }

    /// 読むための場所（interp の `eval_borrow`）。一時的な値の中（`make().x`・`make()[0]`・`*make()`）なら、
    /// その値を入れた一時的な変数も返す（読み終わったら drop する）
    fn read_place(&mut self, e: &Expr) -> L<(Place, Option<Local>)> {
        let base = match &e.kind {
            ExprKind::Deref(b, _) | ExprKind::Field(b, ..) | ExprKind::Index(b, ..) if !is_place(b) => b,
            _ => return Ok((self.place(e)?, None)),
        };
        let key = match &e.kind {
            ExprKind::Index(_, idx, _) => Some(self.expr(idx)?),
            _ => None,
        };
        let t = match self.expr(base)? {
            Operand::Move(t, _) => t,
            _ => return Err("定数の中の場所".to_string()),
        };
        // 一時的な own・構造体の中から own は取り出せない（interp のエラー）
        if !matches!(self.ty(base), Ty::Ref(..)) && self.moves(&self.ty(e))? {
            return Err("一時的な値の中の own".to_string());
        }
        let mut p = Place::local(t, base.span);
        self.project(&mut p, e, base, key)?;
        Ok((p, Some(t)))
    }

    /// `base` の場所 `p` から、`e`（`*base`・`base.f`・`base[k]`）の場所へ進む。own / ref の先は自動でたどる
    fn project(&mut self, p: &mut Place, e: &Expr, base: &Expr, key: Option<Operand>) -> L<()> {
        let bt = self.ty(base);
        let inner = match (&e.kind, &bt) {
            (ExprKind::Deref(_, c), _) => {
                p.proj.push(Proj::Deref(c.get(), e.span));
                return Ok(());
            }
            (ExprKind::Field(_, _, c) | ExprKind::Index(_, _, c), Ty::Own(t) | Ty::Ref(t, _)) => {
                p.proj.push(Proj::Deref(c.get(), e.span));
                (**t).clone()
            }
            _ => bt.clone(),
        };
        match (&e.kind, &inner) {
            (ExprKind::Field(_, name, _), Ty::Struct(sname, _)) => {
                let def = self.structs.get(sname).ok_or("構造体がない")?;
                let i = def.field_index(name).ok_or("ないフィールド")?;
                p.proj.push(Proj::Field(i, e.span));
            }
            (ExprKind::Index(_, idx, _), Ty::Array(..) | Ty::List(_) | Ty::Map(..)) => {
                p.proj.push(Proj::Index(key.expect("index key"), idx.span, base.span));
            }
            _ => return Err(format!("{} の中の場所", inner.show())),
        }
        Ok(())
    }

    /// 場所がリスト・辞書の中を通るか（中の値は借りられない。interp の `through_list`）
    fn through_list(&self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Field(base, ..) | ExprKind::Index(base, ..) => {
                let bt = self.ty(base);
                let (inner, ptr) = match &bt {
                    Ty::Own(t) | Ty::Ref(t, _) => ((**t).clone(), true),
                    t => (t.clone(), false),
                };
                if matches!(e.kind, ExprKind::Index(..)) && matches!(inner, Ty::List(_) | Ty::Map(..)) {
                    return true;
                }
                !ptr && self.through_list(base)
            }
            _ => false,
        }
    }
}

/// どこからも飛んでこないブロック（return や break のあと）を取り除き、番号を詰める
fn reachable(blocks: Vec<Block>) -> Vec<Block> {
    fn succ(t: &Term) -> Vec<Bb> {
        match t {
            Term::Goto(b) => vec![*b],
            Term::Branch(_, a, b) | Term::Step { next: a, done: b, .. } => vec![*a, *b],
            Term::Switch(_, arms, d) => arms.iter().map(|(_, b)| *b).chain(std::iter::once(*d)).collect(),
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
                Term::Switch(p, arms, d) => {
                    Term::Switch(p, arms.into_iter().map(|(v, b)| (v, new_index[b])).collect(), new_index[d])
                }
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
    matches!(e.kind, ExprKind::Var(_) | ExprKind::Deref(..) | ExprKind::Field(..) | ExprKind::Index(..))
}

