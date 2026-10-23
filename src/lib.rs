//! ward: `own` と世代付き `ref` だけで安全を目指す小さな言語の核。

pub mod analyze;
pub mod ast;
pub mod diag;
pub mod interp;
pub mod lexer;
pub mod parser;
pub mod types;

use diag::Diagnostic;

/// 実行結果
pub struct Outcome {
    /// `print` が出した内容
    pub output: String,
    /// エラーがあれば診断
    pub error: Option<Diagnostic>,
    /// 警告（実行は止めない）
    pub warnings: Vec<Diagnostic>,
    /// 実行後に解放されずに残ったヒープオブジェクト数（0 ならリークなし）
    pub live_objects: usize,
    /// 静的解析の結果（`*p` のうち検査を消せた箇所・残った箇所）
    pub analysis: analyze::Stats,
    /// 実行時に行った ref の世代チェックの回数
    pub ref_checks: usize,
    /// 静的解析のおかげで飛ばせた ref の世代チェックの回数
    pub ref_checks_elided: usize,
}

/// どのプログラムにも最初からある定義。`Some(x)` / `None` / `Ok(x)` / `Err(e)` は
/// `Option.Some(x)` などの短い書き方
const PRELUDE: &str = "enum Option<T> {\n    Some(T value);\n    None;\n}\nenum Result<T, E> {\n    Ok(T value);\n    Err(E error);\n}\n";

/// 字句解析と構文解析をして、最初からある定義（`Option`）を前に足す
fn parse(src: &str) -> Result<ast::Program, Diagnostic> {
    let mut prog = parser::Parser::new(lexer::lex(src)?).parse_program()?;
    let prelude = parser::Parser::new(lexer::lex(PRELUDE).expect("prelude lexes"))
        .parse_program()
        .expect("prelude parses");
    let mut enums = prelude.enums;
    enums.append(&mut prog.enums);
    prog.enums = enums;
    Ok(prog)
}

/// 実行せずに型検査だけして、警告を返す（エラーがあれば空。エラーは実行時に返る）
pub fn warnings(src: &str) -> Vec<Diagnostic> {
    let Ok(prog) = parse(src) else { return Vec::new() };
    types::check(&prog).unwrap_or_default()
}

/// ソースを実行する。静的解析で証明できた箇所の検査は消す。
pub fn run_source(src: &str, echo: bool) -> Outcome {
    run_source_with(src, echo, true)
}

/// `elide` が false なら静的解析をせず、すべての ref を実行時に検査する（比較用）。
pub fn run_source_with(src: &str, echo: bool, elide: bool) -> Outcome {
    run_program(src, echo, elide, Box::new(std::io::empty()), Vec::new())
}

/// 入力（`read_line()` が読む）を文字列で渡して実行する（テスト用）
pub fn run_source_with_input(src: &str, input: &str, args: &[&str]) -> Outcome {
    let input = std::io::Cursor::new(input.as_bytes().to_vec());
    run_program(src, false, true, Box::new(input), args.iter().map(|a| a.to_string()).collect())
}

/// `input` は `read_line()` が読む入力、`args` は `args()` が返す引数。
/// 深い再帰でも落ちないよう、大きなスタックのスレッドで動かす。
pub fn run_program(
    src: &str,
    echo: bool,
    elide: bool,
    input: Box<dyn std::io::BufRead + Send>,
    args: Vec<String>,
) -> Outcome {
    std::thread::scope(|s| {
        std::thread::Builder::new()
            .stack_size(256 << 20)
            .spawn_scoped(s, || run_inner(src, echo, elide, input, args))
            .expect("failed to spawn interpreter thread")
            .join()
            .expect("interpreter thread panicked")
    })
}

fn run_inner(src: &str, echo: bool, elide: bool, input: Box<dyn std::io::BufRead + Send>, args: Vec<String>) -> Outcome {
    let fail = |d: Diagnostic| Outcome {
        output: String::new(),
        error: Some(d),
        warnings: Vec::new(),
        live_objects: 0,
        analysis: analyze::Stats::default(),
        ref_checks: 0,
        ref_checks_elided: 0,
    };

    let prog = match parse(src) {
        Ok(p) => p,
        Err(d) => return fail(d),
    };
    let warnings = match types::check(&prog) {
        Ok(w) => w,
        Err(d) => return fail(d),
    };
    let analysis = if elide { analyze::analyze(&prog) } else { analyze::Stats::default() };
    let mut it = interp::Interp::new(echo);
    it.set_io(input, args);
    let res = it.run(prog);
    Outcome {
        output: it.output.clone(),
        error: res.err(),
        warnings,
        live_objects: it.live_objects(),
        analysis,
        ref_checks: it.ref_checks,
        ref_checks_elided: it.ref_checks_elided,
    }
}
