//! ward: `own` と世代付き `ref` だけで安全を目指す小さな言語の核。

pub mod analyze;
pub mod ast;
pub mod diag;
pub mod interp;
pub mod ir;
pub mod lexer;
pub mod modules;
pub mod native;
pub mod parser;
pub mod threads;
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
    /// 読んだファイル（モジュール名、ソース）。診断の位置の `Span.file` がこの番号。0 番は最初のファイル
    pub files: Vec<(String, String)>,
}

/// モジュール名からソースを返す（見つからなければ None）
pub type Reader<'a> = &'a (dyn Fn(&str) -> Option<String> + Sync);

/// モジュールを読まない（`use` はエラーになる）
pub fn no_modules(_: &str) -> Option<String> {
    None
}

/// 診断を、読んだファイルの中の位置で出す。`main` は最初のファイルの名前、モジュールは `名前.ward`
pub fn render(d: &Diagnostic, main: &str, files: &[(String, String)]) -> String {
    let names: Vec<String> =
        files.iter().enumerate().map(|(i, (m, _))| if i == 0 { main.to_string() } else { format!("{}.ward", m) }).collect();
    let fs: Vec<(&str, &str)> = names.iter().zip(files).map(|(n, (_, s))| (n.as_str(), s.as_str())).collect();
    if fs.is_empty() {
        return d.render(main, "");
    }
    d.render_in(&fs)
}

/// どのプログラムにも最初からある定義。`Some(x)` / `None` / `Ok(x)` / `Err(e)` は
/// `Option.Some(x)` などの短い書き方
const PRELUDE: &str = "enum Option<T> {\n    Some(T value);\n    None;\n\
    fn bool is_some() { match this { case Some(_) -> true; case None -> false; } }\n\
    fn bool is_none() { match this { case Some(_) -> false; case None -> true; } }\n}\n\
enum Result<T, E> {\n    Ok(T value);\n    Err(E error);\n\
    fn bool is_ok() { match this { case Ok(_) -> true; case Err(_) -> false; } }\n\
    fn bool is_err() { match this { case Ok(_) -> false; case Err(_) -> true; } }\n}\n\
fn Result<T, E> ok_or<T, E>(Option<T> o, E e) {\n    match o {\n        case Some(v) -> Ok(v);\n        case None -> Err(e);\n    }\n}\n";

/// 字句解析と構文解析をして（`use` したモジュールも読む）、最初からある定義（`Option`）を前に足す。
/// 読んだファイルも返す（エラーの表示に使う）
fn parse(src: &str, read: Reader) -> (Result<ast::Program, Diagnostic>, Vec<(String, String)>) {
    let mut loader = modules::Loader::new(read);
    let r = loader.load(src).map(add_prelude);
    (r, loader.files)
}

fn add_prelude(mut prog: ast::Program) -> ast::Program {
    let prelude = parser::Parser::new(lexer::lex(PRELUDE).expect("prelude lexes"))
        .parse_program()
        .expect("prelude parses");
    let mut enums = prelude.enums;
    enums.append(&mut prog.enums);
    prog.enums = enums;
    // Option を Result に変える `ok_or(o, e)`（`?` で使う）
    let mut fns = prelude.fns;
    fns.append(&mut prog.fns);
    prog.fns = fns;
    prog
}

/// 構文解析と型検査をして、プログラムと型の記録を返す（IR への書き出しや、そのテスト用）。
/// `TypeInfo` は構文木の中の式の位置で引くので、返したプログラムは書き換えないこと
pub fn typed(src: &str, read: Reader) -> Result<(ast::Program, types::TypeInfo), Diagnostic> {
    let prog = parse(src, read).0?;
    let (_, info) = types::check_with_info(&prog)?;
    Ok((prog, info))
}

/// IR（中間表現）を読みやすい文字にする（`ward --ir`）。エラーや、まだ書き出せない書き方があれば Err
pub fn ir_text(src: &str, read: Reader) -> Result<String, String> {
    let (prog, info) = typed(src, read).map_err(|d| d.title)?;
    ir::lower::lower(&prog, &info)
        .map(|mut p| {
            ir::analyze::analyze(&mut p);
            p.to_string()
        })
        .map_err(|r| format!("まだ IR に書き出せない書き方があります: {}", r))
}

/// LLVM のテキスト（`ward --ll`）。`main` は最初のファイルの名前（エラーの位置に出す）
pub fn llvm_text(src: &str, read: Reader, main: &str) -> Result<String, String> {
    llvm_build(src, read, main, false)
}

/// `ward build` 用の LLVM IR。`release` なら実行時エラーは使った位置だけ出す（ソースを入れない）。
/// 書けないときは、表示する文（ソースの行つきのエラー）を返す
pub fn llvm_build(src: &str, read: Reader, main: &str, release: bool) -> Result<String, String> {
    let (prog, files) = parse(src, read);
    let names: Vec<String> =
        files.iter().enumerate().map(|(i, (m, _))| if i == 0 { main.to_string() } else { format!("{}.ward", m) }).collect();
    let prog = prog.map_err(|d| render(&d, main, &files))?;
    let (_, info) = types::check_with_info(&prog).map_err(|d| render(&d, main, &files))?;
    let mut irp = ir::lower::lower(&prog, &info).map_err(|r| format!("まだ IR に書き出せない書き方があります: {}\n", r))?;
    ir::analyze::analyze(&mut irp);
    let sources: Vec<String> = if release { Vec::new() } else { files.iter().map(|(_, t)| t.clone()).collect() };
    ir::llvm::emit(&irp, &names, &sources).map_err(|r| format!("まだ LLVM に書き出せない書き方があります: {}\n", r))
}

/// 実行せずに型検査だけして、警告と読んだファイルを返す（エラーがあれば警告は空。エラーは実行時に返る）
pub fn warnings(src: &str, read: Reader) -> (Vec<Diagnostic>, Vec<(String, String)>) {
    let (prog, files) = parse(src, read);
    let Ok(prog) = prog else { return (Vec::new(), files) };
    (types::check(&prog).unwrap_or_default(), files)
}

/// ソースを実行する。静的解析で証明できた箇所の検査は消す。
pub fn run_source(src: &str, echo: bool) -> Outcome {
    run_source_with(src, echo, true)
}

/// `elide` が false なら静的解析をせず、すべての ref を実行時に検査する（比較用）。
pub fn run_source_with(src: &str, echo: bool, elide: bool) -> Outcome {
    run_program(src, &no_modules, echo, elide, Box::new(std::io::empty()), Vec::new())
}

/// モジュール（名前とソース）を渡して実行する（テスト用）
pub fn run_with_modules(src: &str, mods: &[(&str, &str)]) -> Outcome {
    let mods: Vec<(String, String)> = mods.iter().map(|(n, s)| (n.to_string(), s.to_string())).collect();
    let read = move |name: &str| mods.iter().find(|(n, _)| n == name).map(|(_, s)| s.clone());
    run_program(src, &read, false, true, Box::new(std::io::empty()), Vec::new())
}

/// 入力（`read_line()` が読む）を文字列で渡して実行する（テスト用）
pub fn run_source_with_input(src: &str, input: &str, args: &[&str]) -> Outcome {
    let bytes = input.as_bytes().to_vec();
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let input = Box::new(std::io::Cursor::new(bytes.clone()));
    in_big_stack(|| run_inner(src, &no_modules, false, true, input, args, Some(bytes)))
}

/// `input` は `read_line()` が読む入力、`args` は `args()` が返す引数。
/// 深い再帰でも落ちないよう、大きなスタックのスレッドで動かす。
pub fn run_program(
    src: &str,
    read: Reader,
    echo: bool,
    elide: bool,
    input: Box<dyn std::io::BufRead + Send>,
    args: Vec<String>,
) -> Outcome {
    in_big_stack(|| run_inner(src, read, echo, elide, input, args, None))
}

/// 新しいスレッドのために、読んだファイルからプログラムを作り直す（型検査と解析も同じにする）
fn reloader(files: &[(String, String)], elide: bool) -> Box<dyn Fn() -> ast::Program + Send + Sync> {
    let files = files.to_vec();
    Box::new(move || {
        let read = |name: &str| files.iter().skip(1).find(|(n, _)| n == name).map(|(_, s)| s.clone());
        let (prog, _) = parse(&files[0].1, &read);
        let prog = prog.expect("parsed before");
        types::check_with_info(&prog).expect("checked before");
        if elide {
            analyze::analyze(&prog);
        }
        prog
    })
}

/// IR で動かすときの、新しいスレッドのための IR の作り直し
fn ir_reloader(files: &[(String, String)]) -> Box<dyn Fn() -> ir::Program + Send + Sync> {
    let load = reloader(files, false);
    Box::new(move || {
        let prog = load();
        let (_, info) = types::check_with_info(&prog).expect("checked before");
        let mut irp = ir::lower::lower(&prog, &info).expect("lowered before");
        ir::analyze::analyze(&mut irp);
        irp
    })
}

fn in_big_stack(f: impl FnOnce() -> Outcome + Send) -> Outcome {
    std::thread::scope(|s| {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn_scoped(s, f)
            .expect("failed to spawn interpreter thread")
            .join()
            .expect("interpreter thread panicked")
    })
}

fn run_inner(
    src: &str,
    read: Reader,
    echo: bool,
    elide: bool,
    input: Box<dyn std::io::BufRead + Send>,
    args: Vec<String>,
    replay: Option<Vec<u8>>,
) -> Outcome {
    let (prog, files) = parse(src, read);
    let fail = |d: Diagnostic| Outcome {
        output: String::new(),
        error: Some(d),
        warnings: Vec::new(),
        live_objects: 0,
        analysis: analyze::Stats::default(),
        ref_checks: 0,
        ref_checks_elided: 0,
        files: files.clone(),
    };

    let prog = match prog {
        Ok(p) => p,
        Err(d) => return fail(d),
    };
    let (warnings, info) = match types::check_with_info(&prog) {
        Ok(w) => w,
        Err(d) => return fail(d),
    };
    let analysis = if elide { analyze::analyze(&prog) } else { analyze::Stats::default() };
    // デバッグビルドでは、書き出せるプログラムは IR でも実行して結果を突き合わせる（IR の書き出しの検査）
    let lowered = if cfg!(debug_assertions) { ir::lower::lower(&prog, &info).ok() } else { None };
    let lowered = lowered.map(|mut p| {
        ir::analyze::analyze(&mut p);
        p
    });
    drop(info);
    let mut it = interp::Interp::new(echo);
    it.set_world(threads::World::new(echo, reloader(&files, elide)));
    let io = (replay, args.clone());
    it.set_io(input, args);
    let res = it.run(prog);
    // 入力を使うプログラムは、同じ入力をもう一度渡せるときだけ突き合わせる
    let lowered = lowered.filter(|p| io.0.is_some() || !p.uses_input());
    if let Some(irp) = lowered {
        let input = io.0.clone().unwrap_or_default();
        if let Some(diff) = ir_mismatch(&irp, &files, &it.output(), res.as_ref().err(), it.live_objects(), &input, &io.1) {
            panic!("IR の実行結果がインタプリタと違います: {}\n--- IR ---\n{}", diff, irp);
        }
        // WARD_NATIVE があれば、clang で作った実行ファイルとも突き合わせる（clang が要る）
        if std::env::var_os("WARD_NATIVE").is_some() {
            if let Some(diff) = native_mismatch(&irp, &files, &it.output(), res.as_ref().err(), it.live_objects(), &input, &io.1) {
                panic!("実行ファイルの結果がインタプリタと違います: {}\n--- IR ---\n{}", diff, irp);
            }
        }
    }
    Outcome {
        output: it.output(),
        error: res.err(),
        warnings,
        live_objects: it.live_objects(),
        analysis,
        ref_checks: it.ref_checks,
        ref_checks_elided: it.ref_checks_elided,
        files,
    }
}

/// IR を実行して、インタプリタの結果（出力・エラーの題と位置・残ったヒープの数）と違えばその説明を返す
fn ir_mismatch(
    irp: &ir::Program,
    files: &[(String, String)],
    output: &str,
    error: Option<&Diagnostic>,
    live: usize,
    input: &[u8],
    args: &[String],
) -> Option<String> {
    let mut it = interp::Interp::new(false);
    it.set_world(threads::World::new_ir(ir_reloader(files)));
    it.set_io(Box::new(std::io::Cursor::new(input.to_vec())), args.to_vec());
    let res = ir::exec::run(irp, &mut it);
    if it.output() != output {
        return Some(format!("出力が違う\ninterp:\n{}\nIR:\n{}", output, it.output()));
    }
    let key = |d: &Diagnostic| (d.title.clone(), d.labels.iter().find(|l| l.primary).map(|l| l.span));
    match (error, res.as_ref().err()) {
        (None, None) if live != it.live_objects() => {
            Some(format!("残ったヒープの数が違う（interp {}・IR {}）", live, it.live_objects()))
        }
        (None, None) => None,
        (Some(a), Some(b)) if key(a) == key(b) => None,
        (a, b) => Some(format!("エラーが違う\ninterp: {:?}\nIR: {:?}", a.map(key), b.map(key))),
    }
}

/// LLVM に書き出して clang で作った実行ファイルを走らせ、インタプリタの結果と違えばその説明を返す
fn native_mismatch(
    irp: &ir::Program,
    files: &[(String, String)],
    output: &str,
    error: Option<&Diagnostic>,
    live: usize,
    input: &[u8],
    args: &[String],
) -> Option<String> {
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let trace = std::env::var_os("WARD_NATIVE_TRACE").is_some();
    let names: Vec<String> =
        files.iter().enumerate().map(|(i, (m, _))| if i == 0 { "main.ward".to_string() } else { format!("{}.ward", m) }).collect();
    let sources: Vec<String> = files.iter().map(|(_, t)| t.clone()).collect();
    let ll = match ir::llvm::emit(irp, &names, &sources) {
        Ok(t) => t,
        Err(why) => {
            if trace {
                eprintln!("\nNATIVE-SKIP {}", why);
            }
            return None;
        }
    };
    let dir = std::env::temp_dir().join(format!("ward-native-{}", std::process::id()));
    let name = format!("p{}", COUNTER.fetch_add(1, Ordering::SeqCst));
    let exe = match native::compile_with_cached_runtime(&ll, &dir, &name) {
        Ok(e) => e,
        Err(e) => return Some(format!("{}\n--- ll ---\n{}", e, ll)),
    };
    let child = std::process::Command::new(&exe)
        .args(args)
        .env("WARD_LEAKS", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();
    let run = child.and_then(|mut c| {
        if let Some(mut si) = c.stdin.take() {
            let _ = si.write_all(input);
        }
        c.wait_with_output()
    });
    let out = match run {
        Ok(o) => o,
        Err(e) => return Some(format!("実行できません: {}", e)),
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let diff = if stdout != output {
        Some(format!("出力が違う\ninterp:\n{}\n実行ファイル:\n{}\n{}", output, stdout, stderr))
    } else {
        match error {
            None => {
                let got = stderr.lines().find_map(|l| l.strip_prefix("[live] ")).and_then(|n| n.trim().parse::<usize>().ok());
                if !out.status.success() {
                    Some(format!("実行ファイルが失敗した（{:?}）\n{}", out.status.code(), stderr))
                } else if got != Some(live) {
                    Some(format!("残ったヒープの数が違う（interp {}・実行ファイル {:?}）", live, got))
                } else {
                    None
                }
            }
            Some(d) => {
                let shown: Vec<(&str, &str)> = names.iter().zip(files).map(|(n, (_, t))| (n.as_str(), t.as_str())).collect();
                let want = d.render_in(&shown);
                if out.status.code() != Some(1) || stderr != want {
                    Some(format!("エラーが違う\ninterp:\n{}\n実行ファイル（{:?}）:\n{}", want, out.status.code(), stderr))
                } else {
                    None
                }
            }
        }
    };
    if diff.is_none() {
        let _ = std::fs::remove_file(&exe);
        let _ = std::fs::remove_file(dir.join(format!("{}.ll", name)));
        if trace {
            eprintln!("\nNATIVE-OK");
        }
    }
    diff.map(|d| format!("{}\n--- ll: {} ---", d, dir.join(format!("{}.ll", name)).display()))
}
