//! ward: `own` と世代付き `ref` だけで安全を目指す小さな言語の核。

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
}

/// ソースを実行する。深い再帰でも落ちないよう、大きなスタックのスレッドで動かす。
pub fn run_source(src: &str, echo: bool) -> Outcome {
    std::thread::scope(|s| {
        std::thread::Builder::new()
            .stack_size(256 << 20)
            .spawn_scoped(s, || run_inner(src, echo))
            .expect("failed to spawn interpreter thread")
            .join()
            .expect("interpreter thread panicked")
    })
}

fn run_inner(src: &str, echo: bool) -> Outcome {
    let fail = |d: Diagnostic| Outcome { output: String::new(), error: Some(d), live_objects: 0 };

    let toks = match lexer::lex(src) {
        Ok(t) => t,
        Err(d) => return fail(d),
    };
    let prog = match parser::Parser::new(toks).parse_program() {
        Ok(p) => p,
        Err(d) => return fail(d),
    };
    let mut it = interp::Interp::new(echo);
    let res = it.run(prog);
    Outcome {
        output: it.output.clone(),
        error: res.err(),
        live_objects: it.live_objects(),
    }
}
