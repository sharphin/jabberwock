use std::{env, fs, process};

fn main() {
    let mut stats = false;
    let mut elide = true;
    let mut path = None;
    for a in env::args().skip(1) {
        match a.as_str() {
            "-h" | "--help" => {
                eprintln!("使い方: ward [--stats] [--no-elide] <file.ward>");
                eprintln!("  --stats     ref の検査を静的解析でどれだけ消せたか表示する");
                eprintln!("  --no-elide  静的解析をせず、すべての ref を実行時に検査する");
                process::exit(0);
            }
            "--stats" => stats = true,
            "--no-elide" => elide = false,
            _ if path.is_none() => path = Some(a),
            _ => {
                eprintln!("引数が多すぎます: {}", a);
                process::exit(2);
            }
        }
    }
    let Some(path) = path else {
        eprintln!("使い方: ward [--stats] [--no-elide] <file.ward>");
        process::exit(2);
    };

    let src = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ファイルを読めません: {}: {}", path, e);
            process::exit(1);
        }
    };

    let outcome = ward::run_source_with(&src, true, elide);
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
