use std::{env, fs, process};

fn main() {
    match env::args().nth(1).as_deref() {
        Some("build") => {
            let rest: Vec<String> = env::args().skip(2).collect();
            // ファイルを書かなければパッケージ（jabberwock.toml）を作る
            if rest.iter().any(|a| a.ends_with(".jw") || a == "-h" || a == "--help") {
                build(rest);
            } else {
                project_build(rest);
            }
            return;
        }
        Some("new") => return project_new(env::args().skip(2).collect()),
        Some("run") => return project_run(env::args().skip(2).collect()),
        Some("test") => return project_test(env::args().skip(2).collect()),
        _ => {}
    }
    let mut stats = false;
    let mut elide = true;
    let mut show_ir = false;
    let mut show_ll = false;
    let mut show_tokens = false;
    let mut show_ast = false;
    let mut show_typed = false;
    let mut show_ir_raw = false;
    let mut path = None;
    let mut prog_args = Vec::new();
    for a in env::args().skip(1) {
        if path.is_some() {
            // ファイル名のあとの引数は、プログラムの `args()` に渡す
            prog_args.push(a);
            continue;
        }
        match a.as_str() {
            "-h" | "--help" => {
                eprintln!("使い方: jabberwock [--stats] [--no-elide] [--ir] [--ll] <file.jw> [プログラムへの引数...]");
                eprintln!("        jabberwock build [--release] [-o 出力] <file.jw>   実行ファイルを作る（clang が要る）");
                eprintln!("パッケージ（jabberwock.toml のあるフォルダ。docs/packages.md）:");
                eprintln!("        jabberwock new <名前> [--lib]       新しいパッケージを作る");
                eprintln!("        jabberwock build [--release]       target/debug（release）に実行ファイルを作る");
                eprintln!("        jabberwock run [--native] [-- 引数...]   src/main.jw を動かす（--native は実行ファイルを作って動かす）");
                eprintln!("        jabberwock test                    tests/*.jw を 1 本ずつ動かす（tests/名前.out があれば出力と比べる）");
                eprintln!("  --stats     ref の検査を静的解析でどれだけ消せたか表示する");
                eprintln!("  --no-elide  静的解析をせず、すべての ref を実行時に検査する");
                eprintln!("  --ir        実行せず、IR（中間表現）を表示する");
                eprintln!("  --ll        実行せず、LLVM IR を表示する");
                process::exit(0);
            }
            "--stats" => stats = true,
            "--no-elide" => elide = false,
            "--ir" => show_ir = true,
            "--ll" => show_ll = true,
            "--tokens" => show_tokens = true,
            "--ast" => show_ast = true,
            "--typed" => show_typed = true,
            "--ir-raw" => show_ir_raw = true,
            _ => path = Some(a),
        }
    }
    let Some(path) = path else {
        eprintln!("使い方: jabberwock [--stats] [--no-elide] [--ir] [--ll] <file.jw> [プログラムへの引数...]");
        process::exit(2);
    };

    let src = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ファイルを読めません: {}: {}", path, e);
            process::exit(1);
        }
    };

    // `use geom;` は、このファイルと同じフォルダの geom.jw を読む
    let dir = std::path::Path::new(&path).parent().map(|d| d.to_path_buf()).unwrap_or_default();
    let read = move |name: &str| fs::read_to_string(dir.join(format!("{}.jw", name))).ok();

    if show_tokens {
        println!("{}", jabberwock::token_dump(&src, &path));
        return;
    }

    if show_ast {
        println!("{}", jabberwock::ast_dump(&src, &read, &path));
        return;
    }

    if show_ir_raw {
        print!("{}", jabberwock::ir_dump(&src, &read, &path, false));
        return;
    }

    if show_typed {
        println!("{}", jabberwock::typed_dump(&src, &read, &path));
        return;
    }

    if show_ll {
        match jabberwock::llvm_text(&src, &read, &path) {
            Ok(t) => print!("{}", t),
            Err(e) => {
                eprintln!("{}", e);
                process::exit(1);
            }
        }
        return;
    }

    if show_ir {
        match jabberwock::ir_text(&src, &read) {
            Ok(t) => print!("{}", t),
            Err(e) => {
                eprintln!("{}", e);
                process::exit(1);
            }
        }
        return;
    }

    // 警告は実行の前に出す（Rust と同じ）
    let (warnings, files) = jabberwock::warnings(&src, &read);
    for w in warnings {
        eprint!("{}", jabberwock::render(&w, &path, &files));
    }
    let input = Box::new(std::io::BufReader::new(std::io::stdin()));
    let outcome = jabberwock::run_program(&src, &read, true, elide, input, prog_args);
    if stats {
        let a = outcome.analysis;
        eprintln!("[stats] `*p` と `p.f` の箇所: 検査を消せた {} / 実行時に残る {}", a.elided, a.required);
        eprintln!(
            "[stats] ref の世代チェック: 実行 {} 回 / 省略 {} 回",
            outcome.ref_checks, outcome.ref_checks_elided
        );
    }
    if let Some(d) = &outcome.error {
        eprint!("{}", jabberwock::render(d, &path, &outcome.files));
        process::exit(1);
    }
    if let Some(code) = outcome.exit_code {
        process::exit(code);
    }
}

/// `jabberwock build [--release] [-o 出力] <file.jw>`: LLVM に書き出し、clang で実行ファイルを作る
fn build(args: Vec<String>) {
    let usage = "使い方: jabberwock build [--release] [-o 出力] [測定用の切り替え] <file.jw>";
    let mut release = false;
    // 測定用（ベンチマークで検査の費用を分ける）。--no-gen-checks / --no-bounds-checks / --unchecked は安全でない
    let mut elide = true;
    let mut stats = false;
    let mut opts = jabberwock::ir::llvm::Opts::default();
    let mut out = None;
    let mut path = None;
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--release" => release = true,
            "--no-elide" => elide = false,
            "--no-gen-checks" => opts.gen = false,
            "--no-bounds-checks" => opts.bounds = false,
            "--unchecked" => {
                opts.gen = false;
                opts.bounds = false;
            }
            "--count-checks" => opts.count = true,
            "--stats" => stats = true,
            "-o" => match it.next() {
                Some(o) => out = Some(o),
                None => {
                    eprintln!("{}", usage);
                    process::exit(2);
                }
            },
            "-h" | "--help" => {
                eprintln!("{}", usage);
                eprintln!("  --release  実行時エラーは使った位置だけ出す（ソースを実行ファイルに入れない）");
                eprintln!("  -o 出力     実行ファイルの名前（なければファイル名から .jw を除いたもの）");
                eprintln!("測定用（ベンチマークで検査の費用を分けるため）:");
                eprintln!("  --no-elide          検査を消す解析をしない（世代の検査を全部残す）");
                eprintln!("  --no-gen-checks     ref の世代を確かめない（安全でない。上限値を測るだけ）");
                eprintln!("  --no-bounds-checks  添字の範囲を確かめない（安全でない）");
                eprintln!("  --unchecked         上の 2 つの両方");
                eprintln!("  --count-checks      実行時に通った検査の数を、終わるときに標準エラーへ出す（Linux）");
                eprintln!("  --stats             書き出した検査の場所の数を出す");
                process::exit(0);
            }
            _ if path.is_none() => path = Some(a),
            _ => {
                eprintln!("{}", usage);
                process::exit(2);
            }
        }
    }
    let Some(path) = path else {
        eprintln!("{}", usage);
        process::exit(2);
    };
    let src = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ファイルを読めません: {}: {}", path, e);
            process::exit(1);
        }
    };
    let dir = std::path::Path::new(&path).parent().map(|d| d.to_path_buf()).unwrap_or_default();
    let read = move |name: &str| fs::read_to_string(dir.join(format!("{}.jw", name))).ok();
    let (warnings, files) = jabberwock::warnings(&src, &read);
    for w in warnings {
        eprint!("{}", jabberwock::render(&w, &path, &files));
    }
    let ll = match jabberwock::llvm_build_with(&src, &read, &path, release, elide, opts) {
        Ok((t, st)) => {
            if stats {
                eprintln!("[stats] 世代の検査 {} か所・解析で消した {} か所・添字の検査 {} か所", st.gen, st.elided, st.bounds);
            }
            t
        }
        Err(e) => {
            eprint!("{}", e);
            process::exit(1);
        }
    };
    let out = out.unwrap_or_else(|| {
        let stem = std::path::Path::new(&path).file_stem().and_then(|s| s.to_str()).unwrap_or("a");
        stem.to_string()
    });
    match jabberwock::native::build(&ll, std::path::Path::new(&out), "-O2") {
        Ok(exe) => eprintln!("作りました: {}", exe.display()),
        Err(e) => {
            eprintln!("{}", e);
            if e.starts_with("clang を起動できません") {
                eprintln!("clang が要ります。入れ方は docs/native.md を見てください（別の clang を使うなら JW_CLANG）");
            }
            process::exit(1);
        }
    }
}

/// 今のフォルダから上へたどってパッケージを読む（なければ終わる）
fn current_project() -> jabberwock::project::Project {
    let cwd = env::current_dir().unwrap_or_default();
    let Some(root) = jabberwock::project::Project::find(&cwd) else {
        eprintln!("{} が見つかりません（このフォルダか上のフォルダに要ります。`jabberwock new 名前` で作れます）", jabberwock::project::MANIFEST);
        process::exit(1);
    };
    match jabberwock::project::Project::load(&root) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}

/// パッケージでのエラーの表示（モジュールのファイルはパッケージからの道筋で出す）
fn render_in(p: &jabberwock::project::Project, d: &jabberwock::diag::Diagnostic, main: &str, files: &[(String, String)]) -> String {
    jabberwock::render_named(d, main, files, &|m| match p.modules.get(m) {
        Some(f) => shown(p, f),
        None => format!("{}.jw", m),
    })
}

/// パッケージの中のファイルを、表示用にパッケージからの道筋で書く
fn shown(p: &jabberwock::project::Project, f: &std::path::Path) -> String {
    f.strip_prefix(&p.root).unwrap_or(f).display().to_string()
}

fn read_or_exit(f: &std::path::Path) -> String {
    match fs::read_to_string(f) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ファイルを読めません: {}: {}", f.display(), e);
            process::exit(1);
        }
    }
}

/// `jabberwock new 名前 [--lib]`
fn project_new(args: Vec<String>) {
    let usage = "使い方: jabberwock new <名前> [--lib]";
    let mut lib = false;
    let mut name = None;
    for a in args {
        match a.as_str() {
            "--lib" => lib = true,
            _ if name.is_none() && !a.starts_with('-') => name = Some(a),
            _ => {
                eprintln!("{}", usage);
                process::exit(2);
            }
        }
    }
    let Some(name) = name else {
        eprintln!("{}", usage);
        process::exit(2);
    };
    match jabberwock::project::create(std::path::Path::new(&name), lib) {
        Ok(()) => eprintln!("作りました: {}（{}）", name, if lib { "ライブラリ" } else { "実行ファイル" }),
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}

/// パッケージの src/main.jw から実行ファイルを作る（新しければ作り直さない）。作った場所を返す
fn project_exe(p: &jabberwock::project::Project, release: bool) -> std::path::PathBuf {
    let main = p.main_file();
    if !main.is_file() {
        eprintln!("{} がありません（ライブラリは build できません。tests は `jabberwock test` で動かせます）", shown(p, &main));
        process::exit(1);
    }
    let exe = p.exe_path(release);
    if !p.stale(&exe) {
        return exe;
    }
    let src = read_or_exit(&main);
    let read = p.reader();
    let name = shown(p, &main);
    let (warnings, files) = jabberwock::warnings(&src, &read);
    for w in warnings {
        eprint!("{}", render_in(p, &w, &name, &files));
    }
    let ll = match jabberwock::llvm_build(&src, &read, &name, release) {
        Ok(t) => t,
        Err(e) => {
            eprint!("{}", e);
            process::exit(1);
        }
    };
    if let Some(d) = exe.parent() {
        let _ = fs::create_dir_all(d);
    }
    match jabberwock::native::build(&ll, &exe, "-O2") {
        Ok(e) => {
            eprintln!("作りました: {}", shown(p, &e));
            e
        }
        Err(e) => {
            eprintln!("{}", e);
            if e.starts_with("clang を起動できません") {
                eprintln!("clang が要ります。入れ方は docs/native.md を見てください（別の clang を使うなら JW_CLANG）");
            }
            process::exit(1);
        }
    }
}

/// `jabberwock build [--release]`（パッケージ）
fn project_build(args: Vec<String>) {
    let mut release = false;
    for a in args {
        match a.as_str() {
            "--release" => release = true,
            _ => {
                eprintln!("使い方: jabberwock build [--release]（パッケージ） / jabberwock build [--release] [-o 出力] <file.jw>");
                process::exit(2);
            }
        }
    }
    let p = current_project();
    let exe = p.exe_path(release);
    let fresh = !p.stale(&exe);
    let exe = project_exe(&p, release);
    if fresh {
        eprintln!("最新です: {}", shown(&p, &exe));
    }
}

/// `jabberwock run [--native] [--release] [-- 引数...]`
fn project_run(args: Vec<String>) {
    let mut native = false;
    let mut release = false;
    let mut prog_args = Vec::new();
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--native" => native = true,
            "--release" => {
                native = true;
                release = true;
            }
            "--" => {
                prog_args.extend(it.by_ref());
            }
            _ => {
                eprintln!("使い方: jabberwock run [--native] [--release] [-- プログラムへの引数...]");
                process::exit(2);
            }
        }
    }
    let p = current_project();
    if native {
        let exe = project_exe(&p, release);
        let status = process::Command::new(&exe).args(&prog_args).status();
        match status {
            Ok(s) => process::exit(s.code().unwrap_or(1)),
            Err(e) => {
                eprintln!("{} を動かせません: {}", exe.display(), e);
                process::exit(1);
            }
        }
    }
    let main = p.main_file();
    if !main.is_file() {
        eprintln!("{} がありません（ライブラリは run できません。tests は `jabberwock test` で動かせます）", shown(&p, &main));
        process::exit(1);
    }
    let src = read_or_exit(&main);
    let read = p.reader();
    let name = shown(&p, &main);
    let (warnings, files) = jabberwock::warnings(&src, &read);
    for w in warnings {
        eprint!("{}", render_in(&p, &w, &name, &files));
    }
    let input = Box::new(std::io::BufReader::new(std::io::stdin()));
    let outcome = jabberwock::run_program(&src, &read, true, true, input, prog_args);
    if let Some(d) = &outcome.error {
        eprint!("{}", render_in(&p, d, &name, &outcome.files));
        process::exit(1);
    }
    if let Some(code) = outcome.exit_code {
        process::exit(code);
    }
}

/// `jabberwock test`: tests/*.jw を 1 本ずつ interp で動かす。エラーか 0 でない exit で失敗、tests/名前.out があれば出力も比べる
fn project_test(args: Vec<String>) {
    if !args.is_empty() {
        eprintln!("使い方: jabberwock test");
        process::exit(2);
    }
    let p = current_project();
    let tests = p.tests();
    if tests.is_empty() {
        eprintln!("テストがありません（tests/*.jw）");
        return;
    }
    let read = p.reader();
    let (mut ok, mut failed) = (0, Vec::new());
    for t in &tests {
        let name = shown(&p, t);
        let src = read_or_exit(t);
        let input = Box::new(std::io::Cursor::new(Vec::new()));
        let o = jabberwock::run_program(&src, &read, false, true, input, Vec::new());
        let want = fs::read_to_string(t.with_extension("out")).ok();
        let problem = if let Some(d) = &o.error {
            Some(render_in(&p, d, &name, &o.files))
        } else if o.exit_code.unwrap_or(0) != 0 {
            Some(format!("exit({}) で終わりました\n", o.exit_code.unwrap_or(0)))
        } else {
            match &want {
                Some(w) if *w != o.output => Some(format!("出力が違います\n--- 期待（{}）\n{}--- 実際\n{}", shown(&p, &t.with_extension("out")), w, o.output)),
                _ => None,
            }
        };
        match problem {
            None => {
                println!("test {} ... ok", name);
                ok += 1;
            }
            Some(why) => {
                println!("test {} ... 失敗", name);
                failed.push((name, why));
            }
        }
    }
    for (name, why) in &failed {
        println!("\n---- {} ----\n{}", name, why);
    }
    println!("\n結果: {} 本成功・{} 本失敗", ok, failed.len());
    if !failed.is_empty() {
        process::exit(1);
    }
}
