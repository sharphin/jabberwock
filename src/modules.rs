//! モジュール: 1 ファイルが 1 モジュール。`use geom;` で同じフォルダの geom.jw を読む。
//!
//! モジュールの中のトップレベルの名前（関数・構造体・enum・interface）は、`geom.area` のように
//! モジュール名をつけた名前に書き換える（`qualify`）。使う側の `geom.area` はパーサが 1 つの名前にまとめる
//! （`parser::merge_paths`。`pub` でない名前はそこでエラー）。なので、型検査・解析・実行は
//! 全部をまとめた 1 つのプログラムとして扱える。
//! `use` が輪になるのはエラー（読む順が決まらないため。Go と同じ）。

use std::collections::{HashMap, HashSet};

use crate::ast::*;
use crate::diag::{Diagnostic, Span};
use crate::lexer::{lex_file, Tok};
use crate::parser::{ModuleInfo, NameKind, Parser};

type R<T> = Result<T, Diagnostic>;

/// ファイルを読みながら、モジュールを順に解析する
pub struct Loader<'a> {
    read: &'a dyn Fn(&str) -> Option<String>,
    /// 読んだファイル（モジュール名、ソース）。番号が `Span.file`。0 番が最初のファイル（名前は空）
    pub files: Vec<(String, String)>,
    infos: HashMap<String, ModuleInfo>,
    /// 今読んでいる途中のモジュール（輪を見つけるため）
    loading: Vec<String>,
    parts: Vec<Program>,
}

impl<'a> Loader<'a> {
    /// `read` はモジュール名からソースを返す（見つからなければ None）
    pub fn new(read: &'a dyn Fn(&str) -> Option<String>) -> Self {
        Loader { read, files: Vec::new(), infos: HashMap::new(), loading: Vec::new(), parts: Vec::new() }
    }

    /// 最初のファイルと、そこから `use` でたどれるモジュールをすべて読み、1 つのプログラムにする
    pub fn load(&mut self, src: &str) -> R<Program> {
        self.files.push((String::new(), src.to_string()));
        self.module("", 0)?;
        let mut all = Program {
            structs: Vec::new(),
            enums: Vec::new(),
            interfaces: Vec::new(),
            fns: Vec::new(),
            uses: Vec::new(),
            public: Vec::new(),
            modules: self.files.iter().skip(1).map(|(n, _)| n.clone()).collect(),
            consts: Vec::new(),
        };
        for p in self.parts.drain(..) {
            all.structs.extend(p.structs);
            all.enums.extend(p.enums);
            all.interfaces.extend(p.interfaces);
            all.fns.extend(p.fns);
            all.consts.extend(p.consts);
        }
        Ok(all)
    }

    /// `file` 番目のファイルを、モジュール `name` として読む（使うモジュールを先に読む）
    fn module(&mut self, name: &str, file: usize) -> R<ModuleInfo> {
        let src = self.files[file].1.clone();
        let toks = lex_file(&src, file)?;
        let uses: Vec<(String, Span)> = toks
            .windows(2)
            .filter_map(|w| match (&w[0].tok, &w[1].tok) {
                (Tok::Use, Tok::Ident(n)) => Some((n.clone(), w[1].span)),
                _ => None,
            })
            .collect();
        self.loading.push(name.to_string());
        for (u, span) in &uses {
            if self.infos.contains_key(u) {
                continue;
            }
            if self.loading.contains(u) {
                return Err(Diagnostic::new(format!("モジュール `{}` の use が輪になっています", u))
                    .primary(*span, "ここで使うと、読む順が決まらない")
                    .note("共通の部分を別のモジュールに分けてください"));
            }
            let Some(text) = (self.read)(u) else {
                return Err(Diagnostic::new(format!("モジュール `{}` が見つかりません", u))
                    .primary(*span, format!("同じフォルダに {}.jw がない", u)));
            };
            self.files.push((u.clone(), text));
            let info = self.module(u, self.files.len() - 1)?;
            self.infos.insert(u.clone(), info);
        }
        self.loading.pop();
        let deps: HashMap<String, ModuleInfo> =
            uses.iter().map(|(u, _)| (u.clone(), self.infos[u].clone())).collect();
        let mut prog = Parser::with_modules(toks, &deps)?.parse_program()?;
        let info = info_of(&prog);
        if !name.is_empty() {
            qualify(&mut prog, name, &info);
            prog.consts = prog.consts.iter().map(|c| format!("{}.{}", name, c)).collect();
        }
        self.parts.push(prog);
        Ok(info)
    }
}

/// モジュールのトップレベルの名前（書き換える前）
fn info_of(p: &Program) -> ModuleInfo {
    let mut names = HashMap::new();
    let public = |n: &str| p.public.iter().any(|x| x == n);
    // メソッド（`Point.len`）はトップレベルの名前ではない
    for f in p.fns.iter().filter(|f| !f.name.contains('.')) {
        names.insert(f.name.clone(), (NameKind::Fn, public(&f.name)));
    }
    for s in &p.structs {
        names.insert(s.name.clone(), (NameKind::Struct, public(&s.name)));
    }
    let mut variants = HashMap::new();
    for e in &p.enums {
        names.insert(e.name.clone(), (NameKind::Enum, public(&e.name)));
        variants.insert(e.name.clone(), e.variants.iter().map(|v| v.name.clone()).collect());
    }
    for i in &p.interfaces {
        names.insert(i.name.clone(), (NameKind::Interface, public(&i.name)));
    }
    ModuleInfo { names, variants }
}

/// モジュールの中の名前を、モジュール名をつけた名前に書き換える
struct Qualify<'a> {
    module: &'a str,
    fns: HashSet<String>,
    structs: HashSet<String>,
    /// 型として書ける名前（構造体・enum・interface）
    types: HashSet<String>,
    enums: HashSet<String>,
}

fn qualify(p: &mut Program, module: &str, info: &ModuleInfo) {
    let of = |k: NameKind| -> HashSet<String> {
        info.names.iter().filter(|(_, (kind, _))| *kind == k).map(|(n, _)| n.clone()).collect()
    };
    let q = Qualify {
        module,
        fns: of(NameKind::Fn),
        structs: of(NameKind::Struct),
        enums: of(NameKind::Enum),
        types: info
            .names
            .iter()
            .filter(|(_, (k, _))| *k != NameKind::Fn)
            .map(|(n, _)| n.clone())
            .collect(),
    };
    for s in &mut p.structs {
        s.name = q.name(&s.name);
        for t in &mut s.field_types {
            q.ty(t, &s.tparams);
        }
    }
    for e in &mut p.enums {
        e.name = q.name(&e.name);
        for v in &mut e.variants {
            for (t, _) in &mut v.fields {
                q.ty(t, &e.tparams);
            }
        }
    }
    for i in &mut p.interfaces {
        i.name = q.name(&i.name);
        for (m, _) in &mut i.methods {
            q.sig(m);
        }
    }
    for f in &mut p.fns {
        f.name = match f.name.split_once('.') {
            // メソッド・静的メソッド（`Point.len`）は構造体の名前を書き換える
            Some((s, m)) => format!("{}.{}", q.name(s), m),
            None => q.name(&f.name),
        };
        q.sig(f);
        let tps = f.tparams.clone();
        q.block(&mut f.body, &tps);
    }
}

impl<'a> Qualify<'a> {
    fn name(&self, n: &str) -> String {
        format!("{}.{}", self.module, n)
    }

    /// 関数の形（引数・戻り値の型と、型の引数の条件）
    fn sig(&self, f: &mut FnDef) {
        for p in &mut f.params {
            self.ty(&mut p.ty, &f.tparams);
        }
        if let Some(r) = &mut f.ret {
            self.ty(r, &f.tparams);
        }
        for bs in &mut f.bounds {
            for b in bs {
                if self.types.contains(b.as_str()) {
                    *b = self.name(b);
                }
            }
        }
    }

    /// 型の中の名前。型の引数（`tps`）は書き換えない
    fn ty(&self, t: &mut TypeExpr, tps: &[String]) {
        match &mut t.kind {
            TypeKind::Name(n) | TypeKind::App(n, _) if self.types.contains(n.as_str()) && !tps.contains(n) => {
                *n = self.name(n);
            }
            _ => {}
        }
        match &mut t.kind {
            TypeKind::Name(_) => {}
            TypeKind::App(_, args) => args.iter_mut().for_each(|a| self.ty(a, tps)),
            TypeKind::Own(x) | TypeKind::Ref(x, _) | TypeKind::Array(x, _) | TypeKind::List(x) => self.ty(x, tps),
            TypeKind::Map(v, k) => {
                self.ty(v, tps);
                self.ty(k, tps);
            }
            TypeKind::Fn(ps, r) => {
                ps.iter_mut().for_each(|p| self.ty(p, tps));
                if let Some(r) = r {
                    self.ty(r, tps);
                }
            }
        }
    }

    fn block(&self, b: &mut Block, tps: &[String]) {
        for s in &mut b.stmts {
            self.stmt(s, tps);
        }
    }

    fn stmt(&self, s: &mut Stmt, tps: &[String]) {
        match s {
            Stmt::Let { ty, init, .. } => {
                if let Some(t) = ty {
                    self.ty(t, tps);
                }
                self.expr(init, tps);
            }
            Stmt::Assign { target, value, .. } => {
                self.expr(target, tps);
                self.expr(value, tps);
            }
            Stmt::Expr(e) => self.expr(e, tps),
            Stmt::While { cond, body } => {
                self.expr(cond, tps);
                self.block(body, tps);
            }
            Stmt::Loop { body } => self.block(body, tps),
            Stmt::For { range, body, els, .. } => {
                range.iter_mut().for_each(|a| self.expr(a, tps));
                self.block(body, tps);
                if let Some(els) = els {
                    self.block(els, tps);
                }
            }
            Stmt::ForEach { iter, body, els, .. } => {
                self.expr(iter, tps);
                self.block(body, tps);
                if let Some(els) = els {
                    self.block(els, tps);
                }
            }
            Stmt::Break(_) | Stmt::Continue(_) => {}
            Stmt::Return { value, .. } => {
                if let Some(e) = value {
                    self.expr(e, tps);
                }
            }
        }
    }

    fn expr(&self, e: &mut Expr, tps: &[String]) {
        match &mut e.kind {
            ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Bool(_) | ExprKind::Str(_) | ExprKind::Unit => {}
            // 名前のある関数を値として使う（変数に関数と同じ名前はつけられないので、関数の名前なら関数）
            ExprKind::Var(n) => {
                if self.fns.contains(n.as_str()) {
                    *n = self.name(n);
                }
            }
            ExprKind::Call(n, args) => {
                let static_call = matches!(n.split_once('.'), Some((s, _)) if self.structs.contains(s));
                if self.fns.contains(n.as_str()) || static_call {
                    *n = self.name(n);
                }
                args.iter_mut().for_each(|a| self.expr(a, tps));
            }
            ExprKind::StructLit(n, inits) => {
                if self.structs.contains(n.as_str()) {
                    *n = self.name(n);
                }
                inits.iter_mut().for_each(|i| self.expr(&mut i.value, tps));
            }
            ExprKind::Variant { enum_name, args, .. } => {
                if self.enums.contains(enum_name.as_str()) {
                    *enum_name = self.name(enum_name);
                }
                args.iter_mut().for_each(|a| self.expr(a, tps));
            }
            ExprKind::Cast(x, t) => {
                self.expr(x, tps);
                self.ty(t, tps);
            }
            ExprKind::Try(x) | ExprKind::Own(x) | ExprKind::Ref(x, _) | ExprKind::Deref(x, _) => self.expr(x, tps),
            ExprKind::Field(x, ..) | ExprKind::Unary(_, x) | ExprKind::ArrayRepeat(x, ..) => self.expr(x, tps),
            ExprKind::If { cond, then, els } => {
                self.expr(cond, tps);
                self.block(then, tps);
                if let Some(els) = els {
                    self.block(els, tps);
                }
            }
            ExprKind::Match { scrut, arms, default } => {
                self.expr(scrut, tps);
                arms.iter_mut().for_each(|a| self.block(&mut a.body, tps));
                if let Some(d) = default {
                    self.block(d, tps);
                }
            }
            ExprKind::Binary(_, l, r) | ExprKind::Index(l, r, _) => {
                self.expr(l, tps);
                self.expr(r, tps);
            }
            ExprKind::ArrayLit(items, _) => items.iter_mut().for_each(|x| self.expr(x, tps)),
            ExprKind::MapLit(pairs) => {
                for (k, v) in pairs {
                    self.expr(k, tps);
                    self.expr(v, tps);
                }
            }
            ExprKind::Interp(parts) => {
                for p in parts {
                    if let InterpPart::Expr(x) = p {
                        self.expr(x, tps);
                    }
                }
            }
            ExprKind::Lambda(l) => {
                let l = std::rc::Rc::get_mut(l).expect("lambdas are not shared before type checking");
                for p in &mut l.params {
                    if let Some(t) = &mut p.ty {
                        self.ty(t, tps);
                    }
                }
                self.block(&mut l.body, tps);
            }
            ExprKind::MethodCall(recv, _, args, _) => {
                self.expr(recv, tps);
                args.iter_mut().for_each(|a| self.expr(a, tps));
            }
        }
    }
}
