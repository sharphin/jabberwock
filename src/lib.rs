//! ward: `own` と世代付き `ref` だけで安全を目指す小さな言語の核。

pub mod analyze;
pub mod ast;
pub mod diag;
pub mod interp;
pub mod lexer;
pub mod parser;

use diag::Diagnostic;

/// 実行結果
pub struct Outcome {
    /// `print` が出した内容
    pub output: String,
    /// エラーがあれば診断
    pub error: Option<Diagnostic>,
    /// 実行後に解放されずに残ったヒープオブジェクト数（0 ならリークなし）
    pub live_objects: usize,
    /// 静的解析の結果（`*p` のうち検査を消せた箇所・残った箇所）
    pub analysis: analyze::Stats,
    /// 実行時に行った ref の世代チェックの回数
    pub ref_checks: usize,
    /// 静的解析のおかげで飛ばせた ref の世代チェックの回数
    pub ref_checks_elided: usize,
}

/// ソースを実行する。静的解析で証明できた箇所の検査は消す。
pub fn run_source(src: &str, echo: bool) -> Outcome {
    run_source_with(src, echo, true)
}

/// `elide` が false なら静的解析をせず、すべての ref を実行時に検査する（比較用）。
/// 深い再帰でも落ちないよう、大きなスタックのスレッドで動かす。
pub fn run_source_with(src: &str, echo: bool, elide: bool) -> Outcome {
    std::thread::scope(|s| {
        std::thread::Builder::new()
            .stack_size(256 << 20)
            .spawn_scoped(s, || run_inner(src, echo, elide))
            .expect("failed to spawn interpreter thread")
            .join()
            .expect("interpreter thread panicked")
    })
}

fn run_inner(src: &str, echo: bool, elide: bool) -> Outcome {
    let fail = |d: Diagnostic| Outcome {
        output: String::new(),
        error: Some(d),
        live_objects: 0,
        analysis: analyze::Stats::default(),
        ref_checks: 0,
        ref_checks_elided: 0,
    };

    let toks = match lexer::lex(src) {
        Ok(t) => t,
        Err(d) => return fail(d),
    };
    let prog = match parser::Parser::new(toks).parse_program() {
        Ok(p) => p,
        Err(d) => return fail(d),
    };
    let analysis = if elide { analyze::analyze(&prog) } else { analyze::Stats::default() };
    let mut it = interp::Interp::new(echo);
    let res = it.run(prog);
    Outcome {
        output: it.output.clone(),
        error: res.err(),
        live_objects: it.live_objects(),
        analysis,
        ref_checks: it.ref_checks,
        ref_checks_elided: it.ref_checks_elided,
    }
}
