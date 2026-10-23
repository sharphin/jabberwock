use std::cell::Cell;

use crate::diag::Span;

/// ソースに書いた型。`i64`、`Player`、`own T`、`ref T`、`ref mut T`
#[derive(Debug, Clone)]
pub struct TypeExpr {
    pub kind: TypeKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TypeKind {
    Name(String),
    /// `Pair<i64>`、`Option<T>`。型の引数つきの名前
    App(String, Vec<TypeExpr>),
    Own(Box<TypeExpr>),
    /// bool は `mut`
    Ref(Box<TypeExpr>, bool),
    /// `i64[3]`: 長さが決まった配列
    Array(Box<TypeExpr>, u64),
}

/// 整数の型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntTy {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    /// 配列の添字などに使う符号なし。幅は 64 ビットだが、u64 とは別の型
    Usize,
}

impl IntTy {
    pub fn from_name(n: &str) -> Option<IntTy> {
        Some(match n {
            "i8" => IntTy::I8,
            "i16" => IntTy::I16,
            "i32" => IntTy::I32,
            "i64" => IntTy::I64,
            "u8" => IntTy::U8,
            "u16" => IntTy::U16,
            "u32" => IntTy::U32,
            "u64" => IntTy::U64,
            "usize" => IntTy::Usize,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            IntTy::I8 => "i8",
            IntTy::I16 => "i16",
            IntTy::I32 => "i32",
            IntTy::I64 => "i64",
            IntTy::U8 => "u8",
            IntTy::U16 => "u16",
            IntTy::U32 => "u32",
            IntTy::U64 => "u64",
            IntTy::Usize => "usize",
        }
    }

    pub fn bits(self) -> u32 {
        match self {
            IntTy::I8 | IntTy::U8 => 8,
            IntTy::I16 | IntTy::U16 => 16,
            IntTy::I32 | IntTy::U32 => 32,
            IntTy::I64 | IntTy::U64 | IntTy::Usize => 64,
        }
    }

    pub fn signed(self) -> bool {
        matches!(self, IntTy::I8 | IntTy::I16 | IntTy::I32 | IntTy::I64)
    }

    pub fn min(self) -> i128 {
        if self.signed() {
            -(1i128 << (self.bits() - 1))
        } else {
            0
        }
    }

    pub fn max(self) -> i128 {
        if self.signed() {
            (1i128 << (self.bits() - 1)) - 1
        } else {
            (1i128 << self.bits()) - 1
        }
    }

    pub fn contains(self, v: i128) -> bool {
        self.min() <= v && v <= self.max()
    }

    /// 範囲の外の値を一周させて範囲に入れる（あふれ・`as` での変換）
    pub fn wrap(self, v: i128) -> i128 {
        let m = 1i128 << self.bits();
        let u = v.rem_euclid(m);
        if self.signed() && u > self.max() {
            u - m
        } else {
            u
        }
    }

    /// 値が欠けずに `to` へ変換できるか（自動で変換してよいか）。
    /// usize は環境で幅が変わりうる型として扱い、u8 / u16 / u32 からだけ受け取る
    pub fn widens_to(self, to: IntTy) -> bool {
        if self == to {
            return true;
        }
        if self == IntTy::Usize || to == IntTy::Usize {
            return to == IntTy::Usize && matches!(self, IntTy::U8 | IntTy::U16 | IntTy::U32);
        }
        match (self.signed(), to.signed()) {
            (false, false) | (true, true) => self.bits() <= to.bits(),
            (false, true) => self.bits() < to.bits(),
            (true, false) => false,
        }
    }

    /// 二つの整数の演算をどの型でするか（片方へ欠けずに変換できるなら広い方）
    pub fn common(a: IntTy, b: IntTy) -> Option<IntTy> {
        if a.widens_to(b) {
            Some(b)
        } else if b.widens_to(a) {
            Some(a)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
    /// 整数の式なら、型検査が決めた「この式の値を使う型」。実行時はこの型に変換してから使う
    /// （数の書き方を相手の型に合わせる・欠けない変換を自動でする、のため）
    pub int: Cell<Option<IntTy>>,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Expr {
        Expr { kind, span, int: Cell::new(None) }
    }
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

/// `*p` で ref の世代チェックが要るかどうか。静的解析（`analyze`）が書き込む。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Check {
    /// 解析がまだ見ていない（実行時に検査する）
    Unvisited,
    /// 「借りた先は必ず生きている」と証明できた。実行時の検査を消す
    Elided,
    /// 証明できなかった。実行時に検査する
    Required,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Int(i128),
    Bool(bool),
    Str(String),
    Var(String),
    /// `e as T`。整数の型を明示的に変える（範囲の外は一周する）
    Cast(Box<Expr>, TypeExpr),
    /// `e?`: e が `Err(x)` / `None` なら、関数からそれをそのまま return する。そうでなければ中身
    Try(Box<Expr>),
    /// `Shape.Circle(5)` / `Shape.Empty` / `Some(x)` / `None`。enum の値を作る。
    /// Option の enum 名は "Option"。`copy` は型検査が決める「値として読むとコピーか」
    /// （中に own や構造体を持ちうる enum は、構造体と同じく読むとムーブ）。
    /// None は型の引数を含んで決められないとき（実行時に中身を見て決める）
    Variant { enum_name: String, variant: String, args: Vec<Expr>, copy: Cell<Option<bool>> },
    /// `own e` : ヒープに値を置き、唯一の所有者を返す
    Own(Box<Expr>),
    /// `ref p` / `ref mut p` : 所有者（変数か `*p` か `p.f`）を世代付き参照として借りる。
    /// bool は `mut`（書き換えられる ref か）
    Ref(Box<Expr>, bool),
    /// `*p` : own / ref の先を読む。2つ目は静的解析の結論（世代チェックを消せるか）。
    Deref(Box<Expr>, Cell<Check>),
    /// `e.f` : フィールド。`e` が own / ref なら自動で先をたどる（そのときの検査が `Check`）。
    Field(Box<Expr>, String, Cell<Check>),
    /// `Name { f: e, ... }` : 構造体リテラル。フィールドはソースに書いた順
    StructLit(String, Vec<FieldInit>),
    /// `if c { ... } else { ... }`。文としても値としても使える。値はブロックの最後の行。
    /// 値として使う位置では else が必須（パーサが確かめる）。
    If { cond: Box<Expr>, then: Block, els: Option<Block> },
    /// `match x { case 1, 2 -> ... default -> ... }`（Java の switch の矢印形式）。
    /// 上から順に調べ、当てはまる case がなければ default。値として使う位置では default が必須。
    Match { scrut: Box<Expr>, arms: Vec<MatchArm>, default: Option<Block> },
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
    /// `a[i]`: 配列の要素。`a` が own / ref なら自動で先をたどる（そのときの検査が `Check`）
    Index(Box<Expr>, Box<Expr>, Cell<Check>),
    /// `[1, 2, 3]`
    ArrayLit(Vec<Expr>),
    /// `[0; 5]`: 同じ値を並べた配列（値はコピーできる型だけ）
    ArrayRepeat(Box<Expr>, u64),
    /// `"x = {x}"`: 埋め込みのある文字列。式は print と同じく借りて読む（ムーブしない）
    Interp(Vec<InterpPart>),
    /// `p.name(args)` : メソッド呼び出し。メソッドは `構造体名.name` という名前の関数で、
    /// 最初の引数 `this` に p を借りて渡す（p が own / ref ならその先、構造体の値なら一時的にヒープへ移して借りる）
    MethodCall(Box<Expr>, String, Vec<Expr>),
}

#[derive(Debug, Clone)]
pub enum InterpPart {
    Lit(String),
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub struct FieldInit {
    pub name: String,
    pub span: Span,
    pub value: Expr,
}

/// `case` に書ける値（今のところリテラルだけ）
#[derive(Debug, Clone, PartialEq)]
pub enum Pat {
    Int(i128),
    Bool(bool),
    Str(String),
    /// `Circle(r, _)` / `None`。enum の種類と、中身を受け取る名前（`_` は受け取らない）
    Variant(String, Vec<(String, Span)>),
}

impl Pat {
    /// 同じ場合を指すか（受け取る名前は問わない）
    pub fn same_case(&self, other: &Pat) -> bool {
        match (self, other) {
            (Pat::Variant(a, _), Pat::Variant(b, _)) => a == b,
            _ => self == other,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pats: Vec<(Pat, Span)>,
    /// `-> 式` も `-> { ... }` もブロックにそろえる。値はブロックの最後の行
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    /// 最後の文が `;` のない式で、ブロックの値になるか（Rust と同じ規則）
    pub tail: bool,
    /// 閉じ括弧 `}` の位置。スコープ終了での解放を指し示すのに使う。
    pub end: Span,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    /// `let 型 mut 名前 = 式`。型は省略できる（初期値から決める）
    Let { name: String, ty: Option<TypeExpr>, mutable: bool, span: Span, init: Expr },
    Assign { target: Expr, value: Expr, span: Span },
    /// 式文。`if` もここに入る（文として使う if）
    Expr(Expr),
    While { cond: Expr, body: Block },
    Loop { body: Block },
    /// `for i in range(a, b, step) { ... } else { ... }`。else は 1 回も回らなかったときに実行
    For { var: String, range: Vec<Expr>, range_span: Span, body: Block, els: Option<Block> },
    Break(Span),
    Continue(Span),
    Return { value: Option<Expr>, span: Span },
}

#[derive(Debug, Clone)]
pub struct Param {
    pub ty: TypeExpr,
    pub name: String,
    pub mutable: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    /// 型の引数（`fn T max<T>(...)` の `T`）
    pub tparams: Vec<String>,
    /// 型の引数ごとの条件（`<T: Shape + ordered>` の `Shape`, `ordered`）。`tparams` と同じ順
    pub bounds: Vec<Vec<String>>,
    /// 戻り値の型。なければ値を返さない関数
    pub ret: Option<TypeExpr>,
    pub params: Vec<Param>,
    pub body: Block,
    pub span: Span,
}

/// `struct Name { i64 a; own Point b; }`
#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub tparams: Vec<String>,
    pub fields: Vec<String>,
    /// `fields` と同じ順のフィールドの型
    pub field_types: Vec<TypeExpr>,
    pub span: Span,
}

impl StructDef {
    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|f| f == name)
    }
}

/// `enum Shape { Circle(i64 r); Rect(i64 w, i64 h); Empty; }`
#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub tparams: Vec<String>,
    pub variants: Vec<VariantDef>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct VariantDef {
    pub name: String,
    /// 中身（型 名前）
    pub fields: Vec<(TypeExpr, String)>,
    pub span: Span,
}

impl EnumDef {
    pub fn variant(&self, name: &str) -> Option<&VariantDef> {
        self.variants.iter().find(|v| v.name == name)
    }
}

/// `interface Shape { fn i64 area(); fn mut grow(i64 n); }`（Go 風: 構造体は宣言なしで当てはまる）
#[derive(Debug, Clone)]
pub struct InterfaceDef {
    pub name: String,
    /// メソッドの形（本体は空）と、`fn mut` か
    pub methods: Vec<(FnDef, bool)>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub structs: Vec<StructDef>,
    pub enums: Vec<EnumDef>,
    pub interfaces: Vec<InterfaceDef>,
    pub fns: Vec<FnDef>,
}
