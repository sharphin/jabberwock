use std::{env, fs, process};

fn main() {
    let mut stats = false;
    let mut elide = true;
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
                eprintln!("使い方: ward [--stats] [--no-elide] <file.ward> [プログラムへの引数...]");
                eprintln!("  --stats     ref の検査を静的解析でどれだけ消せたか表示する");
                eprintln!("  --no-elide  静的解析をせず、すべての ref を実行時に検査する");
                process::exit(0);
            }
            "--stats" => stats = true,
            "--no-elide" => elide = false,
            _ => path = Some(a),
        }
    }
    let Some(path) = path else {
        eprintln!("使い方: ward [--stats] [--no-elide] <file.ward> [プログラムへの引数...]");
        process::exit(2);
    };

    let src = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ファイルを読めません: {}: {}", path, e);
            process::exit(1);
        }
    };

    // 警告は実行の前に出す（Rust と同じ）
    for w in ward::warnings(&src) {
        eprint!("{}", w.render(&path, &src));
    }
    let input = Box::new(std::io::BufReader::new(std::io::stdin()));
    let outcome = ward::run_program(&src, true, elide, input, prog_args);
    if stats {
        let a = outcome.analysis;
        eprintln!("[stats] `*p` と `p.f` の箇所: 検査を消せた {} / 実行時に残る {}", a.elided, a.required);
        eprintln!(
            "[stats] ref の世代チェック: 実行 {} 回 / 省略 {} 回",
            outcome.ref_checks, outcome.ref_checks_elided
        );
    }
    if let Some(d) = outcome.error {
        eprint!("{}", d.render(&path, &src));
        process::exit(1);
    }
}
