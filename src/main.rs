use std::{env, fs, process};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args[0] == "-h" || args[0] == "--help" {
        eprintln!("使い方: ward <file.ward>");
        process::exit(if args.is_empty() { 2 } else { 0 });
    }

    let path = &args[0];
    let src = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ファイルを読めません: {}: {}", path, e);
            process::exit(1);
        }
    };

    let outcome = ward::run_source(&src, true);
    if let Some(d) = outcome.error {
        eprint!("{}", d.render(path, &src));
        process::exit(1);
    }
}
