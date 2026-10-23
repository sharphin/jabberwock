//! IR を実行する小さなインタプリタ。今のインタプリタ（`interp.rs`）と出力・エラー・リークの有無を突き合わせて、
//! 書き出し（`lower`）が正しいことを確かめるためのもの。
//!
//! ヒープ（スロット＋世代）・表示・計算は `Interp` のものをそのまま使う。変数も interp と同じくスロットに置く
//! （`ref x` で変数そのものを借りられるように）。

use std::rc::Rc;

use super::*;
use crate::ast::UnOp;
use crate::diag::Diagnostic;
use crate::interp::{
    as_bool, binary, bounds, float_to_int, locate, merge_sort, moved_error, none, some, str_method, Interp, MapVal, RefVal,
    Value, MAX_DEPTH,
};

type R<T> = Result<T, Diagnostic>;

/// 変数の今の状態
#[derive(Clone)]
enum Loc {
    /// まだ値がない（宣言の前・drop のあと）
    Unset,
    Live(usize),
    /// ムーブ済み（どこでムーブしたか）
    Moved(Span),
}

/// `main` を実行する。出力・ヒープは `it` に残る
pub fn run(prog: &Program, it: &mut Interp) -> R<()> {
    let main = prog.by_name["main"];
    let mut m = Machine { prog, it, depth: 0 };
    m.call(main, Vec::new(), prog.fns[main].span).map(|_| ())
}

struct Machine<'a> {
    prog: &'a Program,
    it: &'a mut Interp,
    depth: usize,
}

impl<'a> Machine<'a> {
    fn call(&mut self, fi: usize, args: Vec<Value>, span: Span) -> R<Value> {
        if self.depth >= MAX_DEPTH {
            return Err(Diagnostic::new("関数呼び出しが深すぎます")
                .primary(span, format!("呼び出しの深さが {} を超えました", MAX_DEPTH)));
        }
        self.depth += 1;
        let prog = self.prog;
        let f = &prog.fns[fi];
        let mut locs = vec![Loc::Unset; f.locals.len()];
        for (i, a) in args.into_iter().enumerate() {
            locs[i] = Loc::Live(self.it.heap.alloc(a));
        }
        let mut bb = 0;
        loop {
            let block = &f.blocks[bb];
            for inst in &block.insts {
                self.inst(f, &mut locs, inst)?;
            }
            bb = match &block.term {
                Term::Goto(b) => *b,
                Term::Branch(c, t, e) => {
                    let v = self.operand(f, &mut locs, c)?;
                    if as_bool(v, f.span)? {
                        *t
                    } else {
                        *e
                    }
                }
                Term::Return(o) => {
                    let v = self.operand(f, &mut locs, o)?;
                    self.depth -= 1;
                    return Ok(v);
                }
                Term::Switch(place, arms, other) => {
                    let (slot, path) = self.resolve(f, &mut locs, place)?;
                    let Value::Enum { variant, .. } = at(self.it, slot, &path) else { unreachable!("switch on an enum") };
                    arms.iter().find(|(v, _)| *v == *variant).map_or(*other, |(_, b)| *b)
                }
                Term::Step { counter, step, next, done } => {
                    let Loc::Live(slot) = locs[*counter] else { unreachable!("loop counter is live") };
                    let st = match self.operand(f, &mut locs, step)? {
                        Value::Int(n, _) => n,
                        _ => unreachable!("range step is an integer"),
                    };
                    let v = self.it.heap.slots[slot].value.as_mut().expect("live slot");
                    let Value::Int(n, t) = v else { unreachable!("loop counter is an integer") };
                    let n2 = *n + st;
                    if t.contains(n2) {
                        *n = n2;
                        *next
                    } else {
                        *done
                    }
                }
                Term::Fail(title, span, label) => return Err(Diagnostic::new(title.clone()).primary(*span, label.clone())),
                Term::Unreachable => unreachable!("reached an unreachable block in {}", f.name),
            };
        }
    }

    fn inst(&mut self, f: &Func, locs: &mut [Loc], inst: &Inst) -> R<()> {
        match inst {
            Inst::Assign { place, value, span } => {
                let v = self.rvalue(f, locs, value)?;
                if place.proj.is_empty() {
                    match locs[place.local] {
                        Loc::Live(slot) => {
                            let old = self.it.heap.slots[slot].value.replace(v).expect("live slot");
                            let why = format!("`{}` に再代入されて、古い値が解放された", f.locals[place.local].name);
                            self.it.release(old, *span, &why);
                        }
                        Loc::Unset | Loc::Moved(_) => locs[place.local] = Loc::Live(self.it.heap.alloc(v)),
                    }
                    return Ok(());
                }
                let (slot, path) = self.resolve_in(f, locs, place, true)?;
                let old = std::mem::replace(at_mut(self.it, slot, &path), v);
                self.it.release(old, *span, "上書きされて、古い値が解放された");
            }
            Inst::Drop { local, span, why } => {
                if let Loc::Live(slot) = locs[*local] {
                    let v = self.it.vacate(slot, *span, why);
                    self.it.release(v, *span, why);
                }
                locs[*local] = Loc::Unset;
            }
            Inst::Discard { value, span, why } => {
                let v = self.operand(f, locs, value)?;
                self.it.release(v, *span, why);
            }
        }
        Ok(())
    }

    /// 場所が指すスロットと、その中の道筋。ref の先は世代チェックをする
    fn resolve(&mut self, f: &Func, locs: &mut [Loc], place: &Place) -> R<(usize, Vec<usize>)> {
        self.resolve_in(f, locs, place, false)
    }

    /// `create` なら、最後の辞書のキーがなければ入れる（`m[k] = v` の代入。値は呼び出し元が置く）
    fn resolve_in(&mut self, f: &Func, locs: &mut [Loc], place: &Place, create: bool) -> R<(usize, Vec<usize>)> {
        let mut slot = match locs[place.local] {
            Loc::Live(s) => s,
            Loc::Moved(at) => return Err(moved_error(&f.locals[place.local].name, at, place.span)),
            Loc::Unset => unreachable!("read of an unset local _{} in {}", place.local, f.name),
        };
        let mut path = Vec::new();
        for p in &place.proj {
            match p {
                Proj::Deref(check, span) => {
                    let ptr = at(self.it, slot, &path).clone();
                    let (s, pth) = self.it.check_ptr(&ptr, *span, *check)?;
                    slot = s;
                    path = pth;
                }
                Proj::Field(i, _) | Proj::Variant(_, i) => path.push(*i),
                Proj::Index(k, key_span, base_span) => {
                    let key = self.operand(f, locs, k)?;
                    let last = std::ptr::eq(p, place.proj.last().expect("non-empty"));
                    let c = at_mut(self.it, slot, &path);
                    let i = match c {
                        Value::Map(m) if create && last && m.find(&key).is_none() => {
                            m.insert(key, Value::Unit);
                            m.entries.len() - 1
                        }
                        c => locate(c, &key, *key_span, *base_span)?,
                    };
                    path.push(i);
                }
            }
        }
        Ok((slot, path))
    }

    fn operand(&mut self, f: &Func, locs: &mut [Loc], o: &Operand) -> R<Value> {
        match o {
            Operand::Const(c) => Ok(match c {
                Const::Unit => Value::Unit,
                Const::Int(n, t) => Value::Int(*n, *t),
                Const::Float(x) => Value::Float(*x),
                Const::Bool(b) => Value::Bool(*b),
                Const::Str(s) => Value::Str(s.clone()),
            }),
            Operand::Copy(place) => {
                let (slot, path) = self.resolve(f, locs, place)?;
                Ok(at(self.it, slot, &path).clone())
            }
            Operand::Move(l, span) => match locs[*l] {
                Loc::Live(slot) => {
                    let name = &f.locals[*l].name;
                    let v = self.it.vacate(slot, *span, &format!("`{}` がムーブされた", name));
                    locs[*l] = Loc::Moved(*span);
                    Ok(v)
                }
                Loc::Moved(at) => Err(moved_error(&f.locals[*l].name, at, *span)),
                Loc::Unset => unreachable!("move of an unset local _{} in {}", l, f.name),
            },
        }
    }

    fn operands(&mut self, f: &Func, locs: &mut [Loc], os: &[Operand]) -> R<Vec<Value>> {
        let mut out = Vec::new();
        for o in os {
            out.push(self.operand(f, locs, o)?);
        }
        Ok(out)
    }

    fn rvalue(&mut self, f: &Func, locs: &mut [Loc], r: &Rvalue) -> R<Value> {
        match r {
            Rvalue::Use(o) => self.operand(f, locs, o),
            Rvalue::Bin(op, a, b, span) => {
                let a = self.operand(f, locs, a)?;
                let b = self.operand(f, locs, b)?;
                binary(*op, a, b, *span)
            }
            Rvalue::Un(op, a, span) => match (op, self.operand(f, locs, a)?) {
                (UnOp::Neg, Value::Int(n, t)) => Ok(Value::Int(t.wrap(-n), t)),
                (UnOp::Neg, Value::Float(x)) => Ok(Value::Float(-x)),
                (UnOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
                (_, other) => Err(Diagnostic::new("型が合わない演算です")
                    .primary(*span, format!("{} にはこの演算を使えません", other.type_name()))),
            },
            Rvalue::Cast(a, ty) => Ok(match (self.operand(f, locs, a)?, ty) {
                (Value::Int(n, _), Ty::Int(t)) => Value::Int(t.wrap(n), *t),
                (Value::Int(n, _), Ty::Float) => Value::Float(n as f64),
                (Value::Float(x), Ty::Int(t)) => Value::Int(t.wrap(float_to_int(x)), *t),
                (v, _) => v,
            }),
            Rvalue::Ref { place, mutable, what, span } => {
                let (slot, path) = self.resolve(f, locs, place)?;
                Ok(match at(self.it, slot, &path) {
                    Value::Own(s) => {
                        let s = *s;
                        Value::Ref(RefVal {
                            slot: s,
                            generation: self.it.heap.slots[s].generation,
                            path: Vec::new(),
                            borrowed_at: *span,
                            what: what.clone(),
                            mutable: *mutable,
                        })
                    }
                    Value::Ref(r) => Value::Ref(RefVal { mutable: *mutable, ..r.clone() }),
                    _ => {
                        if self.it.through_list(slot, &path) {
                            return Err(Diagnostic::new("リスト・辞書の中の値は借りられません").primary(*span, "ここ"));
                        }
                        Value::Ref(RefVal {
                            slot,
                            generation: self.it.heap.slots[slot].generation,
                            path,
                            borrowed_at: *span,
                            what: what.clone(),
                            mutable: *mutable,
                        })
                    }
                })
            }
            Rvalue::Enum(variant, ops, copy) => {
                Ok(Value::Enum { variant: variant.clone(), fields: self.operands(f, locs, ops)?, copy: *copy })
            }
            Rvalue::Take(place) => {
                let (slot, path) = self.resolve(f, locs, place)?;
                Ok(std::mem::replace(at_mut(self.it, slot, &path), Value::Unit))
            }
            Rvalue::Array(items, list) => Ok(Value::Array(self.operands(f, locs, items)?, *list)),
            Rvalue::Repeat(x, n, list) => {
                let v = self.operand(f, locs, x)?;
                Ok(Value::Array(vec![v; *n as usize], *list))
            }
            Rvalue::Map(pairs, span) => {
                let mut m = Box::<MapVal>::default();
                for (k, v) in pairs {
                    let k = self.operand(f, locs, k)?;
                    let v = self.operand(f, locs, v)?;
                    if let Some(old) = m.insert(k, v) {
                        self.it.release(old, *span, "辞書の同じキーに後から入れた値で置き換えられた");
                    }
                }
                Ok(Value::Map(m))
            }
            Rvalue::SeqOp { name, place, args, span, arg_span } => {
                let vals = self.operands(f, locs, args)?;
                let (slot, path) = self.resolve(f, locs, place)?;
                self.seq_op(name, slot, &path, vals, *span, *arg_span)
            }
            Rvalue::Items { place, what, span } => {
                let (slot, path) = self.resolve(f, locs, place)?;
                let items: Vec<Value> = match at(self.it, slot, &path) {
                    Value::Array(items, _) => items.clone(),
                    Value::Map(m) => m.entries.iter().map(|(_, v)| v.clone()).collect(),
                    _ => unreachable!("for over an array, a list or a map"),
                };
                Ok(Value::Array(items.into_iter().map(|v| self.it.view_of(v, *span, what)).collect(), true))
            }
            Rvalue::Keys(place) => {
                let (slot, path) = self.resolve(f, locs, place)?;
                Ok(Value::Array(
                    match at(self.it, slot, &path) {
                        Value::Array(items, _) => (0..items.len()).map(|i| Value::Int(i as i128, IntTy::I64)).collect(),
                        Value::Map(m) => m.entries.iter().map(|(k, _)| k.clone()).collect(),
                        _ => unreachable!("for over an array, a list or a map"),
                    },
                    true,
                ))
            }
            Rvalue::Own(a) => {
                let v = self.operand(f, locs, a)?;
                Ok(Value::Own(self.it.heap.alloc(v)))
            }
            Rvalue::Struct(name, ops) => {
                let def = self.prog.structs[name].clone();
                Ok(Value::Struct(def, self.operands(f, locs, ops)?))
            }
            Rvalue::Call(fi, ops, span) => {
                let args = self.operands(f, locs, ops)?;
                self.call(*fi, args, *span)
            }
            Rvalue::Show(o, temp, span) => {
                let v = self.operand(f, locs, o)?;
                let shown = self.it.show(&v, *span, 0);
                if *temp {
                    self.it.release(v, *span, "使い終わった一時的な値");
                }
                Ok(Value::Str(Rc::from(shown?.as_str())))
            }
            Rvalue::Builtin(name, ops, span) if name.starts_with("str.") => {
                let mut vals = self.operands(f, locs, ops)?;
                let Value::Str(text) = vals.remove(0) else { unreachable!("string method on a string") };
                str_method(&text, &name[4..], vals, *span)
            }
            Rvalue::Builtin(name, ops, span) => {
                let vals = self.operands(f, locs, ops)?;
                let strs = || -> Vec<String> {
                    vals.iter()
                        .map(|v| match v {
                            Value::Str(s) => s.to_string(),
                            _ => unreachable!("shown values are strings"),
                        })
                        .collect()
                };
                match name.as_str() {
                    "print" => {
                        let line = strs().join(" ");
                        self.it.output.push_str(&line);
                        self.it.output.push('\n');
                        Ok(Value::Unit)
                    }
                    "concat" => Ok(Value::Str(Rc::from(strs().concat().as_str()))),
                    "assert" => {
                        if !as_bool(vals[0].clone(), *span)? {
                            return Err(Diagnostic::new("assert に失敗しました").primary(*span, "この条件が false でした"));
                        }
                        Ok(Value::Unit)
                    }
                    _ => unreachable!("unknown builtin {}", name),
                }
            }
        }
    }

    /// 配列・リスト・辞書の操作（interp の `eval_method` の配列・辞書の部分と同じ）
    fn seq_op(&mut self, name: &str, slot: usize, path: &[usize], mut vals: Vec<Value>, span: Span, arg_span: Span) -> R<Value> {
        if name == "sort" {
            let Value::Array(items, _) = at(self.it, slot, path) else { unreachable!("sort on an array") };
            let items = items.clone();
            let mut less = |_: &mut (), a: usize, b: usize| -> R<bool> {
                Ok(match (&items[a], &items[b]) {
                    (Value::Int(x, _), Value::Int(y, _)) => x < y,
                    (Value::Float(x), Value::Float(y)) => x < y,
                    (Value::Str(x), Value::Str(y)) => x < y,
                    _ => unreachable!("type checked: ordered elements"),
                })
            };
            let order = merge_sort(&mut (), (0..items.len()).collect(), &mut less)?;
            let Value::Array(items, _) = at_mut(self.it, slot, path) else { unreachable!() };
            let mut old: Vec<Option<Value>> = std::mem::take(items).into_iter().map(Some).collect();
            *items = order.into_iter().map(|i| old[i].take().expect("each index once")).collect();
            return Ok(Value::Unit);
        }
        let c = at_mut(self.it, slot, path);
        if let Value::Map(m) = c {
            return Ok(match name {
                "len" => Value::Int(m.entries.len() as i128, IntTy::I64),
                "contains" => Value::Bool(m.find(&vals[0]).is_some()),
                "get" => match m.find(&vals[0]) {
                    Some(i) => some(m.entries[i].1.clone()),
                    None => none(),
                },
                "remove" => match m.remove(&vals[0]) {
                    Some(v) => {
                        let copy = !v.moves();
                        Value::Enum { variant: Rc::from("Some"), fields: vec![v], copy }
                    }
                    None => none(),
                },
                "keys" => Value::Array(m.entries.iter().map(|(k, _)| k.clone()).collect(), true),
                _ => unreachable!("map method {}", name),
            });
        }
        let Value::Array(items, _) = c else { unreachable!("array method on an array") };
        Ok(match name {
            "len" => Value::Int(items.len() as i128, IntTy::I64),
            "push" => {
                items.push(vals.remove(0));
                Value::Unit
            }
            "pop" => match items.pop() {
                Some(v) => {
                    let copy = !v.moves();
                    Value::Enum { variant: Rc::from("Some"), fields: vec![v], copy }
                }
                None => none(),
            },
            "remove" => {
                let Value::Int(i, _) = vals[0] else { unreachable!("type checked") };
                let k = bounds(i, items.len(), arg_span)?;
                items.remove(k)
            }
            "insert" => {
                let Value::Int(i, _) = vals[0] else { unreachable!("type checked") };
                if i < 0 || i > items.len() as i128 {
                    let len = items.len();
                    let v = vals.remove(1);
                    self.it.release(v, span, "入れられずに捨てられた");
                    return Err(Diagnostic::new("insert の位置が範囲の外です")
                        .primary(arg_span, format!("位置は {} ですが、長さは {} です", i, len))
                        .note("入れられる位置は 0〜長さ です"));
                }
                items.insert(i as usize, vals.remove(1));
                Value::Unit
            }
            _ => unreachable!("array method {}", name),
        })
    }
}

/// スロットの中の道筋の先にある値。interp の `at` と違い、enum の中身もたどる（match で受け取るとき）
fn at<'v>(it: &'v Interp, slot: usize, path: &[usize]) -> &'v Value {
    let mut v = it.heap.slots[slot].value.as_ref().expect("live slot");
    for &i in path {
        v = match v {
            Value::Struct(_, fields) | Value::Array(fields, _) | Value::Enum { fields, .. } => &fields[i],
            Value::Map(m) => &m.entries[i].1,
            _ => unreachable!("paths go through structs, arrays, maps and enums"),
        };
    }
    v
}

fn at_mut<'v>(it: &'v mut Interp, slot: usize, path: &[usize]) -> &'v mut Value {
    let mut v = it.heap.slots[slot].value.as_mut().expect("live slot");
    for &i in path {
        v = match v {
            Value::Struct(_, fields) | Value::Array(fields, _) | Value::Enum { fields, .. } => &mut fields[i],
            Value::Map(m) => &mut m.entries[i].1,
            _ => unreachable!("paths go through structs, arrays, maps and enums"),
        };
    }
    v
}
