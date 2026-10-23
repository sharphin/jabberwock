use std::{env, fs, process};

fn main() {
    let mut stats = false;
    let mut elide = true;
    let mut show_ir = false;
    let mut show_ll = false;
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
                eprintln!("使い方: ward [--stats] [--no-elide] [--ir] [--ll] <file.ward> [プログラムへの引数...]");
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
            _ => path = Some(a),
        }
    }
    let Some(path) = path else {
        eprintln!("使い方: ward [--stats] [--no-elide] [--ir] [--ll] <file.ward> [プログラムへの引数...]");
        process::exit(2);
    };

    let src = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ファイルを読めません: {}: {}", path, e);
            process::exit(1);
        }
    };

    // `use geom;` は、このファイルと同じフォルダの geom.ward を読む
    let dir = std::path::Path::new(&path).parent().map(|d| d.to_path_buf()).unwrap_or_default();
    let read = move |name: &str| fs::read_to_string(dir.join(format!("{}.ward", name))).ok();

    if show_ll {
        match ward::llvm_text(&src, &read, &path) {
            Ok(t) => print!("{}", t),
            Err(e) => {
                eprintln!("{}", e);
                process::exit(1);
            }
        }
        return;
    }

    if show_ir {
        match ward::ir_text(&src, &read) {
            Ok(t) => print!("{}", t),
            Err(e) => {
                eprintln!("{}", e);
                process::exit(1);
            }
        }
        return;
    }

    // 警告は実行の前に出す（Rust と同じ）
    let (warnings, files) = ward::warnings(&src, &read);
    for w in warnings {
        eprint!("{}", ward::render(&w, &path, &files));
    }
    let input = Box::new(std::io::BufReader::new(std::io::stdin()));
    let outcome = ward::run_program(&src, &read, true, elide, input, prog_args);
    if stats {
        let a = outcome.analysis;
        eprintln!("[stats] `*p` と `p.f` の箇所: 検査を消せた {} / 実行時に残る {}", a.elided, a.required);
        eprintln!(
            "[stats] ref の世代チェック: 実行 {} 回 / 省略 {} 回",
            outcome.ref_checks, outcome.ref_checks_elided
        );
    }
    if let Some(d) = &outcome.error {
        eprint!("{}", ward::render(d, &path, &outcome.files));
        process::exit(1);
    }
}
