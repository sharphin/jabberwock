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
    Int(IntTy),
    Bool,
    Str,
    /// 値なし（戻り値のない関数の呼び出しなど）
    Unit,
    /// 決して終わらない（return / break / continue で必ず抜ける）。どの型にも合う
    Never,
    /// 構造体・enum。2 つ目は型の引数（`Pair<i64>` の `i64`）
    Struct(String, Vec<Ty>),
    Enum(String, Vec<Ty>),
    /// 型の引数（`fn T max<T>` の中の `T`）。中身が何かは分からない
    Param(String),
    /// interface（`own Shape` / `ref Shape` の `Shape`）。当てはまる構造体のどれか。own か ref を通してだけ使う
    Iface(String),
    Own(Box<Ty>),
    /// bool は `mut`
    Ref(Box<Ty>, bool),
    /// `i64[3]`
    Array(Box<Ty>, u64),
}

impl Ty {
    pub fn show(&self) -> String {
        match self {
            Ty::Int(t) => t.name().to_string(),
            Ty::Bool => "bool".to_string(),
            Ty::Str => "str".to_string(),
            Ty::Unit => "値なし".to_string(),
            Ty::Never => "（終わらない）".to_string(),
            Ty::Struct(n, args) | Ty::Enum(n, args) if args.is_empty() => n.clone(),
            Ty::Struct(n, args) | Ty::Enum(n, args) => {
                let a: Vec<String> = args.iter().map(Ty::show).collect();
                format!("{}<{}>", n, a.join(", "))
            }
            Ty::Param(n) | Ty::Iface(n) => n.clone(),
            Ty::Own(t) => format!("own {}", t.show()),
            Ty::Ref(t, false) => format!("ref {}", t.show()),
            Ty::Ref(t, true) => format!("ref mut {}", t.show()),
            Ty::Array(t, n) => format!("{}[{}]", t.show(), n),
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

const I64: Ty = Ty::Int(IntTy::I64);

/// `from` の値を `to` の場所に入れられるか。`ref mut T` は `ref T` としても使える。
/// 整数は値が欠けない変換（u8 → i64 など）なら自動で変換する
fn assignable(from: &Ty, to: &Ty) -> bool {
    match (from, to) {
        (Ty::Never, _) => true,
        (Ty::Int(a), Ty::Int(b)) => a.widens_to(*b),
        (Ty::Ref(a, true), Ty::Ref(b, false)) => a == b,
        _ => from == to,
    }
}

fn mismatch(want: &Ty, got: &Ty, span: Span) -> Diagnostic {
    let d = Diagnostic::new("型が合いません")
        .primary(span, format!("ここは `{}` ですが、`{}` が必要です", got.show(), want.show()));
    if let (Ty::Int(_), Ty::Int(w)) = (got, want) {
        // 値が欠けうる変換は自動ではしない
        return d.note(format!("値が欠けるかもしれない変換は `as` で明示してください（例: `x as {}`）", w.name()));
    }
    d
}

/// 型の引数を具体的な型に置き換える（決まっていないものはそのまま）
fn subst(t: &Ty, map: &HashMap<String, Ty>) -> Ty {
    match t {
        Ty::Param(p) => map.get(p).cloned().unwrap_or_else(|| t.clone()),
        Ty::Struct(n, args) => Ty::Struct(n.clone(), args.iter().map(|a| subst(a, map)).collect()),
        Ty::Enum(n, args) => Ty::Enum(n.clone(), args.iter().map(|a| subst(a, map)).collect()),
        Ty::Own(x) => Ty::Own(Box::new(subst(x, map))),
        Ty::Ref(x, m) => Ty::Ref(Box::new(subst(x, map)), *m),
        Ty::Array(x, n) => Ty::Array(Box::new(subst(x, map)), *n),
        _ => t.clone(),
    }
}

fn contains_iface(t: &Ty) -> bool {
    match t {
        Ty::Iface(_) => true,
        Ty::Struct(_, args) | Ty::Enum(_, args) => args.iter().any(contains_iface),
        Ty::Own(x) | Ty::Ref(x, _) | Ty::Array(x, _) => contains_iface(x),
        _ => false,
    }
}

fn contains_param(t: &Ty) -> bool {
    match t {
        Ty::Param(_) => true,
        Ty::Struct(_, args) | Ty::Enum(_, args) => args.iter().any(contains_param),
        Ty::Own(x) | Ty::Ref(x, _) | Ty::Array(x, _) => contains_param(x),
        _ => false,
    }
}

/// `pattern` の中の型の引数（`tparams`）が、まだ決まっていないものを含むか
fn has_unbound(pattern: &Ty, tparams: &[String], map: &HashMap<String, Ty>) -> bool {
    match pattern {
        Ty::Param(p) => tparams.contains(p) && !map.contains_key(p),
        Ty::Struct(_, args) | Ty::Enum(_, args) => args.iter().any(|a| has_unbound(a, tparams, map)),
        Ty::Own(x) | Ty::Ref(x, _) | Ty::Array(x, _) => has_unbound(x, tparams, map),
        _ => false,
    }
}

/// 型の形を見比べて、型の引数が何かを決める（`T` と `i64` なら T = i64）。
/// 合わないところは放っておく（あとで型の食い違いとして見つかる）。
/// 整数どうしは、欠けずにそろえられる広い方にする（`max(a_u8, b_i64)` の T は i64）
fn unify(pattern: &Ty, actual: &Ty, tparams: &[String], map: &mut HashMap<String, Ty>) {
    match (pattern, actual) {
        (_, Ty::Never) | (_, Ty::Unit) => {}
        (Ty::Param(p), _) if tparams.contains(p) => match map.get(p) {
            None => {
                map.insert(p.clone(), actual.clone());
            }
            Some(Ty::Int(a)) => {
                if let Ty::Int(b) = actual {
                    if let Some(c) = IntTy::common(*a, *b) {
                        map.insert(p.clone(), Ty::Int(c));
                    }
                }
            }
            Some(_) => {}
        },
        (Ty::Struct(n, xs), Ty::Struct(m, ys)) | (Ty::Enum(n, xs), Ty::Enum(m, ys)) if n == m => {
            for (x, y) in xs.iter().zip(ys) {
                unify(x, y, tparams, map);
            }
        }
        (Ty::Own(x), Ty::Own(y)) | (Ty::Ref(x, _), Ty::Ref(y, _)) => unify(x, y, tparams, map),
        (Ty::Array(x, n), Ty::Array(y, m)) if n == m => unify(x, y, tparams, map),
        _ => {}
    }
}

/// 整数の値を、型 `want` の場所で使えるようにする（実行時に変換する印をつける）
fn convert(e: &Expr, got: &Ty, want: &Ty) {
    if let (Ty::Int(a), Ty::Int(b)) = (got, want) {
        if a != b {
            e.int.set(Some(*b));
        }
    }
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
            Access::ReadOnlyRef { what, .. } if what == "this" => Err(Diagnostic::new("書き換えないメソッドの中で書き換えようとしました")
                .primary(span, format!("ここで{}", doing))
                .note("フィールドを書き換えるメソッドは `fn mut 名前()` と書いてください")),
            Access::ReadOnlyRef { what, origin } => Err(Diagnostic::new("読むだけの ref を通して書き換えようとしました")
                .label(*origin, format!("`{}` は読むだけの ref（mut がない）", what))
                .primary(span, format!("ここで{}", doing))
                .note("書き換えるなら `ref mut` で借りてください")),
        }
    }
}

struct FnSig {
    /// 最初の引数が `this` のメソッドか（`p.m()` でだけ呼べる）
    method: bool,
    tparams: Vec<String>,
    /// 型の引数ごとの条件（`tparams` と同じ順）
    bounds: Vec<Vec<String>>,
    params: Vec<Ty>,
    ret: Ty,
}

struct Checker<'a> {
    structs: HashMap<&'a str, (&'a StructDef, Vec<Ty>)>,
    /// enum ごとの定義と、種類ごとの中身の型
    enums: HashMap<&'a str, (&'a EnumDef, Vec<Vec<Ty>>)>,
    fns: HashMap<&'a str, FnSig>,
    scopes: Vec<HashMap<String, VarInfo>>,
    /// 今調べている関数の戻り値の型
    ret: Ty,
    /// 今いる定義（関数・構造体・enum）の型の引数
    tparams: Vec<String>,
    /// interface ごとのメソッドの形
    interfaces: HashMap<&'a str, Vec<IMethod>>,
    /// 今調べている関数の、型の引数ごとの条件
    bounds: HashMap<String, Vec<String>>,
    warnings: Vec<Diagnostic>,
}

/// interface のメソッドの形（`this` を除いた引数）
#[derive(Clone)]
struct IMethod {
    name: String,
    mutable: bool,
    params: Vec<Ty>,
    ret: Ty,
}

/// 組み込みの条件: 大小を比べられる型（整数と str）
const ORDERED: &str = "ordered";

/// プログラム全体の型を調べる。よければ警告（実行は止めない）を返す
pub fn check(prog: &Program) -> R<Vec<Diagnostic>> {
    let mut c = Checker {
        structs: HashMap::new(),
        enums: HashMap::new(),
        fns: HashMap::new(),
        scopes: Vec::new(),
        ret: Ty::Unit,
        tparams: Vec::new(),
        interfaces: HashMap::new(),
        bounds: HashMap::new(),
        warnings: Vec::new(),
    };
    for s in &prog.structs {
        if c.structs.contains_key(s.name.as_str()) {
            return Err(Diagnostic::new(format!("構造体 `{}` が二重に定義されています", s.name))
                .primary(s.span, "ここで再定義されている"));
        }
        c.structs.insert(&s.name, (s, Vec::new()));
    }
    for en in &prog.enums {
        if c.structs.contains_key(en.name.as_str()) || c.enums.contains_key(en.name.as_str()) {
            return Err(Diagnostic::new(format!("型 `{}` が二重に定義されています", en.name))
                .primary(en.span, "同じ名前の型がすでにある"));
        }
        c.enums.insert(&en.name, (en, Vec::new()));
    }
    for i in &prog.interfaces {
        let n = i.name.as_str();
        if c.structs.contains_key(n) || c.enums.contains_key(n) || c.interfaces.contains_key(n) || n == ORDERED {
            return Err(Diagnostic::new(format!("型 `{}` が二重に定義されています", n))
                .primary(i.span, "同じ名前の型がすでにある"));
        }
        c.interfaces.insert(n, Vec::new());
    }
    for i in &prog.interfaces {
        c.tparams = Vec::new();
        let mut ms = Vec::new();
        for (m, mutable) in &i.methods {
            let params = m.params.iter().map(|p| c.resolve(&p.ty)).collect::<R<Vec<_>>>()?;
            let ret = match &m.ret {
                Some(t) => c.resolve(t)?,
                None => Ty::Unit,
            };
            ms.push(IMethod { name: m.name.clone(), mutable: *mutable, params, ret });
        }
        c.interfaces.insert(&i.name, ms);
    }
    for s in &prog.structs {
        c.tparams = s.tparams.clone();
        let tys = s.field_types.iter().map(|t| c.resolve(t)).collect::<R<Vec<_>>>()?;
        c.structs.get_mut(s.name.as_str()).expect("inserted").1 = tys;
    }
    for en in &prog.enums {
        c.tparams = en.tparams.clone();
        let mut all = Vec::new();
        for v in &en.variants {
            all.push(v.fields.iter().map(|(t, _)| c.resolve(t)).collect::<R<Vec<_>>>()?);
        }
        c.enums.get_mut(en.name.as_str()).expect("inserted").1 = all;
    }
    for s in &prog.structs {
        c.check_finite(&s.name, s.span)?;
    }
    for en in &prog.enums {
        c.check_finite(&en.name, en.span)?;
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
        for b in f.bounds.iter().flatten() {
            if b != ORDERED && !c.interfaces.contains_key(b.as_str()) {
                return Err(Diagnostic::new(format!("条件 `{}` は定義されていません", b))
                    .primary(f.span, "型の引数の条件には interface の名前か `ordered` を書きます"));
            }
        }
        c.tparams = f.tparams.clone();
        let params = f.params.iter().map(|p| c.resolve(&p.ty)).collect::<R<Vec<_>>>()?;
        let ret = match &f.ret {
            Some(t) => c.resolve(t)?,
            None => Ty::Unit,
        };
        c.fns.insert(&f.name, FnSig {
            method: f.name.contains('.') && matches!(f.params.first(), Some(p) if p.name == "this"),
            tparams: f.tparams.clone(),
            bounds: f.bounds.clone(),
            params,
            ret,
        });
    }
    for f in &prog.fns {
        c.check_fn(f)?;
    }
    Ok(c.warnings)
}

impl<'a> Checker<'a> {
    /// own / ref の先の型。interface の名前はここでだけ書ける
    fn resolve_pointee(&self, t: &TypeExpr) -> R<Ty> {
        match &t.kind {
            TypeKind::Name(n) if self.interfaces.contains_key(n.as_str()) && !self.tparams.contains(n) => {
                Ok(Ty::Iface(n.clone()))
            }
            _ => self.resolve(t),
        }
    }

    fn resolve(&self, t: &TypeExpr) -> R<Ty> {
        Ok(match &t.kind {
            TypeKind::Own(inner) => Ty::Own(Box::new(self.resolve_pointee(inner)?)),
            TypeKind::App(n, args) => {
                let want = match (self.structs.get(n.as_str()), self.enums.get(n.as_str())) {
                    (Some((d, _)), _) => d.tparams.len(),
                    (_, Some((d, _))) => d.tparams.len(),
                    _ => {
                        return Err(Diagnostic::new(format!("未定義の型 `{}`", n))
                            .primary(t.span, "この型は定義されていません"))
                    }
                };
                if args.len() != want {
                    return Err(Diagnostic::new(format!("`{}` の型の引数の数が違います", n))
                        .primary(t.span, format!("{} 個必要ですが {} 個書かれています", want, args.len())));
                }
                let args = args.iter().map(|a| self.resolve(a)).collect::<R<Vec<_>>>()?;
                if self.structs.contains_key(n.as_str()) {
                    Ty::Struct(n.clone(), args)
                } else {
                    Ty::Enum(n.clone(), args)
                }
            }
            TypeKind::Ref(inner, m) => Ty::Ref(Box::new(self.resolve_pointee(inner)?), *m),
            TypeKind::Array(inner, n) => Ty::Array(Box::new(self.resolve(inner)?), *n),
            TypeKind::Name(n) => match n.as_str() {
                "bool" => Ty::Bool,
                "str" => Ty::Str,
                _ if IntTy::from_name(n).is_some() => Ty::Int(IntTy::from_name(n).expect("checked")),
                "isize" | "f32" | "f64" => {
                    return Err(Diagnostic::new(format!("型 `{}` はまだ使えません", n))
                        .primary(t.span, "今使える数の型は整数（i8〜i64、u8〜u64、usize）だけです"))
                }
                _ if self.tparams.contains(n) => Ty::Param(n.clone()),
                _ if self.interfaces.contains_key(n.as_str()) => {
                    return Err(Diagnostic::new(format!("interface `{}` は own か ref を通して使います", n))
                        .primary(t.span, format!("`own {}` か `ref {}` と書いてください", n, n))
                        .note("中身の構造体によって大きさが違うので、そのままの値としては持てません"))
                }
                _ if self.structs.contains_key(n.as_str()) || self.enums.contains_key(n.as_str()) => {
                    let tps = match self.structs.get(n.as_str()) {
                        Some((d, _)) => &d.tparams,
                        None => &self.enums[n.as_str()].0.tparams,
                    };
                    if !tps.is_empty() {
                        return Err(Diagnostic::new(format!("`{}` には型の引数が必要です", n))
                            .primary(t.span, format!("`{}<{}>` のように書いてください", n, vec!["i64"; tps.len()].join(", "))));
                    }
                    if self.structs.contains_key(n.as_str()) {
                        Ty::Struct(n.clone(), Vec::new())
                    } else {
                        Ty::Enum(n.clone(), Vec::new())
                    }
                }
                _ => {
                    return Err(Diagnostic::new(format!("未定義の型 `{}`", n))
                        .primary(t.span, "この型は定義されていません")
                        .note("使える型: i8〜i64、u8〜u64、usize、bool、str、構造体の名前、own 型、ref 型、ref mut 型"))
                }
            },
        })
    }

    /// 構造体や enum が自分自身を（own / ref を通さず）じかに含んでいないか。含むと大きさが無限になる
    fn check_finite(&self, name: &str, span: Span) -> R<()> {
        let mut stack = vec![name.to_string()];
        let mut seen: Vec<String> = Vec::new();
        while let Some(n) = stack.pop() {
            let mut direct = Vec::new();
            for t in self.contents(&n) {
                by_value_names(t, &mut direct);
            }
            for m in direct {
                if m == name {
                    return Err(Diagnostic::new(format!("`{}` が自分自身をじかに含んでいます", name))
                        .primary(span, "これだと大きさが無限になる")
                        .note("`own` を通して持ってください（例: `own Node next;`）"));
                }
                if !seen.contains(&m) {
                    seen.push(m.clone());
                    stack.push(m);
                }
            }
        }
        Ok(())
    }

    /// 構造体のフィールド、または enum のすべての種類の中身の型
    fn contents(&self, name: &str) -> Vec<&Ty> {
        if let Some((_, tys)) = self.structs.get(name) {
            return tys.iter().collect();
        }
        match self.enums.get(name) {
            Some((_, vs)) => vs.iter().flatten().collect(),
            None => Vec::new(),
        }
    }

    /// 値として読んだときにコピーされる型か（own や構造体を中に持ちうるならムーブ）。
    /// 型の引数（`T`）は分からないのでコピーとは言えない
    fn copyable(&self, t: &Ty) -> bool {
        match t {
            Ty::Int(_) | Ty::Bool | Ty::Str | Ty::Ref(..) | Ty::Unit | Ty::Never => true,
            Ty::Own(_) | Ty::Struct(..) | Ty::Param(_) | Ty::Iface(_) => false,
            // 要素がコピーできる配列はコピー（Rust と同じ）
            Ty::Array(x, _) => self.copyable(x),
            // check_finite が自分をじかに含む enum を先に弾いているので、再帰は止まる
            Ty::Enum(..) => self.variants_of(t).expect("enum").iter().flat_map(|(_, f)| f).all(|t| self.copyable(t)),
        }
    }

    /// 定義の型の引数と、実際の型の引数の対応
    fn targs(tparams: &[String], args: &[Ty]) -> HashMap<String, Ty> {
        tparams.iter().cloned().zip(args.iter().cloned()).collect()
    }

    /// enum の種類の一覧: (名前, 中身の型)。型の引数は置き換え済み
    fn variants_of(&self, t: &Ty) -> Option<Vec<(String, Vec<Ty>)>> {
        match t {
            Ty::Enum(n, args) => {
                let (def, tys) = &self.enums[n.as_str()];
                let map = Self::targs(&def.tparams, args);
                Some(
                    def.variants
                        .iter()
                        .zip(tys)
                        .map(|(v, t)| (v.name.clone(), t.iter().map(|x| subst(x, &map)).collect()))
                        .collect(),
                )
            }
            _ => None,
        }
    }

    /// 型の引数を決めながら、引数（や中身）を調べる。`params` は定義の側の型（`T` を含みうる）。
    /// `pre` は (戻り値などの型, 入れる先の型) の組。先に見比べて型の引数を決める手がかりにする。
    /// 数や `None` のように相手しだいで型が決まる式は、ほかの引数で型の引数が決まってから調べる
    fn instantiate(
        &mut self,
        tparams: &[String],
        params: &[Ty],
        args: &[&Expr],
        pre: &[(&Ty, &Ty)],
    ) -> R<HashMap<String, Ty>> {
        let mut map = HashMap::new();
        for (pat, hint) in pre {
            unify(pat, hint, tparams, &mut map);
        }
        let mut got: Vec<Option<Ty>> = vec![None; args.len()];
        // 0: ふつうの式、1: 数の式、2: `None` など enum の値（中身の型を相手に合わせたいもの）
        let order = |a: &Expr| match a.kind {
            ExprKind::Variant { .. } => 2,
            _ if is_const_tree(a) => 1,
            _ => 0,
        };
        for pass in 0..3 {
            for (i, (a, p)) in args.iter().zip(params).enumerate() {
                if got[i].is_some() || order(a) != pass {
                    continue;
                }
                let hint = if has_unbound(p, tparams, &map) { None } else { Some(subst(p, &map)) };
                let t = self.expr_h(a, hint.as_ref())?;
                if t == Ty::Unit {
                    return Err(Diagnostic::new("値のない式です").primary(a.span, "これは値を返さない"));
                }
                unify(p, &t, tparams, &mut map);
                got[i] = Some(t);
            }
        }
        for ((a, p), t) in args.iter().zip(params).zip(&got) {
            let t = t.as_ref().expect("all arguments are checked");
            if has_unbound(p, tparams, &map) {
                continue;
            }
            let want = subst(p, &map);
            if !self.fits(t, &want) {
                return Err(mismatch(&want, t, a.span));
            }
            convert(a, t, &want);
        }
        Ok(map)
    }

    /// 型の引数がすべて決まったか。決まらなければエラー
    fn all_bound(&self, tparams: &[String], map: &HashMap<String, Ty>, span: Span, what: &str) -> R<Vec<Ty>> {
        tparams
            .iter()
            .map(|p| {
                map.get(p).cloned().ok_or_else(|| {
                    Diagnostic::new(format!("{} の型の引数 `{}` が決まりません", what, p))
                        .primary(span, "何の型か分からない")
                        .note("`let Option<i64> x = None;` のように、入れる先の型を書いてください")
                })
            })
            .collect()
    }

    /// `Shape.Circle(5)` / `Some(x)` / `None` の型。`hint` は値を入れる先の型
    fn variant_expr(&mut self, e: &Expr, hint: Option<&Ty>) -> R<Ty> {
        let ExprKind::Variant { enum_name, variant, args, copy } = &e.kind else { unreachable!() };
        let (def, tys) = &self.enums[enum_name.as_str()];
        let def: &EnumDef = def;
        let Some(i) = def.variants.iter().position(|v| v.name == *variant) else {
            let names: Vec<&str> = def.variants.iter().map(|v| v.name.as_str()).collect();
            return Err(Diagnostic::new(format!("`{}` に種類 `{}` はありません", enum_name, variant))
                .primary(e.span, "この種類は定義されていません")
                .note(format!("`{}` の種類: {}", enum_name, names.join(", "))));
        };
        let params = tys[i].clone();
        if args.len() != params.len() {
            return Err(arity_error(&format!("{}.{}", enum_name, variant), params.len(), args.len(), e.span));
        }
        let tparams = def.tparams.clone();
        let generic = Ty::Enum(enum_name.clone(), tparams.iter().map(|p| Ty::Param(p.clone())).collect());
        let pre = match hint {
            Some(h @ Ty::Enum(n, _)) if n == enum_name => Some((&generic, h)),
            _ => None,
        };
        let args: Vec<&Expr> = args.iter().collect();
        let map = self.instantiate(&tparams, &params, &args, &pre.into_iter().collect::<Vec<_>>())?;
        let targs = match self.all_bound(&tparams, &map, e.span, enum_name) {
            Ok(t) => t,
            Err(_) if enum_name == "Option" => {
                return Err(Diagnostic::new(format!("{} の型が決まりません", variant))
                    .primary(e.span, "何の Option か分からない")
                    .note("`let Option<i64> x = None;` のように型を書いてください"))
            }
            Err(d) => return Err(d),
        };
        let ty = Ty::Enum(enum_name.clone(), targs);
        // 型の引数を含む（ジェネリックな関数の中）なら、実行時に中身を見て決める
        copy.set(if contains_param(&ty) { None } else { Some(self.copyable(&ty)) });
        Ok(ty)
    }

    /// enum（または Option）の match。全部の種類を書いたかも調べる
    fn match_enum(
        &mut self,
        e: &Expr,
        st: &Ty,
        arms: &[MatchArm],
        default: Option<&Block>,
        want_value: bool,
        hint: Option<&Ty>,
    ) -> R<Ty> {
        // `match x`（値）か `match ref x` / ref の変数（借りる）か
        let (ety, by_ref) = match st {
            Ty::Ref(t, m) => ((**t).clone(), Some(*m)),
            Ty::Own(t) => ((**t).clone(), None),
            t => (t.clone(), None),
        };
        let variants = self.variants_of(&ety).expect("caller checked");
        let mut covered: Vec<String> = Vec::new();
        let mut tys = Vec::new();
        let mut hint = hint.cloned();
        for arm in arms {
            self.scopes.push(HashMap::new());
            let r = self.match_arm(&ety, &variants, by_ref, arm, &mut covered);
            let r = r.and_then(|_| self.block(&arm.body, want_value, hint.as_ref()));
            self.scopes.pop();
            let t = r?;
            // `case Some(x) -> Some(x); case None -> None;` の None の型を前の case から決める
            if hint.is_none() && matches!(&t, Ty::Enum(_, a) if !a.is_empty()) {
                hint = Some(t.clone());
            }
            tys.push((t, &arm.body));
        }
        if let Some(d) = default {
            tys.push((self.block(d, want_value, hint.as_ref())?, d));
        } else {
            let missing: Vec<&str> =
                variants.iter().map(|(n, _)| n.as_str()).filter(|n| !covered.iter().any(|c| c == n)).collect();
            if !missing.is_empty() {
                return Err(Diagnostic::new(format!("match で {} の場合が書かれていません", missing.join("・")))
                    .primary(e.span, format!("`{}` の種類をすべて書く必要がある", ety.show()))
                    .note("case を足すか、`default -> ...` でまとめてください"));
            }
        }
        self.join(tys, None, want_value, "match", hint.as_ref())
    }

    /// enum の case 1 つ分。受け取る名前を今のスコープに置く
    fn match_arm(
        &mut self,
        ety: &Ty,
        variants: &[(String, Vec<Ty>)],
        by_ref: Option<bool>,
        arm: &MatchArm,
        covered: &mut Vec<String>,
    ) -> R<()> {
        let several = arm.pats.len() > 1;
        for (pat, span) in &arm.pats {
            let Pat::Variant(name, binds) = pat else {
                return Err(Diagnostic::new("enum の match の case には種類の名前を書きます")
                    .primary(*span, format!("`{}` の種類ではない", ety.show())));
            };
            let Some((_, fields)) = variants.iter().find(|(n, _)| n == name) else {
                let names: Vec<&str> = variants.iter().map(|(n, _)| n.as_str()).collect();
                return Err(Diagnostic::new(format!("`{}` に種類 `{}` はありません", ety.show(), name))
                    .primary(*span, "この種類はない")
                    .note(format!("種類: {}", names.join(", "))));
            };
            if binds.len() != fields.len() {
                return Err(Diagnostic::new(format!("`{}` の中身の数が違います", name))
                    .primary(*span, format!("中身は {} 個ですが、{} 個の名前が書かれています", fields.len(), binds.len()))
                    .note("受け取らない中身には `_` と書いてください"));
            }
            covered.push(name.clone());
            for ((b, bspan), ft) in binds.iter().zip(fields) {
                if b == "_" {
                    continue;
                }
                if several {
                    return Err(Diagnostic::new("種類をまとめた case では中身を受け取れません")
                        .primary(*bspan, "ここ")
                        .note("case を分けるか、`_` にしてください"));
                }
                let t = match by_ref {
                    None => ft.clone(),
                    Some(_) if self.copyable(ft) => ft.clone(),
                    Some(m) => match ft {
                        Ty::Own(inner) => Ty::Ref(inner.clone(), m),
                        _ => {
                            return Err(Diagnostic::new("借りて match するとき、この中身は受け取れません")
                                .primary(*bspan, format!("中身は `{}`（own を通さずに持つ値）", ft.show()))
                                .note("中身を `own` で持たせるか、`_` で受け取らないでください"))
                        }
                    },
                };
                self.declare(b, VarInfo { ty: t, mutable: false, decl: *bspan });
            }
        }
        Ok(())
    }

    fn check_fn(&mut self, f: &FnDef) -> R<()> {
        let sig = &self.fns[f.name.as_str()];
        self.ret = sig.ret.clone();
        self.tparams = f.tparams.clone();
        self.bounds = f.tparams.iter().cloned().zip(f.bounds.iter().cloned()).collect();
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
        let got = self.block(&f.body, wants, Some(&ret))?;
        if wants && !self.fits(&got, &ret) {
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
        if let Some(t) = tail_expr(&f.body) {
            convert(t, &got, &ret);
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
    /// `hint` は値を入れる先の型（数の書き方をそれに合わせる）
    fn block(&mut self, b: &Block, want_value: bool, hint: Option<&Ty>) -> R<Ty> {
        self.scopes.push(HashMap::new());
        let r = self.block_inner(b, want_value, hint);
        self.scopes.pop();
        r
    }

    fn block_inner(&mut self, b: &Block, want_value: bool, hint: Option<&Ty>) -> R<Ty> {
        let mut diverges = false;
        for (i, s) in b.stmts.iter().enumerate() {
            match s {
                Stmt::Expr(e) if want_value && b.tail && i + 1 == b.stmts.len() => {
                    let t = self.expr_ctx(e, true, hint)?;
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
                let want = match ty {
                    Some(te) => Some(self.resolve(te)?),
                    None => None,
                };
                let got = self.expr_h(init, want.as_ref())?;
                if got == Ty::Unit {
                    return Err(Diagnostic::new("値のない式は変数に入れられません")
                        .primary(init.span, "これは値を返さない"));
                }
                let t = match (ty, want) {
                    (Some(te), Some(want)) => {
                        if !self.fits(&got, &want) {
                            return Err(mismatch(&want, &got, init.span).label(te.span, "ここで型を決めた"));
                        }
                        convert(init, &got, &want);
                        want
                    }
                    _ => got,
                };
                self.declare(name, VarInfo { ty: t, mutable: *mutable, decl: *span });
                Ok(false)
            }
            Stmt::Assign { target, value, .. } => {
                let want = self.expr(target)?;
                self.access(target)?.require(target.span, "書き換えようとした")?;
                self.expect(value, &want)?;
                Ok(false)
            }
            Stmt::Expr(e) => Ok(self.expr_ctx(e, false, None)? == Ty::Never),
            Stmt::While { cond, body } => {
                self.expect(cond, &Ty::Bool)?;
                self.block(body, false, None)?;
                Ok(false)
            }
            Stmt::Loop { body } => {
                self.block(body, false, None)?;
                Ok(!has_break(body))
            }
            Stmt::For { var, range, range_span, body, els } => {
                // 始めと終わりは同じ整数型にそろえる（それがループ変数の型）。刻みはいつも i64（負にもなるため）
                let ends = &range[..range.len().min(2)];
                let mut t: Option<IntTy> = None;
                for a in ends.iter().filter(|a| !is_const_tree(a)) {
                    let at = match self.expr(a)? {
                        Ty::Int(at) => at,
                        other => return Err(mismatch(&I64, &other, a.span)),
                    };
                    t = match t {
                        None => Some(at),
                        Some(prev) => match IntTy::common(prev, at) {
                            Some(c) => Some(c),
                            None => {
                                return Err(Diagnostic::new("range の始めと終わりの型が違います")
                                    .primary(*range_span, format!("`{}` と `{}`", prev.name(), at.name()))
                                    .note("`as` で型をそろえてください"))
                            }
                        },
                    };
                }
                let t = Ty::Int(t.unwrap_or(IntTy::I64));
                for a in ends {
                    self.expect(a, &t)?;
                }
                if let Some(step) = range.get(2) {
                    self.expect(step, &I64)?;
                }
                self.scopes.push(HashMap::new());
                self.declare(var, VarInfo { ty: t, mutable: false, decl: *range_span });
                let r = self.block(body, false, None);
                self.scopes.pop();
                r?;
                if let Some(els) = els {
                    self.block(els, false, None)?;
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
                    (Some(e), _) => self.expect(e, &ret)?,
                }
                Ok(true)
            }
        }
    }

    /// 型 `want` の場所で使う式。整数は欠けない変換なら自動で変換する
    fn expect(&mut self, e: &Expr, want: &Ty) -> R<()> {
        let got = self.expr_h(e, Some(want))?;
        if self.fits(&got, want) {
            convert(e, &got, want);
            Ok(())
        } else {
            Err(mismatch(want, &got, e.span))
        }
    }

    fn expr(&mut self, e: &Expr) -> R<Ty> {
        self.expr_ctx(e, true, None)
    }

    /// `hint` は値を入れる先の型（数の書き方をそれに合わせる）
    fn expr_h(&mut self, e: &Expr, hint: Option<&Ty>) -> R<Ty> {
        self.expr_ctx(e, true, hint)
    }

    /// 式の型。`want_value` が false なら、if / match は文として使われている（値を捨てる）。
    /// `hint` は値を入れる先の型。整数の定数（`300` や `1 + 2`）はその型になる（なければ i64）
    fn expr_ctx(&mut self, e: &Expr, want_value: bool, hint: Option<&Ty>) -> R<Ty> {
        if is_const_tree(e) {
            let t = match hint {
                Some(Ty::Int(t)) => *t,
                _ => IntTy::I64,
            };
            return self.const_expr(e, t);
        }
        match &e.kind {
            ExprKind::Int(_) => unreachable!("integer literals are constant trees"),
            ExprKind::Try(inner) => self.try_expr(inner, e.span, hint),
            ExprKind::Index(base, idx, _) => {
                // インタプリタと同じく、添字を先に調べる
                self.index_value(idx)?;
                let t = self.expr(base)?;
                let (elem, len) = match &t {
                    Ty::Array(x, n) => ((**x).clone(), *n),
                    Ty::Own(p) | Ty::Ref(p, _) => match &**p {
                        Ty::Array(x, n) => ((**x).clone(), *n),
                        _ => return Err(not_an_array(&t, base.span)),
                    },
                    _ => return Err(not_an_array(&t, base.span)),
                };
                if is_const_tree(idx) {
                    if let Ok(v) = const_val(idx) {
                        if v < 0 || v >= len as i128 {
                            return Err(Diagnostic::new(format!("添字 {} は配列の範囲の外です", v))
                                .primary(idx.span, format!("`{}` の添字は 0〜{} です", t.show(), len as i128 - 1)));
                        }
                    }
                }
                Ok(elem)
            }
            ExprKind::ArrayLit(items) => {
                let want = match hint {
                    Some(Ty::Array(x, n)) if *n == items.len() as u64 => Some((**x).clone()),
                    _ => None,
                };
                if let Some(w) = want {
                    for x in items {
                        self.expect(x, &w)?;
                    }
                    return Ok(Ty::Array(Box::new(w), items.len() as u64));
                }
                if items.is_empty() {
                    return Err(Diagnostic::new("空の配列の型が決まりません")
                        .primary(e.span, "要素の型が分からない")
                        .note("`let i64[0] a = [];` のように型を書いてください"));
                }
                // 要素の型をそろえる（数の書き方は、ほかの要素に合わせる）
                let mut tys: Vec<Option<Ty>> = vec![None; items.len()];
                let mut acc: Option<Ty> = None;
                for (i, x) in items.iter().enumerate() {
                    if is_const_tree(x) {
                        continue;
                    }
                    let t = self.value(x)?;
                    acc = Some(match acc {
                        None => t.clone(),
                        Some(Ty::Int(a)) => match (&t, IntTy::common(a, if let Ty::Int(b) = &t { *b } else { a })) {
                            (Ty::Int(_), Some(c)) => Ty::Int(c),
                            _ => return Err(mismatch(&Ty::Int(a), &t, x.span)),
                        },
                        Some(a) if self.fits(&t, &a) => a,
                        Some(a) => return Err(mismatch(&a, &t, x.span)),
                    });
                    tys[i] = Some(t);
                }
                let elem = acc.unwrap_or(I64);
                for (x, t) in items.iter().zip(tys) {
                    match t {
                        None => {
                            self.expect(x, &elem)?;
                        }
                        Some(t) => convert(x, &t, &elem),
                    }
                }
                Ok(Ty::Array(Box::new(elem), items.len() as u64))
            }
            ExprKind::ArrayRepeat(x, n) => {
                let h = match hint {
                    Some(Ty::Array(x, _)) => Some((**x).clone()),
                    _ => None,
                };
                let t = match &h {
                    Some(w) => {
                        self.expect(x, w)?;
                        w.clone()
                    }
                    None => self.value(x)?,
                };
                if !self.copyable(&t) {
                    return Err(Diagnostic::new(format!("`{}` は複製できないので並べられません", t.show()))
                        .primary(x.span, "own や構造体は 1 つしかない")
                        .note("`[a, b, c]` のように 1 つずつ書いてください"));
                }
                Ok(Ty::Array(Box::new(t), *n))
            }
            ExprKind::Interp(parts) => {
                // print と同じく、どんな値でも文字にできる
                for p in parts {
                    if let InterpPart::Expr(x) = p {
                        self.value(x)?;
                    }
                }
                Ok(Ty::Str)
            }
            ExprKind::Cast(inner, te) => {
                let target = self.resolve(te)?;
                let Ty::Int(t) = target else {
                    return Err(Diagnostic::new("`as` で変換できるのは整数の型だけです")
                        .primary(te.span, format!("`{}` は整数の型ではありません", target.show())));
                };
                let got = self.expr(inner)?;
                if !matches!(got, Ty::Int(_)) {
                    return Err(Diagnostic::new("`as` で変換できるのは整数だけです")
                        .primary(inner.span, format!("ここは `{}` です", got.show())));
                }
                Ok(Ty::Int(t))
            }
            ExprKind::Bool(_) => Ok(Ty::Bool),
            ExprKind::Str(_) => Ok(Ty::Str),
            ExprKind::Var(name) => match self.lookup(name) {
                Some(v) => Ok(v.ty.clone()),
                None => Err(Diagnostic::new(format!("未定義の変数 `{}`", name))
                    .primary(e.span, "この名前は見つかりません")),
            },
            ExprKind::Own(inner) => {
                let inner_hint = match hint {
                    Some(Ty::Own(t)) => Some(&**t),
                    _ => None,
                };
                let t = self.expr_h(inner, inner_hint)?;
                if t == Ty::Unit {
                    return Err(Diagnostic::new("値のない式です").primary(inner.span, "これは値を返さない"));
                }
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
                let (sname, targs) = match &t {
                    Ty::Struct(n, a) => (n, a),
                    Ty::Own(x) | Ty::Ref(x, _) => match &**x {
                        Ty::Struct(n, a) => (n, a),
                        _ => return Err(not_a_struct(&t, base.span)),
                    },
                    _ => return Err(not_a_struct(&t, base.span)),
                };
                let (def, tys) = &self.structs[sname.as_str()];
                match def.field_index(name) {
                    Some(i) => Ok(subst(&tys[i], &Self::targs(&def.tparams, targs))),
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
                let mut params = Vec::new();
                for init in inits {
                    let Some(i) = def.field_index(&init.name) else {
                        return Err(no_field(def, &init.name, init.span));
                    };
                    if seen[i] {
                        return Err(Diagnostic::new(format!("フィールド `{}` が二回書かれています", init.name))
                            .primary(init.span, "ここが二回目"));
                    }
                    seen[i] = true;
                    params.push(tys[i].clone());
                }
                let missing: Vec<&str> =
                    def.fields.iter().zip(&seen).filter(|(_, s)| !**s).map(|(n, _)| n.as_str()).collect();
                if !missing.is_empty() {
                    return Err(Diagnostic::new(format!("`{}` のフィールドが足りません", name))
                        .primary(e.span, format!("{} がありません", missing.join(", "))));
                }
                let generic = Ty::Struct(name.clone(), def.tparams.iter().map(|p| Ty::Param(p.clone())).collect());
                let pre = match hint {
                    Some(h @ Ty::Struct(n, _)) if n == name => Some((&generic, h)),
                    _ => None,
                };
                let values: Vec<&Expr> = inits.iter().map(|i| &i.value).collect();
                let map = self.instantiate(&def.tparams, &params, &values, &pre.into_iter().collect::<Vec<_>>())?;
                let targs = self.all_bound(&def.tparams, &map, e.span, name)?;
                Ok(Ty::Struct(name.clone(), targs))
            }
            ExprKind::If { cond, then, els } => {
                self.expect(cond, &Ty::Bool)?;
                let a = self.block(then, want_value, hint)?;
                // `if c { Some(1) } else { None }` の None の型を then から決める
                let else_hint = match (hint, &a) {
                    (None, Ty::Enum(_, args)) if !args.is_empty() => Some(&a),
                    _ => hint,
                };
                let mut tys = vec![(a.clone(), then)];
                if let Some(els) = els {
                    tys.push((self.block(els, want_value, else_hint)?, els));
                }
                let missing = if els.is_none() { Some(e.span) } else { None };
                self.join(tys, missing, want_value, "if", hint)
            }
            ExprKind::Variant { .. } => self.variant_expr(e, hint),
            ExprKind::Match { scrut, arms, default } => {
                let st = self.expr(scrut)?;
                let enum_like = |t: &Ty| matches!(t, Ty::Enum(..));
                let subject = match &st {
                    Ty::Ref(t, _) | Ty::Own(t) => &**t,
                    t => t,
                };
                if enum_like(subject) {
                    return self.match_enum(e, &st, arms, default.as_ref(), want_value, hint);
                }
                if !matches!(st, Ty::Int(_) | Ty::Bool | Ty::Str) {
                    return Err(Diagnostic::new("match できるのは、今のところ整数・真偽値・文字列・enum だけです")
                        .primary(scrut.span, format!("ここは `{}` です", st.show())));
                }
                let mut tys = Vec::new();
                for arm in arms {
                    for (pat, span) in &arm.pats {
                        let pt = match (pat, &st) {
                            (Pat::Int(v), Ty::Int(t)) => {
                                if !t.contains(*v) {
                                    return Err(out_of_range(*v, *t, *span));
                                }
                                st.clone()
                            }
                            (Pat::Int(_), _) => I64,
                            (Pat::Bool(_), _) => Ty::Bool,
                            (Pat::Str(_), _) => Ty::Str,
                            (Pat::Variant(n, _), _) => {
                                return Err(Diagnostic::new("enum ではない値に、種類の case は使えません")
                                    .label(scrut.span, format!("これは `{}`", st.show()))
                                    .primary(*span, format!("`{}` は種類の名前として読まれた", n)))
                            }
                        };
                        if pt != st {
                            return Err(Diagnostic::new("case の値と match する値の型が合いません")
                                .label(scrut.span, format!("これは `{}`", st.show()))
                                .primary(*span, format!("この case は `{}`", pt.show())));
                        }
                    }
                    tys.push((self.block(&arm.body, want_value, hint)?, &arm.body));
                }
                if let Some(d) = default {
                    tys.push((self.block(d, want_value, hint)?, d));
                }
                // default がなく、どの case にも当たらなければ何もせず抜ける
                let missing = if default.is_none() && !covers_both_bools(arms) { Some(e.span) } else { None };
                self.join(tys, missing, want_value, "match", hint)
            }
            ExprKind::Unary(UnOp::Not, inner) => {
                self.expect(inner, &Ty::Bool)?;
                Ok(Ty::Bool)
            }
            ExprKind::Unary(UnOp::Neg, inner) => match self.expr_h(inner, hint)? {
                Ty::Int(t) if t.signed() => Ok(Ty::Int(t)),
                Ty::Int(t) => Err(Diagnostic::new(format!("符号なしの型 `{}` に `-` は使えません", t.name()))
                    .primary(e.span, "ここ")
                    .note("符号つきの型（i64 など）に `as` で変換してください")),
                other => Err(mismatch(&I64, &other, inner.span)),
            },
            ExprKind::Binary(op, l, r) => self.binary(*op, l, r, e.span, hint),
            ExprKind::Call(name, args) => self.call(name, args, e.span, hint),
            ExprKind::MethodCall(recv, name, args) => self.method_call(recv, name, args, e.span, hint),
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

    /// if / match の枝の型をまとめる。`missing` は枝がない場合（else / default なし）の位置
    fn join(&self, branches: Vec<(Ty, &Block)>, missing: Option<Span>, want_value: bool, what: &str, hint: Option<&Ty>) -> R<Ty> {
        if !want_value {
            // 文として使うときは値を捨てる。どの枝も必ず抜けるなら、ここで必ず抜ける
            let all_never = missing.is_none() && branches.iter().all(|(t, _)| *t == Ty::Never);
            return Ok(if all_never { Ty::Never } else { Ty::Unit });
        }
        let mut list: Vec<(Ty, Span)> = branches.iter().map(|(t, b)| (t.clone(), tail_span(b))).collect();
        if let Some(sp) = missing {
            list.push((Ty::Unit, sp));
        }
        // 枝ごとに違う構造体でも、入れる先が interface でみな当てはまるなら、その interface にまとめる
        if let Some(h) = hint {
            if contains_iface(h) && list.iter().all(|(t, _)| self.fits(t, h)) {
                return Ok(h.clone());
            }
        }
        let mut acc = Ty::Never;
        let mut first: Option<(Ty, Span)> = None;
        for (t, span) in list {
            if self.fits(&t, &acc) {
                continue;
            }
            if self.fits(&acc, &t) {
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
        // 整数の枝は広い方の型にそろえる
        for (t, b) in &branches {
            if let Some(e) = tail_expr(b) {
                convert(e, t, &acc);
            }
        }
        Ok(acc)
    }

    fn binary(&mut self, op: BinOp, l: &Expr, r: &Expr, span: Span, hint: Option<&Ty>) -> R<Ty> {
        // 片方が定数（`x + 1` の `1`）なら、もう片方の型に合わせる。算術なら外の型も手がかりにする
        let arith = matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem);
        let outer = if arith { hint } else { None };
        let (lt, rt) = if is_const_tree(l) {
            let rt = self.expr_h(r, outer)?;
            let h = if matches!(rt, Ty::Int(_)) { Some(&rt) } else { outer };
            (self.expr_h(l, h)?, rt)
        } else {
            let lt = self.expr_h(l, outer)?;
            let h = if matches!(lt, Ty::Int(_)) { Some(&lt) } else { outer };
            let rt = self.expr_h(r, h)?;
            (lt, rt)
        };
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
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => match (&lt, &rt) {
                (Ty::Int(_), Ty::Int(_)) => Ok(Ty::Int(self.common(l, &lt, r, &rt, span)?)),
                _ => Err(bad()),
            },
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => match (&lt, &rt) {
                (Ty::Int(_), Ty::Int(_)) => {
                    self.common(l, &lt, r, &rt, span)?;
                    Ok(Ty::Bool)
                }
                (Ty::Str, Ty::Str) => Ok(Ty::Bool),
                _ if lt == rt && self.is_ordered_param(&lt) => Ok(Ty::Bool),
                (Ty::Param(_), _) | (_, Ty::Param(_)) => Err(bad().note("型の引数どうしを比べるには `<T: ordered>` と条件をつけてください")),
                _ => Err(bad()),
            },
            BinOp::Eq | BinOp::Ne => {
                if let (Ty::Int(_), Ty::Int(_)) = (&lt, &rt) {
                    self.common(l, &lt, r, &rt, span)?;
                    return Ok(Ty::Bool);
                }
                if lt == rt && self.is_ordered_param(&lt) {
                    return Ok(Ty::Bool);
                }
                if lt == rt && matches!(lt, Ty::Bool | Ty::Str) {
                    Ok(Ty::Bool)
                } else {
                    Err(Diagnostic::new("型が合わない比較です")
                        .primary(span, format!("`{}` と `{}` は比較できません", lt.show(), rt.show())))
                }
            }
        }
    }

    /// `from` の値を `to` の場所に入れられるか（`assignable` に、構造体 → interface を足したもの）
    fn fits(&self, from: &Ty, to: &Ty) -> bool {
        if assignable(from, to) {
            return true;
        }
        match (from, to) {
            (Ty::Ref(a, m), Ty::Ref(b, n)) if *m || !*n => self.fits_iface(a, b),
            (Ty::Own(a), Ty::Own(b)) => self.fits_iface(a, b),
            _ => false,
        }
    }

    /// 構造体（や条件つきの T）を interface として扱えるか
    fn fits_iface(&self, from: &Ty, to: &Ty) -> bool {
        match to {
            Ty::Iface(i) => !matches!(from, Ty::Iface(_)) && self.satisfies(from, i).is_ok(),
            _ => false,
        }
    }

    /// 添字の式。整数ならどの型でもよい（数の書き方は i64）
    fn index_value(&mut self, idx: &Expr) -> R<()> {
        match self.expr_h(idx, Some(&I64))? {
            Ty::Int(_) => Ok(()),
            t => Err(Diagnostic::new("添字は整数です").primary(idx.span, format!("ここは `{}` です", t.show()))),
        }
    }

    /// `<T: ordered>` の T か
    fn is_ordered_param(&self, t: &Ty) -> bool {
        matches!(t, Ty::Param(p) if matches!(self.bounds.get(p), Some(b) if b.iter().any(|x| x == ORDERED)))
    }

    /// 型 `t` が条件 `bound` を満たすか。満たさなければその理由
    fn satisfies(&self, t: &Ty, bound: &str) -> Result<(), String> {
        if let Ty::Iface(i) = t {
            return if i == bound { Ok(()) } else { Err(format!("interface `{}` は `{}` ではありません", i, bound)) };
        }
        if let Ty::Param(p) = t {
            // 型の引数のまま渡すなら、こちらの関数にも同じ条件が要る
            let ok = matches!(self.bounds.get(p), Some(b) if b.iter().any(|x| x == bound));
            return if ok { Ok(()) } else { Err(format!("`{}` に条件 `{}` がついていません", p, bound)) };
        }
        if bound == ORDERED {
            return if matches!(t, Ty::Int(_) | Ty::Str) {
                Ok(())
            } else {
                Err(format!("`{}` は大小を比べられません（ordered は整数と str だけ）", t.show()))
            };
        }
        let Ty::Struct(sname, targs) = t else {
            return Err(format!("`{}` は構造体ではないのでメソッドを持ちません", t.show()));
        };
        let map = Self::targs(&self.structs[sname.as_str()].0.tparams, targs);
        for im in &self.interfaces[bound] {
            let full = format!("{}.{}", sname, im.name);
            let Some(sig) = self.fns.get(full.as_str()).filter(|s| s.method) else {
                return Err(format!("`{}` にメソッド `{}` がありません", sname, im.name));
            };
            let Ty::Ref(_, smut) = &sig.params[0] else { unreachable!("this is a ref") };
            let params: Vec<Ty> = sig.params[1..].iter().map(|p| subst(p, &map)).collect();
            let shape_ok = sig.tparams.len() == self.structs[sname.as_str()].0.tparams.len()
                && params == im.params
                && subst(&sig.ret, &map) == im.ret;
            if !shape_ok {
                return Err(format!("`{}.{}` の引数か戻り値の型が `{}` の形と違います", sname, im.name, bound));
            }
            if *smut && !im.mutable {
                return Err(format!("`{}.{}` は fn mut ですが、`{}` では書き換えないメソッドです", sname, im.name, bound));
            }
        }
        Ok(())
    }

    /// 呼び出しで決まった型の引数が、それぞれの条件を満たすか
    fn check_bounds(&self, tparams: &[String], bounds: &[Vec<String>], map: &HashMap<String, Ty>, span: Span) -> R<()> {
        for (p, bs) in tparams.iter().zip(bounds) {
            let Some(t) = map.get(p) else { continue };
            for b in bs {
                if let Err(why) = self.satisfies(t, b) {
                    return Err(Diagnostic::new(format!("`{}` は条件 `{}` を満たしません", t.show(), b))
                        .primary(span, format!("ここで `{}` は `{}` になる", p, t.show()))
                        .note(why));
                }
            }
        }
        Ok(())
    }

    /// 型の引数（`T`）や interface の値に対するメソッド呼び出し。`ifaces` の interface から探す
    #[allow(clippy::too_many_arguments)]
    fn iface_method_call(
        &mut self,
        recv: &Expr,
        rt: &Ty,
        ifaces: &[String],
        label: &str,
        help: &str,
        name: &str,
        args: &[Expr],
        span: Span,
    ) -> R<Ty> {
        let found = ifaces.iter().find_map(|b| self.interfaces[b.as_str()].iter().find(|m| m.name == name).cloned());
        let Some(m) = found else {
            return Err(Diagnostic::new(format!("`{}` にメソッド `{}` があるか分かりません", label, name))
                .primary(span, "呼べるのは interface に書いたメソッドだけです")
                .note(help));
        };
        if args.len() != m.params.len() {
            return Err(arity_error(name, m.params.len(), args.len(), span));
        }
        if m.mutable {
            let a = if is_place(recv) { self.access(recv)? } else { Access::Write };
            a.through(rt, self.origin(recv), describe(recv))
                .require(span, format!("書き換えるメソッド `{}` を呼ぼうとした", name).as_str())?;
        }
        for (a, t) in args.iter().zip(&m.params) {
            self.expect(a, t)?;
        }
        Ok(m.ret)
    }

    /// `e?`。e が `Result<T, E>` なら関数の戻り値も `Result<_, E>`、`Option<T>` なら `Option<_>` でなければならない。
    /// 式の型は中身の `T`
    fn try_expr(&mut self, inner: &Expr, span: Span, hint: Option<&Ty>) -> R<Ty> {
        // 中身の型の手がかりを、戻り値の型と合わせて渡す（`let i64 x = Ok(5)?;` の 5 を i64 に）
        let inner_hint = match (&self.ret, hint) {
            (Ty::Enum(n, a), Some(h)) if n == "Result" => Some(Ty::Enum(n.clone(), vec![h.clone(), a[1].clone()])),
            (Ty::Enum(n, _), Some(h)) if n == "Option" => Some(Ty::Enum(n.clone(), vec![h.clone()])),
            _ => None,
        };
        let t = self.expr_h(inner, inner_hint.as_ref())?;
        let kind = match &t {
            Ty::Enum(n, _) if n == "Result" || n == "Option" => n.as_str(),
            _ => {
                return Err(Diagnostic::new("`?` は Result か Option に使います")
                    .primary(inner.span, format!("これは `{}` です", t.show())))
            }
        };
        let Ty::Enum(_, args) = &t else { unreachable!() };
        let usage = match kind {
            "Result" => "`?` は、Result を返す関数の中で使えます（Err ならそのまま返す）",
            _ => "`?` は、Option を返す関数の中で使えます（None ならそのまま返す）",
        };
        match &self.ret {
            Ty::Enum(m, rargs) if m == kind => {
                if kind == "Result" && args[1] != rargs[1] {
                    return Err(Diagnostic::new("エラーの型が合いません")
                        .primary(span, format!("ここの Err は `{}` ですが、関数が返す Err は `{}` です", args[1].show(), rargs[1].show()))
                        .note("`match` で取り出して、関数のエラーの型に作り直してください"));
                }
            }
            r => {
                return Err(Diagnostic::new("ここでは `?` を使えません")
                    .primary(span, format!("この関数の戻り値は `{}` です", r.show()))
                    .note(usage))
            }
        }
        Ok(args[0].clone())
    }

    fn call(&mut self, name: &str, args: &[Expr], span: Span, hint: Option<&Ty>) -> R<Ty> {
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
        if sig.method {
            let (s, m) = name.split_once('.').expect("method name");
            return Err(Diagnostic::new(format!("`{}` は this のあるメソッドです", name))
                .primary(span, "構造体名からは呼べない")
                .note(format!("`{}` の値 p に対して `p.{}()` と呼んでください", s, m)));
        }
        if args.len() != sig.params.len() {
            return Err(arity_error(name, sig.params.len(), args.len(), span));
        }
        let (tparams, params, ret) = (sig.tparams.clone(), sig.params.clone(), sig.ret.clone());
        let bounds = sig.bounds.clone();
        let pre = hint.map(|h| (&ret, h));
        let args: Vec<&Expr> = args.iter().collect();
        let map = self.instantiate(&tparams, &params, &args, &pre.into_iter().collect::<Vec<_>>())?;
        if contains_param(&ret) || params.iter().any(|p| has_unbound(p, &tparams, &map)) {
            self.all_bound(&tparams, &map, span, &format!("`{}`", name))?;
        }
        self.check_bounds(&tparams, &bounds, &map, span)?;
        Ok(subst(&ret, &map))
    }

    /// `p.name(args)`。メソッドは `構造体名.name` という関数で、最初の引数が `this`（`ref 構造体`）
    fn method_call(&mut self, recv: &Expr, name: &str, args: &[Expr], span: Span, hint: Option<&Ty>) -> R<Ty> {
        let rt = self.expr(recv)?;
        let inner = match &rt {
            Ty::Own(x) | Ty::Ref(x, _) => (**x).clone(),
            t => t.clone(),
        };
        match &inner {
            Ty::Array(..) => {
                if name != "len" || !args.is_empty() {
                    return Err(Diagnostic::new(format!("配列にメソッド `{}` はありません", name))
                        .primary(span, "配列のメソッドは `len()`（長さ）だけです"));
                }
                return Ok(I64);
            }
            Ty::Param(p) => {
                let ifaces: Vec<String> =
                    self.bounds.get(p).into_iter().flatten().filter(|b| *b != ORDERED).cloned().collect();
                let help = format!("`<{}: 名前>` のように、`{}` を持つ interface を条件にしてください", p, name);
                return self.iface_method_call(recv, &rt, &ifaces, p, &help, name, args, span);
            }
            Ty::Iface(i) => {
                let help = format!("interface `{}` に書いたメソッドだけ呼べます", i);
                return self.iface_method_call(recv, &rt, std::slice::from_ref(i), i, &help, name, args, span);
            }
            _ => {}
        }
        let st = match &rt {
            Ty::Struct(..) => rt.clone(),
            Ty::Own(x) | Ty::Ref(x, _) if matches!(**x, Ty::Struct(..)) => (**x).clone(),
            _ => {
                return Err(Diagnostic::new("メソッドを呼べるのは構造体だけです")
                    .primary(recv.span, format!("これは `{}` です", rt.show())))
            }
        };
        let Ty::Struct(sname, _) = &st else { unreachable!() };
        let full = format!("{}.{}", sname, name);
        let Some(sig) = self.fns.get(full.as_str()) else {
            let prefix = format!("{}.", sname);
            let mut names: Vec<&str> = self.fns.keys().filter_map(|k| k.strip_prefix(prefix.as_str())).collect();
            names.sort();
            let d = Diagnostic::new(format!("`{}` にメソッド `{}` はありません", sname, name))
                .primary(span, "このメソッドは定義されていません");
            return Err(if names.is_empty() {
                d.note(format!("`{}` にはメソッドがありません", sname))
            } else {
                d.note(format!("`{}` のメソッド: {}", sname, names.join(", ")))
            });
        };
        if !sig.method {
            return Err(Diagnostic::new(format!("`{}` は静的メソッドです", full))
                .primary(span, "値からは呼べない")
                .note(format!("`{}.{}(...)` と呼んでください", sname, name)));
        }
        if args.len() + 1 != sig.params.len() {
            return Err(arity_error(&format!("{}.{}", sname, name), sig.params.len() - 1, args.len(), span));
        }
        let (tparams, params, ret) = (sig.tparams.clone(), sig.params.clone(), sig.ret.clone());
        let Ty::Ref(this_ty, mutable) = &params[0] else { unreachable!("this is a ref") };
        if *mutable {
            // 書き換えるメソッド（`fn mut`）は、`ref mut` で借りられる相手にだけ呼べる
            let a = if is_place(recv) { self.access(recv)? } else { Access::Write };
            a.through(&rt, self.origin(recv), describe(recv))
                .require(span, format!("書き換えるメソッド `{}` を呼ぼうとした", name).as_str())?;
        }
        let mut pre: Vec<(&Ty, &Ty)> = vec![(this_ty, &st)];
        if let Some(h) = hint {
            pre.push((&ret, h));
        }
        let args: Vec<&Expr> = args.iter().collect();
        let map = self.instantiate(&tparams, &params[1..], &args, &pre)?;
        if contains_param(&ret) || params.iter().any(|p| has_unbound(p, &tparams, &map)) {
            self.all_bound(&tparams, &map, span, &format!("`{}`", name))?;
        }
        let bounds = self.fns[full.as_str()].bounds.clone();
        self.check_bounds(&tparams, &bounds, &map, span)?;
        Ok(subst(&ret, &map))
    }

    /// 二つの整数を同じ型（広い方）にそろえる。欠けない変換でそろわなければエラー
    fn common(&self, l: &Expr, lt: &Ty, r: &Expr, rt: &Ty, span: Span) -> R<IntTy> {
        let (Ty::Int(a), Ty::Int(b)) = (lt, rt) else { unreachable!("both sides are integers") };
        let Some(c) = IntTy::common(*a, *b) else {
            return Err(Diagnostic::new("型の違う整数どうしの演算です")
                .primary(span, format!("`{}` と `{}`", a.name(), b.name()))
                .note("どちらかに値が欠けずにそろえられないので、`as` で型をそろえてください"));
        };
        convert(l, lt, &Ty::Int(c));
        convert(r, rt, &Ty::Int(c));
        Ok(c)
    }

    /// 整数の定数の式（`300`、`-1`、`60 * 60`）を型 `t` として調べる。
    /// 数をそのまま書いて範囲を超えるならエラー、計算した結果が超えるなら警告（実行すると一周する）
    fn const_expr(&mut self, e: &Expr, t: IntTy) -> R<Ty> {
        set_tree(e, t);
        match const_val(e) {
            Err(ConstErr::DivZero(span)) => {
                self.warnings
                    .push(Diagnostic::warning("0 で割っています").primary(span, "実行するとここでエラーになる"));
            }
            Err(ConstErr::Overflow) => self.warn_overflow(e.span, t),
            Ok(v) if !t.contains(v) => {
                let bare = matches!(&e.kind, ExprKind::Int(_))
                    || matches!(&e.kind, ExprKind::Unary(UnOp::Neg, i) if matches!(i.kind, ExprKind::Int(_)));
                if bare {
                    return Err(out_of_range(v, t, e.span));
                }
                self.warn_overflow(e.span, t);
            }
            Ok(_) => {}
        }
        Ok(Ty::Int(t))
    }

    fn warn_overflow(&mut self, span: Span, t: IntTy) {
        self.warnings.push(
            Diagnostic::warning("整数があふれます")
                .primary(span, format!("この計算は {} の範囲を超える", t.name()))
                .note(format!(
                    "実行すると値は一周します（{} の範囲は {}〜{}。最大値 + 1 は最小値になる）",
                    t.name(),
                    t.min(),
                    t.max()
                )),
        );
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
            ExprKind::Field(base, ..) | ExprKind::Index(base, ..) => {
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

/// 整数の数だけでできた式か（`300`、`-1`、`60 * 60`）。値はコンパイルの時点で決まる
fn is_const_tree(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Int(_) => true,
        ExprKind::Unary(UnOp::Neg, inner) => is_const_tree(inner),
        ExprKind::Binary(BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem, l, r) => {
            is_const_tree(l) && is_const_tree(r)
        }
        _ => false,
    }
}

/// 定数の式の中の数をすべて型 `t` にする（実行時もその型で計算して一周する）
fn set_tree(e: &Expr, t: IntTy) {
    e.int.set(Some(t));
    match &e.kind {
        ExprKind::Unary(_, inner) => set_tree(inner, t),
        ExprKind::Binary(_, l, r) => {
            set_tree(l, t);
            set_tree(r, t);
        }
        _ => {}
    }
}

enum ConstErr {
    DivZero(Span),
    Overflow,
}

/// 定数の式の本当の値（一周させない）
fn const_val(e: &Expr) -> Result<i128, ConstErr> {
    match &e.kind {
        ExprKind::Int(n) => Ok(*n),
        ExprKind::Unary(_, inner) => Ok(-const_val(inner)?),
        ExprKind::Binary(op, l, r) => {
            let (a, b) = (const_val(l)?, const_val(r)?);
            match op {
                BinOp::Add => a.checked_add(b),
                BinOp::Sub => a.checked_sub(b),
                BinOp::Mul => a.checked_mul(b),
                BinOp::Div | BinOp::Rem if b == 0 => return Err(ConstErr::DivZero(e.span)),
                BinOp::Div => a.checked_div(b),
                _ => a.checked_rem(b),
            }
            .ok_or(ConstErr::Overflow)
        }
        _ => unreachable!("only constant trees"),
    }
}

fn out_of_range(v: i128, t: IntTy, span: Span) -> Diagnostic {
    Diagnostic::new(format!("`{}` は `{}` に入りません", v, t.name()))
        .primary(span, format!("{} の範囲は {}〜{}", t.name(), t.min(), t.max()))
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

/// ブロックの値になる最後の式
fn tail_expr(b: &Block) -> Option<&Expr> {
    match b.stmts.last() {
        Some(Stmt::Expr(e)) if b.tail => Some(e),
        _ => None,
    }
}

/// 型の中に（own / ref を通さず）じかに入っている構造体・enum の名前
fn by_value_names(t: &Ty, out: &mut Vec<String>) {
    match t {
        // 配列は要素をじかに含む
        Ty::Array(x, _) => by_value_names(x, out),
        Ty::Struct(n, args) | Ty::Enum(n, args) => {
            out.push(n.clone());
            // 型の引数も、じかに含まれるかもしれないとみなす（安全側）
            for a in args {
                by_value_names(a, out);
            }
        }
        _ => {}
    }
}

fn tail_span(b: &Block) -> Span {
    match b.stmts.last() {
        Some(Stmt::Expr(e)) if b.tail => e.span,
        _ => b.end,
    }
}

fn is_place(e: &Expr) -> bool {
    matches!(e.kind, ExprKind::Var(_) | ExprKind::Deref(..) | ExprKind::Field(..) | ExprKind::Index(..))
}

fn not_an_array(t: &Ty, span: Span) -> Diagnostic {
    Diagnostic::new("添字が使えるのは配列だけです").primary(span, format!("ここは `{}` です", t.show()))
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
