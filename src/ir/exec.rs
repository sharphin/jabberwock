//! IR を実行する小さなインタプリタ。今のインタプリタ（`interp.rs`）と出力・エラー・リークの有無を突き合わせて、
//! 書き出し（`lower`）が正しいことを確かめるためのもの。
//!
//! ヒープ（スロット＋世代）・表示・計算は `Interp` のものをそのまま使う。変数も interp と同じくスロットに置く
//! （`ref x` で変数そのものを借りられるように）。

use std::rc::Rc;

use super::*;
use crate::ast::UnOp;
use crate::diag::Diagnostic;
use crate::interp::{as_bool, binary, float_to_int, moved_error, str_method, Interp, RefVal, Value, MAX_DEPTH};

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
                    let (slot, path) = self.resolve(f, &locs, place)?;
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
                Term::Fail(title, span) => {
                    return Err(Diagnostic::new(title.clone()).primary(*span, "これだと永遠に終わりません"))
                }
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
                let (slot, path) = self.resolve(f, locs, place)?;
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
    fn resolve(&mut self, f: &Func, locs: &[Loc], place: &Place) -> R<(usize, Vec<usize>)> {
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
}

/// スロットの中の道筋の先にある値。interp の `at` と違い、enum の中身もたどる（match で受け取るとき）
fn at<'v>(it: &'v Interp, slot: usize, path: &[usize]) -> &'v Value {
    let mut v = it.heap.slots[slot].value.as_ref().expect("live slot");
    for &i in path {
        v = match v {
            Value::Struct(_, fields) | Value::Array(fields, _) | Value::Enum { fields, .. } => &fields[i],
            _ => unreachable!("paths go through structs, arrays and enums"),
        };
    }
    v
}

fn at_mut<'v>(it: &'v mut Interp, slot: usize, path: &[usize]) -> &'v mut Value {
    let mut v = it.heap.slots[slot].value.as_mut().expect("live slot");
    for &i in path {
        v = match v {
            Value::Struct(_, fields) | Value::Array(fields, _) | Value::Enum { fields, .. } => &mut fields[i],
            _ => unreachable!("paths go through structs, arrays and enums"),
        };
    }
    v
}
