//! 核になるインタプリタ。
//!
//! 安全性の仕組み（この言語の看板）:
//!   * `own`  : 唯一の所有者。代入・引数渡しで「ムーブ」され、元の変数は使えなくなる。
//!   * `ref`  : 世代付き参照。(スロット番号, 世代) を持つ。所有者が解放されるとスロットの
//!     世代が進むので、古い参照を使った瞬間に「確実に」検出できる。
//!     検出時は「借りた → 解放された → 使った」を時系列で報告する。
//!
//! 静的解析（`analyze`）が「絶対安全」と証明できた `*p` は世代チェックを飛ばす（速さ）。
//! 証明できない箇所だけ実行時検査が残る（安全）。

use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::*;
use crate::diag::{Diagnostic, Span};

type R<T> = Result<T, Diagnostic>;

const MAX_DEPTH: usize = 5000;
const BUILTINS: [&str; 3] = ["print", "drop", "assert"];

#[derive(Debug, Clone)]
pub enum Value {
    Unit,
    Int(i64),
    Bool(bool),
    Str(Rc<str>),
    /// 唯一の所有者（ヒープのスロット番号）
    Own(usize),
    /// 世代付き参照
    Ref(RefVal),
    /// 構造体。フィールドは定義の順。中に own を持ちうるので、値として読むとムーブになる。
    Struct(Rc<StructDef>, Vec<Value>),
}

#[derive(Debug, Clone)]
pub struct RefVal {
    slot: usize,
    generation: u32,
    borrowed_at: Span,
    what: String,
    /// `ref mut` で借りたか（書き換えられるか）
    mutable: bool,
}

impl Value {
    fn type_name(&self) -> &'static str {
        match self {
            Value::Unit => "()",
            Value::Int(_) => "整数",
            Value::Bool(_) => "真偽値",
            Value::Str(_) => "文字列",
            Value::Own(_) => "own",
            Value::Ref(_) => "ref",
            Value::Struct(..) => "構造体",
        }
    }

    /// 中に（フィールドも含めて）own を持っているか
    fn has_own(&self) -> bool {
        match self {
            Value::Own(_) => true,
            Value::Struct(_, fields) => fields.iter().any(Value::has_own),
            _ => false,
        }
    }

    /// 中にある own のスロットをすべて集める
    fn collect_owns(&self, out: &mut Vec<usize>) {
        match self {
            Value::Own(s) => out.push(*s),
            Value::Struct(_, fields) => fields.iter().for_each(|f| f.collect_owns(out)),
            _ => {}
        }
    }

    fn show(&self) -> String {
        match self {
            Value::Unit => "()".to_string(),
            Value::Int(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Str(s) => s.to_string(),
            Value::Own(s) => format!("<own #{}>", s),
            Value::Ref(r) => format!("<ref #{}>", r.slot),
            Value::Struct(def, fields) => {
                let parts: Vec<String> =
                    def.fields.iter().zip(fields).map(|(n, v)| format!("{}: {}", n, v.show())).collect();
                format!("{} {{ {} }}", def.name, parts.join(", "))
            }
        }
    }
}

// ---------------------------------------------------------------- ヒープ

struct Slot {
    generation: u32,
    value: Option<Value>,
}

struct Freed {
    span: Span,
    why: String,
}

struct Heap {
    slots: Vec<Slot>,
    free_list: Vec<usize>,
    /// (スロット, その世代) がいつ・なぜ解放されたか。古い参照のエラー報告に使う。
    history: HashMap<(usize, u32), Freed>,
}

impl Heap {
    fn new() -> Self {
        Heap { slots: Vec::new(), free_list: Vec::new(), history: HashMap::new() }
    }

    fn alloc(&mut self, v: Value) -> usize {
        if let Some(i) = self.free_list.pop() {
            self.slots[i].value = Some(v);
            i
        } else {
            self.slots.push(Slot { generation: 0, value: Some(v) });
            self.slots.len() - 1
        }
    }

    fn live(&self) -> usize {
        self.slots.iter().filter(|s| s.value.is_some()).count()
    }
}

// ---------------------------------------------------------------- 環境

#[derive(Clone)]
enum VarState {
    Live(Value),
    /// 所有権が移動済み。どこで移動したかを覚えておく。
    Moved(Span),
}

struct Var {
    name: String,
    state: VarState,
    /// `mut` がついているか（書き換えられるか）
    mutable: bool,
    /// 宣言した位置（書き換えられないときのエラー用）
    decl: Span,
}

struct Frame {
    scopes: Vec<Vec<Var>>,
}

/// 書き込み・読み出しができる場所（変数かヒープのスロット）と、その中のフィールドの道筋
struct Place {
    root: Root,
    path: Vec<usize>,
    access: Access,
}

/// 場所に書き込めるか。書き込めないときはその理由（エラー表示用）
#[derive(Clone)]
enum Access {
    Write,
    /// `mut` のない変数（とその中身）
    ImmutVar { name: String, decl: Span },
    /// `mut` のない ref の先
    ReadOnlyRef { what: String, borrowed_at: Span },
}

impl Access {
    /// ポインタの先の場所に書き込めるか。own の先は持ち主と同じ、ref の先は ref 次第。
    fn through(self, p: &Value) -> Access {
        match p {
            Value::Ref(r) if r.mutable => Access::Write,
            Value::Ref(r) => Access::ReadOnlyRef { what: r.what.clone(), borrowed_at: r.borrowed_at },
            _ => self,
        }
    }

    /// 書き込めなければ、`doing`（「書き換えようとした」など）を添えたエラーにする
    fn require(&self, span: Span, doing: &str) -> R<()> {
        match self {
            Access::Write => Ok(()),
            Access::ImmutVar { name, decl } => Err(Diagnostic::new(format!(
                "`{}` は mut ではないので書き換えられません",
                name
            ))
            .label(*decl, format!("ここで `{}` を宣言した（mut がない）", name))
            .primary(span, format!("ここで{}", doing))
            .note(format!("書き換えるなら `let mut {}` や `fn f(mut {})` のように mut をつけてください", name, name))),
            Access::ReadOnlyRef { what, borrowed_at } => Err(Diagnostic::new("読むだけの ref を通して書き換えようとしました")
                .label(*borrowed_at, format!("ここで `{}` を ref で借りた（mut がない）", what))
                .primary(span, format!("ここで{}", doing))
                .note("書き換えるなら `ref mut` で借りてください")),
        }
    }
}

enum Root {
    Var(String),
    Slot(usize),
}

enum Flow {
    Normal,
    Return(Value, Span),
    Break(Span),
    Continue(Span),
}

pub struct Interp {
    heap: Heap,
    frames: Vec<Frame>,
    fns: HashMap<String, Rc<FnDef>>,
    structs: HashMap<String, Rc<StructDef>>,
    pub output: String,
    echo: bool,
    depth: usize,
    /// 実行時に行った ref の世代チェックの回数
    pub ref_checks: usize,
    /// 静的解析のおかげで飛ばせた ref の世代チェックの回数
    pub ref_checks_elided: usize,
}

impl Interp {
    pub fn new(echo: bool) -> Self {
        Interp {
            heap: Heap::new(),
            frames: Vec::new(),
            fns: HashMap::new(),
            structs: HashMap::new(),
            output: String::new(),
            echo,
            depth: 0,
            ref_checks: 0,
            ref_checks_elided: 0,
        }
    }

    /// 解放されていないヒープオブジェクトの数（リーク検査用）
    pub fn live_objects(&self) -> usize {
        self.heap.live()
    }

    pub fn run(&mut self, prog: Program) -> R<()> {
        for s in prog.structs {
            if self.structs.contains_key(&s.name) {
                return Err(Diagnostic::new(format!("構造体 `{}` が二重に定義されています", s.name))
                    .primary(s.span, "ここで再定義されている"));
            }
            self.structs.insert(s.name.clone(), Rc::new(s));
        }
        for f in prog.fns {
            if BUILTINS.contains(&f.name.as_str()) {
                return Err(Diagnostic::new(format!("`{}` は組み込み関数の名前です", f.name))
                    .primary(f.span, "別の名前にしてください"));
            }
            if self.fns.contains_key(&f.name) {
                return Err(Diagnostic::new(format!("関数 `{}` が二重に定義されています", f.name))
                    .primary(f.span, "ここで再定義されている"));
            }
            self.fns.insert(f.name.clone(), Rc::new(f));
        }
        let main = self.fns.get("main").cloned().ok_or_else(|| {
            Diagnostic::new("`main` 関数がありません").note("`fn main() { ... }` を定義してください")
        })?;
        self.call_fn(&main, Vec::new(), main.span)?;
        Ok(())
    }

    // ------------------------------------------------------------ 解放

    /// スロットを解放し、世代を進める。中身が own なら連鎖して解放する。
    fn free(&mut self, slot: usize, span: Span, why: &str) {
        let mut work: Vec<(usize, String)> = vec![(slot, why.to_string())];
        while let Some((s, why)) = work.pop() {
            let Some(inner) = self.heap.slots[s].value.take() else {
                continue;
            };
            let g = self.heap.slots[s].generation;
            self.heap.slots[s].generation = g.wrapping_add(1);
            self.heap.history.insert((s, g), Freed { span, why });
            self.heap.free_list.push(s);
            let mut children = Vec::new();
            inner.collect_owns(&mut children);
            for child in children {
                work.push((child, "持ち主が解放されたので、中身も連鎖して解放された".to_string()));
            }
        }
    }

    /// 値を捨てる。中にある own は（構造体のフィールドも含めて）すべて解放する。
    fn release(&mut self, v: Value, span: Span, why: &str) {
        let mut owns = Vec::new();
        v.collect_owns(&mut owns);
        for s in owns {
            self.free(s, span, why);
        }
    }

    fn drop_scope(&mut self, scope: Vec<Var>, span: Span, how: &str) {
        for var in scope.into_iter().rev() {
            if let VarState::Live(v) = var.state {
                self.release(v, span, &format!("`{}` は{}ので解放された", var.name, how));
            }
        }
    }

    // ------------------------------------------------------------ 変数

    fn find_var(&mut self, name: &str) -> Option<&mut Var> {
        let frame = self.frames.last_mut()?;
        for scope in frame.scopes.iter_mut().rev() {
            for v in scope.iter_mut().rev() {
                if v.name == name {
                    return Some(v);
                }
            }
        }
        None
    }

    fn declare(&mut self, name: &str, v: Value, mutable: bool, decl: Span) {
        self.frames
            .last_mut()
            .expect("frame")
            .scopes
            .last_mut()
            .expect("scope")
            .push(Var { name: name.to_string(), state: VarState::Live(v), mutable, decl });
    }

    // ------------------------------------------------------------ ポインタ検査

    /// ポインタの先のスロットを返す。参照が古ければ時系列付きのエラーにする。
    /// `check` が `Elided` なら、静的解析が安全を証明済みなので世代チェックを飛ばす。
    fn check_ptr(&mut self, p: &Value, use_span: Span, check: Check) -> R<usize> {
        match p {
            Value::Own(s) => Ok(*s),
            Value::Ref(r) => {
                let slot = &self.heap.slots[r.slot];
                let valid = slot.generation == r.generation && slot.value.is_some();
                if check == Check::Elided {
                    // 解析の誤りを見逃さないよう、デバッグビルドでだけ念のため確かめる
                    debug_assert!(valid, "static analysis elided the check of a dangling ref");
                    self.ref_checks_elided += 1;
                    return Ok(r.slot);
                }
                self.ref_checks += 1;
                if valid {
                    return Ok(r.slot);
                }
                let mut d = Diagnostic::new("無効になった参照を使いました (dangling reference)")
                    .label(r.borrowed_at, format!("ここで `{}` を借りた", r.what));
                if let Some(f) = self.heap.history.get(&(r.slot, r.generation)) {
                    d = d.label(f.span, format!("ここで解放された: {}", f.why));
                }
                Err(d
                    .primary(use_span, "ここで使ったが、借りた先はもう存在しない")
                    .note("参照は、借りた元の所有者より長くは生きられません"))
            }
            other => Err(Diagnostic::new("ポインタではない値に `*` を使いました")
                .primary(use_span, format!("{} は own でも ref でもありません", other.type_name()))),
        }
    }

    // ------------------------------------------------------------ 文

    fn exec_block(&mut self, block: &Block) -> R<Flow> {
        Ok(self.exec_block_value(block, false)?.0)
    }

    /// ブロックを実行する。`want_value` なら最後の行の式の値を返す（if 式の値）。
    fn exec_block_value(&mut self, block: &Block, want_value: bool) -> R<(Flow, Value)> {
        self.frames.last_mut().expect("frame").scopes.push(Vec::new());
        let mut result = Flow::Normal;
        let mut value = Value::Unit;
        for (i, s) in block.stmts.iter().enumerate() {
            let flow = match s {
                Stmt::Expr(e) if want_value && block.tail && i + 1 == block.stmts.len() => {
                    let (flow, v) = self.eval_flow(e, true)?;
                    value = v;
                    flow
                }
                _ => self.exec_stmt(s)?,
            };
            if !matches!(flow, Flow::Normal) {
                result = flow;
                break;
            }
        }
        let scope = self.frames.last_mut().expect("frame").scopes.pop().expect("scope");
        let (span, how) = match &result {
            Flow::Return(_, sp) => (*sp, "関数から return した"),
            Flow::Break(sp) => (*sp, "break でループを抜けた"),
            Flow::Continue(sp) => (*sp, "continue で次の周回へ進んだ"),
            Flow::Normal => (block.end, "スコープを抜けた"),
        };
        self.drop_scope(scope, span, how);
        Ok((result, value))
    }

    /// 式を評価する。文として使う if は中の return / break / continue を外へ伝える。
    fn eval_flow(&mut self, e: &Expr, want_value: bool) -> R<(Flow, Value)> {
        match &e.kind {
            ExprKind::If { cond, then, els } => {
                let c = self.eval(cond)?;
                let blk = if as_bool(c, cond.span)? { Some(then) } else { els.as_ref() };
                match blk {
                    Some(blk) => self.exec_block_value(blk, want_value),
                    None => Ok((Flow::Normal, Value::Unit)),
                }
            }
            ExprKind::Match { scrut, arms, default } => {
                let v = self.eval(scrut)?;
                match select_arm(&v, scrut.span, arms)?.or(default.as_ref()) {
                    Some(blk) => self.exec_block_value(blk, want_value),
                    None => Ok((Flow::Normal, Value::Unit)),
                }
            }
            _ => Ok((Flow::Normal, self.eval(e)?)),
        }
    }

    /// ループ本体を1周実行し、ループを続けるかどうかを返す
    fn loop_iteration(&mut self, body: &Block) -> R<Option<Flow>> {
        match self.exec_block(body)? {
            Flow::Normal | Flow::Continue(_) => Ok(None),
            Flow::Break(_) => Ok(Some(Flow::Normal)),
            r @ Flow::Return(..) => Ok(Some(r)),
        }
    }

    fn exec_stmt(&mut self, s: &Stmt) -> R<Flow> {
        match s {
            Stmt::Let { name, mutable, span, init, .. } => {
                let v = self.eval(init)?;
                self.declare(name, v, *mutable, *span);
                Ok(Flow::Normal)
            }
            Stmt::Assign { target, value, span } => {
                self.assign(target, value, *span)?;
                Ok(Flow::Normal)
            }
            Stmt::Expr(e) => {
                let (flow, v) = self.eval_flow(e, false)?;
                self.release(v, e.span, "どこにも束縛されず、そのまま捨てられた");
                Ok(flow)
            }
            Stmt::While { cond, body } => loop {
                let c = self.eval(cond)?;
                if !as_bool(c, cond.span)? {
                    return Ok(Flow::Normal);
                }
                if let Some(f) = self.loop_iteration(body)? {
                    return Ok(f);
                }
            },
            Stmt::Loop { body } => loop {
                if let Some(f) = self.loop_iteration(body)? {
                    return Ok(f);
                }
            },
            Stmt::For { var, range, range_span, body, els } => {
                let mut nums = Vec::new();
                for a in range {
                    match self.eval(a)? {
                        Value::Int(n) => nums.push(n),
                        other => {
                            return Err(Diagnostic::new("range の引数は整数です")
                                .primary(a.span, format!("ここは {} です", other.type_name())))
                        }
                    }
                }
                let (start, end, step) = match nums[..] {
                    [e] => (0, e, 1),
                    [s, e] => (s, e, 1),
                    [s, e, st] => (s, e, st),
                    _ => unreachable!("parser checks the range arity"),
                };
                if step == 0 {
                    return Err(Diagnostic::new("range の刻みが 0 です")
                        .primary(*range_span, "これだと永遠に終わりません"));
                }
                let mut i = start;
                let mut ran = false;
                while (step > 0 && i < end) || (step < 0 && i > end) {
                    ran = true;
                    // ループ変数は周回ごとの新しいスコープに置く
                    self.frames.last_mut().expect("frame").scopes.push(Vec::new());
                    self.declare(var, Value::Int(i), false, *range_span);
                    let r = self.loop_iteration(body);
                    self.frames.last_mut().expect("frame").scopes.pop();
                    if let Some(f) = r? {
                        return Ok(f);
                    }
                    match i.checked_add(step) {
                        Some(n) => i = n,
                        None => break,
                    }
                }
                match els {
                    Some(els) if !ran => self.exec_block(els),
                    _ => Ok(Flow::Normal),
                }
            }
            Stmt::Break(sp) => Ok(Flow::Break(*sp)),
            Stmt::Continue(sp) => Ok(Flow::Continue(*sp)),
            Stmt::Return { value, span } => {
                let v = match value {
                    Some(e) => self.eval(e)?,
                    None => Value::Unit,
                };
                Ok(Flow::Return(v, *span))
            }
        }
    }

    fn assign(&mut self, target: &Expr, value: &Expr, span: Span) -> R<()> {
        let newv = self.eval(value)?;
        match &target.kind {
            ExprKind::Var(name) => {
                let old = {
                    let var = self
                        .find_var(name)
                        .ok_or_else(|| undefined_var(name, target.span))?;
                    if !var.mutable {
                        let access = Access::ImmutVar { name: name.clone(), decl: var.decl };
                        access.require(target.span, "書き換えようとした")?;
                    }
                    std::mem::replace(&mut var.state, VarState::Live(newv))
                };
                if let VarState::Live(v) = old {
                    self.release(v, span, &format!("`{}` に再代入されて、古い値が解放された", name));
                }
                Ok(())
            }
            ExprKind::Deref(..) | ExprKind::Field(..) => {
                let place = self.place(target)?;
                place.access.require(target.span, "書き換えようとした")?;
                let old = std::mem::replace(self.place_mut(&place), newv);
                self.release(old, span, &format!("`{}` が上書きされて、古い値が解放された", describe(target)));
                Ok(())
            }
            _ => unreachable!("parser only produces Var/Deref/Field assignment targets"),
        }
    }

    // ------------------------------------------------------------ 式

    /// 値を「借りて読む」評価。変数はムーブしない。
    /// 戻り値の bool は「新しく作られた一時値か」（own なら使い終わりで解放が必要）。
    fn eval_borrow(&mut self, e: &Expr) -> R<(Value, bool)> {
        match &e.kind {
            ExprKind::Var(name) => {
                let var = self.find_var(name).ok_or_else(|| undefined_var(name, e.span))?;
                match &var.state {
                    VarState::Live(v) => Ok((v.clone(), false)),
                    VarState::Moved(sp) => Err(moved_error(name, *sp, e.span)),
                }
            }
            ExprKind::Deref(inner, check) => {
                let (p, temp) = self.eval_borrow(inner)?;
                let slot = self.check_ptr(&p, e.span, check.get())?;
                let v = self.heap.slots[slot].value.clone().expect("live slot has a value");
                if temp {
                    if let Value::Own(tmp) = p {
                        if v.has_own() {
                            return Err(Diagnostic::new("一時的な own の中から own を取り出せません")
                                .primary(e.span, "先に変数に入れてください"));
                        }
                        self.free(tmp, e.span, "使い終わった一時的な値");
                    }
                }
                Ok((v, false))
            }
            ExprKind::Field(base, name, check) => {
                if is_place(base) {
                    let place = self.place(e)?;
                    return Ok((self.place_mut(&place).clone(), false));
                }
                // 一時的な値のフィールド（`make().x` など）
                let tmp = self.eval(base)?;
                let (v, holder) = match &tmp {
                    Value::Struct(def, fields) => (fields[self.field_index(def, name, e.span)?].clone(), None),
                    Value::Own(_) | Value::Ref(_) => {
                        let slot = self.check_ptr(&tmp, e.span, check.get())?;
                        let v = self.field_of_slot(slot, name, e.span)?.clone();
                        (v, matches!(tmp, Value::Own(_)).then_some(slot))
                    }
                    other => return Err(not_a_struct(other, base.span)),
                };
                if v.has_own() && !matches!(tmp, Value::Ref(_)) {
                    return Err(Diagnostic::new("一時的な値の中から own を取り出せません")
                        .primary(e.span, "先に変数に入れてください"));
                }
                match holder {
                    Some(slot) => self.free(slot, e.span, "使い終わった一時的な値"),
                    None => self.release(tmp, e.span, "使い終わった一時的な値"),
                }
                Ok((v, false))
            }
            _ => Ok((self.eval(e)?, true)),
        }
    }

    // ------------------------------------------------------------ 場所

    /// 代入やフィールド読み出しの対象になる場所を求める。own / ref は自動でたどる。
    fn place(&mut self, e: &Expr) -> R<Place> {
        match &e.kind {
            ExprKind::Var(name) => {
                let var = self.find_var(name).ok_or_else(|| undefined_var(name, e.span))?;
                if let VarState::Moved(sp) = var.state {
                    return Err(moved_error(name, sp, e.span));
                }
                let access = if var.mutable {
                    Access::Write
                } else {
                    Access::ImmutVar { name: name.clone(), decl: var.decl }
                };
                Ok(Place { root: Root::Var(name.clone()), path: Vec::new(), access })
            }
            ExprKind::Deref(inner, check) => {
                let (p, outer) = if is_place(inner) {
                    let ip = self.place(inner)?;
                    (self.place_mut(&ip).clone(), ip.access)
                } else {
                    let (p, temp) = self.eval_borrow(inner)?;
                    if temp && matches!(p, Value::Own(_)) {
                        // 一時的な own の中に書き込んでも、すぐ捨てられてしまう
                        return Err(Diagnostic::new("一時的な own の中は場所として使えません")
                            .primary(inner.span, "先に変数に入れてください"));
                    }
                    (p, Access::Write)
                };
                let slot = self.check_ptr(&p, e.span, check.get())?;
                Ok(Place { root: Root::Slot(slot), path: Vec::new(), access: outer.through(&p) })
            }
            ExprKind::Field(base, name, check) => {
                let mut place = self.place(base)?;
                // 構造体を丸ごと複製しないよう、ここで必要なものだけ取り出す
                let here = match self.place_mut(&place) {
                    Value::Struct(def, _) => Ok(def.clone()),
                    v => Err(v.clone()),
                };
                match here {
                    Ok(def) => {
                        place.path.push(self.field_index(&def, name, e.span)?);
                        Ok(place)
                    }
                    Err(p @ (Value::Own(_) | Value::Ref(_))) => {
                        let slot = self.check_ptr(&p, e.span, check.get())?;
                        let idx = match &self.heap.slots[slot].value {
                            Some(Value::Struct(def, _)) => self.field_index(def, name, e.span)?,
                            Some(other) => return Err(not_a_struct(other, base.span)),
                            None => unreachable!("check_ptr returns live slots"),
                        };
                        let access = place.access.through(&p);
                        Ok(Place { root: Root::Slot(slot), path: vec![idx], access })
                    }
                    Err(other) => Err(not_a_struct(&other, base.span)),
                }
            }
            _ => unreachable!("only Var/Deref/Field are places"),
        }
    }

    fn place_mut(&mut self, place: &Place) -> &mut Value {
        let mut v = match &place.root {
            Root::Var(name) => match &mut self.find_var(name).expect("place var").state {
                VarState::Live(v) => v,
                VarState::Moved(_) => unreachable!("place() rejects moved vars"),
            },
            Root::Slot(s) => self.heap.slots[*s].value.as_mut().expect("live slot"),
        };
        for &i in &place.path {
            v = match v {
                Value::Struct(_, fields) => &mut fields[i],
                _ => unreachable!("place path goes through structs"),
            };
        }
        v
    }

    fn field_index(&self, def: &StructDef, name: &str, span: Span) -> R<usize> {
        def.field_index(name).ok_or_else(|| {
            Diagnostic::new(format!("`{}` にフィールド `{}` はありません", def.name, name))
                .primary(span, "このフィールドは定義されていません")
                .note(format!("`{}` のフィールド: {}", def.name, def.fields.join(", ")))
        })
    }

    fn field_of_slot(&self, slot: usize, name: &str, span: Span) -> R<&Value> {
        match self.heap.slots[slot].value.as_ref().expect("live slot") {
            Value::Struct(def, fields) => Ok(&fields[self.field_index(def, name, span)?]),
            other => Err(not_a_struct(other, span)),
        }
    }

    fn eval(&mut self, e: &Expr) -> R<Value> {
        match &e.kind {
            ExprKind::Int(n) => Ok(Value::Int(*n)),
            ExprKind::Bool(b) => Ok(Value::Bool(*b)),
            ExprKind::Str(s) => Ok(Value::Str(Rc::from(s.as_str()))),
            ExprKind::Var(name) => {
                let var = self.find_var(name).ok_or_else(|| undefined_var(name, e.span))?;
                match var.state.clone() {
                    // own を読む = ムーブ。元の変数は使えなくなる。
                    VarState::Live(v @ (Value::Own(_) | Value::Struct(..))) => {
                        var.state = VarState::Moved(e.span);
                        Ok(v)
                    }
                    VarState::Live(v) => Ok(v),
                    VarState::Moved(sp) => Err(moved_error(name, sp, e.span)),
                }
            }
            ExprKind::Own(inner) => {
                let v = self.eval(inner)?;
                Ok(Value::Own(self.heap.alloc(v)))
            }
            ExprKind::Ref(inner, mutable) => {
                if !is_place(inner) {
                    return Err(Diagnostic::new("借用できない式です")
                        .primary(inner.span, "`ref` は変数か `*p` か `p.f` にだけ使えます")
                        .note("`let x = own ...` で変数に入れてから `ref x` としてください"));
                }
                let (p, access) = if *mutable {
                    let pl = self.place(inner)?;
                    (self.place_mut(&pl).clone(), pl.access)
                } else {
                    (self.eval_borrow(inner)?.0, Access::Write)
                };
                if *mutable {
                    // own を ref mut で借りるには持ち主が mut、ref から借り直すにはその ref が ref mut
                    access.through(&p).require(e.span, "ref mut で借りようとした")?;
                }
                match p {
                    Value::Own(slot) => Ok(Value::Ref(RefVal {
                        slot,
                        generation: self.heap.slots[slot].generation,
                        borrowed_at: e.span,
                        what: describe(inner),
                        mutable: *mutable,
                    })),
                    Value::Ref(r) => Ok(Value::Ref(RefVal { mutable: *mutable, ..r })),
                    other => Err(Diagnostic::new("借用できない値です")
                        .primary(inner.span, format!("{} は own でも ref でもありません", other.type_name()))
                        .note("値を借りたいときは、まず `own` に入れてください")),
                }
            }
            ExprKind::Deref(..) => {
                let (v, _) = self.eval_borrow(e)?;
                if v.has_own() || matches!(v, Value::Struct(..)) {
                    return Err(Diagnostic::new(format!("ポインタの先にある {} は取り出せません", v.type_name()))
                        .primary(e.span, "所有権をここから動かすことはできない")
                        .note("`ref` で借りるか、`*p = ...` で置き換えてください"));
                }
                Ok(v)
            }
            ExprKind::Field(..) => {
                let (v, _) = self.eval_borrow(e)?;
                if v.has_own() || matches!(v, Value::Struct(..)) {
                    return Err(Diagnostic::new(format!("フィールドにある {} は取り出せません", v.type_name()))
                        .primary(e.span, "所有権をここから動かすことはできない")
                        .note("`ref` で借りるか、`p.f = ...` で置き換えてください"));
                }
                Ok(v)
            }
            ExprKind::StructLit(name, inits) => {
                let def = self.structs.get(name).cloned().ok_or_else(|| {
                    Diagnostic::new(format!("未定義の構造体 `{}`", name))
                        .primary(e.span, "この構造体は定義されていません")
                })?;
                // 書いた順に評価し、定義の順に並べる
                let mut slots: Vec<Option<Value>> = vec![None; def.fields.len()];
                for init in inits {
                    let idx = self.field_index(&def, &init.name, init.span)?;
                    if slots[idx].is_some() {
                        return Err(Diagnostic::new(format!("フィールド `{}` が二回書かれています", init.name))
                            .primary(init.span, "ここが二回目"));
                    }
                    slots[idx] = Some(self.eval(&init.value)?);
                }
                let missing: Vec<&str> = def
                    .fields
                    .iter()
                    .zip(&slots)
                    .filter(|(_, v)| v.is_none())
                    .map(|(n, _)| n.as_str())
                    .collect();
                if !missing.is_empty() {
                    return Err(Diagnostic::new(format!("`{}` のフィールドが足りません", name))
                        .primary(e.span, format!("{} がありません", missing.join(", "))));
                }
                Ok(Value::Struct(def, slots.into_iter().map(|v| v.expect("checked")).collect()))
            }
            ExprKind::Unary(op, inner) => {
                let v = self.eval(inner)?;
                match (op, v) {
                    (UnOp::Neg, Value::Int(n)) => n.checked_neg().map(Value::Int).ok_or_else(|| {
                        Diagnostic::new("整数オーバーフロー").primary(e.span, "符号反転で範囲を超えた")
                    }),
                    (UnOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
                    (_, other) => Err(Diagnostic::new("型が合わない演算です")
                        .primary(e.span, format!("{} にはこの演算を使えません", other.type_name()))),
                }
            }
            ExprKind::Binary(op, l, r) => {
                if matches!(op, BinOp::And | BinOp::Or) {
                    let lv = self.eval(l)?;
                    let lb = as_bool(lv, l.span)?;
                    if (*op == BinOp::And && !lb) || (*op == BinOp::Or && lb) {
                        return Ok(Value::Bool(lb));
                    }
                    let rv = self.eval(r)?;
                    return Ok(Value::Bool(as_bool(rv, r.span)?));
                }
                let lv = self.eval(l)?;
                let rv = self.eval(r)?;
                binary(*op, lv, rv, e.span)
            }
            ExprKind::Call(name, args) => self.eval_call(name, args, e.span),
            ExprKind::If { .. } | ExprKind::Match { .. } => {
                let (flow, v) = self.eval_flow(e, true)?;
                // 値として使う if / match の中では return / break / continue をパーサが禁止している
                debug_assert!(matches!(flow, Flow::Normal));
                Ok(v)
            }
        }
    }

    // ------------------------------------------------------------ 呼び出し

    fn eval_call(&mut self, name: &str, args: &[Expr], span: Span) -> R<Value> {
        match name {
            "print" => {
                let mut parts = Vec::new();
                let mut temps = Vec::new();
                for a in args {
                    let (v, temp) = self.eval_borrow(a)?;
                    parts.push(v.show());
                    if temp {
                        temps.push((v, a.span));
                    }
                }
                let line = parts.join(" ");
                self.output.push_str(&line);
                self.output.push('\n');
                if self.echo {
                    println!("{}", line);
                }
                for (v, sp) in temps {
                    self.release(v, sp, "print の引数として使い終わった一時的な値");
                }
                Ok(Value::Unit)
            }
            "drop" => {
                if args.len() != 1 {
                    return Err(arity_error("drop", 1, args.len(), span));
                }
                let v = self.eval(&args[0])?;
                let why = match &args[0].kind {
                    ExprKind::Var(n) => format!("`{}` が drop で明示的に解放された", n),
                    _ => "drop で明示的に解放された".to_string(),
                };
                self.release(v, span, &why);
                Ok(Value::Unit)
            }
            "assert" => {
                if args.len() != 1 {
                    return Err(arity_error("assert", 1, args.len(), span));
                }
                let v = self.eval(&args[0])?;
                if !as_bool(v, args[0].span)? {
                    return Err(Diagnostic::new("assert に失敗しました")
                        .primary(args[0].span, "この条件が false でした"));
                }
                Ok(Value::Unit)
            }
            _ => {
                let f = self.fns.get(name).cloned().ok_or_else(|| {
                    Diagnostic::new(format!("未定義の関数 `{}`", name))
                        .primary(span, "この関数は定義されていません")
                })?;
                if args.len() != f.params.len() {
                    return Err(arity_error(name, f.params.len(), args.len(), span));
                }
                let mut vals = Vec::new();
                for a in args {
                    vals.push(self.eval(a)?);
                }
                self.call_fn(&f, vals, span)
            }
        }
    }

    fn call_fn(&mut self, f: &Rc<FnDef>, args: Vec<Value>, call_span: Span) -> R<Value> {
        if self.depth >= MAX_DEPTH {
            return Err(Diagnostic::new("関数呼び出しが深すぎます")
                .primary(call_span, format!("呼び出しの深さが {} を超えました", MAX_DEPTH)));
        }
        self.depth += 1;
        let vars: Vec<Var> = f
            .params
            .iter()
            .zip(args)
            .map(|(p, v)| Var { name: p.name.clone(), state: VarState::Live(v), mutable: p.mutable, decl: p.span })
            .collect();
        self.frames.push(Frame { scopes: vec![vars] });

        // 戻り値の型がある関数では、本体の最後の `;` のない式が戻り値になる
        let (flow, tail) = self.exec_block_value(&f.body, f.ret.is_some())?;

        let frame = self.frames.pop().expect("frame");
        self.depth -= 1;
        let (value, span, how) = match flow {
            Flow::Return(v, sp) => (v, sp, "関数から return した"),
            Flow::Normal => (tail, f.body.end, "スコープを抜けた"),
            Flow::Break(_) | Flow::Continue(_) => unreachable!("parser rejects break/continue outside loops"),
        };
        for scope in frame.scopes {
            self.drop_scope(scope, span, how);
        }
        Ok(value)
    }
}

// ---------------------------------------------------------------- ヘルパ

fn describe(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Var(n) => n.clone(),
        ExprKind::Deref(inner, _) => format!("*{}", describe(inner)),
        ExprKind::Field(base, name, _) => format!("{}.{}", describe(base), name),
        _ => "式".to_string(),
    }
}

/// 場所として扱える式か（`ref` やフィールド代入の対象になれる）
fn is_place(e: &Expr) -> bool {
    matches!(e.kind, ExprKind::Var(_) | ExprKind::Deref(..) | ExprKind::Field(..))
}

fn not_a_struct(v: &Value, span: Span) -> Diagnostic {
    Diagnostic::new("フィールドを持たない値です")
        .primary(span, format!("{} は構造体ではありません", v.type_name()))
}

/// match で当てはまる case のブロックを探す（なければ None）
fn select_arm<'a>(v: &Value, scrut_span: Span, arms: &'a [MatchArm]) -> R<Option<&'a Block>> {
    if !matches!(v, Value::Int(_) | Value::Bool(_) | Value::Str(_)) {
        return Err(Diagnostic::new("match できるのは、今のところ整数・真偽値・文字列だけです")
            .primary(scrut_span, format!("ここは {} です", v.type_name())));
    }
    for arm in arms {
        for (pat, span) in &arm.pats {
            let hit = match (pat, v) {
                (Pat::Int(a), Value::Int(b)) => a == b,
                (Pat::Bool(a), Value::Bool(b)) => a == b,
                (Pat::Str(a), Value::Str(b)) => a.as_str() == &**b,
                _ => {
                    return Err(Diagnostic::new("case の値と match する値の型が合いません")
                        .label(scrut_span, format!("これは {}", v.type_name()))
                        .primary(*span, "この case は型が違う"))
                }
            };
            if hit {
                return Ok(Some(&arm.body));
            }
        }
    }
    Ok(None)
}

fn undefined_var(name: &str, span: Span) -> Diagnostic {
    Diagnostic::new(format!("未定義の変数 `{}`", name)).primary(span, "この名前は見つかりません")
}

fn arity_error(name: &str, want: usize, got: usize, span: Span) -> Diagnostic {
    Diagnostic::new(format!("`{}` の引数の数が違います", name))
        .primary(span, format!("{} 個必要ですが {} 個渡されました", want, got))
}

fn moved_error(name: &str, moved_at: Span, used_at: Span) -> Diagnostic {
    if moved_at == used_at {
        return Diagnostic::new(format!("ムーブ済みの値 `{}` を使いました (use after move)", name))
            .primary(
                used_at,
                format!("ここで `{}` の所有権を移すが、前の周回ですでに移動済み", name),
            )
            .note("ループの中で own を渡すと、2周目には手元に残っていません");
    }
    Diagnostic::new(format!("ムーブ済みの値 `{}` を使いました (use after move)", name))
        .label(moved_at, format!("ここで `{}` の所有権が移動した", name))
        .primary(used_at, format!("ここで `{}` を使ったが、もう所有していない", name))
        .note("移した後も使いたいなら `ref` で借りてください（例: `f(ref a)`）")
}

fn as_bool(v: Value, span: Span) -> R<bool> {
    match v {
        Value::Bool(b) => Ok(b),
        other => Err(Diagnostic::new("真偽値が必要です")
            .primary(span, format!("ここは {} です", other.type_name()))),
    }
}

fn binary(op: BinOp, l: Value, r: Value, span: Span) -> R<Value> {
    match (op, &l, &r) {
        (BinOp::Add, Value::Str(a), Value::Str(b)) => {
            let s = format!("{}{}", a, b);
            Ok(Value::Str(Rc::from(s.as_str())))
        }
        (BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem, Value::Int(a), Value::Int(b)) => {
            let res = match op {
                BinOp::Add => a.checked_add(*b),
                BinOp::Sub => a.checked_sub(*b),
                BinOp::Mul => a.checked_mul(*b),
                BinOp::Div | BinOp::Rem => {
                    if *b == 0 {
                        return Err(Diagnostic::new("0 で割りました")
                            .primary(span, "右辺が 0 です"));
                    }
                    if op == BinOp::Div {
                        a.checked_div(*b)
                    } else {
                        a.checked_rem(*b)
                    }
                }
                _ => unreachable!(),
            };
            res.map(Value::Int).ok_or_else(|| {
                Diagnostic::new("整数オーバーフロー").primary(span, "結果が i64 の範囲を超えました")
            })
        }
        (BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge, Value::Int(a), Value::Int(b)) => {
            Ok(Value::Bool(match op {
                BinOp::Lt => a < b,
                BinOp::Le => a <= b,
                BinOp::Gt => a > b,
                _ => a >= b,
            }))
        }
        (BinOp::Eq | BinOp::Ne, _, _) => {
            let eq = match (&l, &r) {
                (Value::Int(a), Value::Int(b)) => a == b,
                (Value::Bool(a), Value::Bool(b)) => a == b,
                (Value::Str(a), Value::Str(b)) => a == b,
                (Value::Unit, Value::Unit) => true,
                _ => {
                    return Err(Diagnostic::new("型が合わない比較です").primary(
                        span,
                        format!("{} と {} は比較できません", l.type_name(), r.type_name()),
                    ))
                }
            };
            Ok(Value::Bool(if op == BinOp::Eq { eq } else { !eq }))
        }
        _ => Err(Diagnostic::new("型が合わない演算です").primary(
            span,
            format!("{} と {} にはこの演算を使えません", l.type_name(), r.type_name()),
        )),
    }
}
