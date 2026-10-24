//! jwc（jwc/ にある、jabberwock で書いた jabberwock のコンパイラ）を Rust 版と突き合わせる。
//! jwc は clang で実行ファイルにして動かす（clang がなければ何もしない）。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use jabberwock::{llvm_build, native};

fn have_clang() -> bool {
    Command::new(native::clang()).arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// jwc の実行ファイル（テストの中で 1 回だけ作る）
fn jwc() -> PathBuf {
    static EXE: Mutex<Option<PathBuf>> = Mutex::new(None);
    let mut g = EXE.lock().unwrap_or_else(|e| e.into_inner());
    g.get_or_insert_with(|| {
        let dir = root().join("jwc");
        let src = std::fs::read_to_string(dir.join("main.jw")).unwrap();
        let read = move |name: &str| std::fs::read_to_string(dir.join(format!("{}.jw", name))).ok();
        let ll = llvm_build(&src, &read, "jwc/main.jw", false).expect("jwc を LLVM に書き出せる");
        let out = std::env::temp_dir().join(format!("jw-jwc-test-{}", std::process::id()));
        native::compile(&ll, &out, "jwc", "-O1").expect("jwc を clang でコンパイルできる")
    })
    .clone()
}

fn run_jwc(args: &[&str]) -> String {
    let out = Command::new(jwc()).args(args).output().expect("jwc を起動できる");
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// `--tokens` の出力が Rust 版（`token_dump`）と同じか
fn same_tokens(path: &Path) {
    let src = std::fs::read_to_string(path).unwrap();
    let name = path.to_str().unwrap();
    let want = jabberwock::token_dump(&src, name) + "\n";
    assert_eq!(run_jwc(&["--tokens", name]), want, "{}", name);
}

fn jw_files(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension() == Some(std::ffi::OsStr::new("jw")))
        .collect();
    v.sort();
    v
}

#[test]
fn tokens_of_the_repository_files() {
    if !have_clang() {
        return;
    }
    for dir in ["examples", "jwc"] {
        for p in jw_files(&root().join(dir)) {
            same_tokens(&p);
        }
    }
}

#[test]
fn tokens_and_lexer_errors() {
    if !have_clang() {
        return;
    }
    let cases = [
        "let x = 0xFF_ff + 0b1010 + 0o17;",
        "let y = 0xZZ;",
        "let z = 0x;",
        "let big = 18446744073709551615;",
        "let bigger = 18446744073709551616;",
        "let h = 0x1_0000_0000_0000_0000;",
        "let f = 1.5 + 1_0.25 + 3.x + 1_;",
        "let s = \"a\\tb\\n\\r\\0\\\"\\\\ \\u{1F600} \\u{41}\";",
        "let s = \"bad \\q\";",
        "let s = \"bad \\u{110000}\";",
        "let s = \"bad \\u{D800}\";",
        "let s = \"bad \\u{}\";",
        "let s = \"bad \\u{41\";",
        "let s = \"open\nnext",
        "let s = \"x = {x}, {a + b} {{lit}} {f(1, 2)}\";",
        "let s = \"{x\";",
        "let s = \"{ }\";",
        "let s = \"a } b\";",
        "let s = \"{x \"y\"}\";",
        "let s = \"\"\"\nmulti\nline {x}\n\"\"\" ;",
        "let s = \"\"\"never closed\n",
        "let 名前 = 3; // コメント\tタブ",
        "\tlet a = b @ c;",
        "let a = b << 2 >> 1 && c || d -> e += f -= g *= h /= i %= j ++ -- != == <= >= & | ^ ~ ? !;",
        "let s = \"日本語{x}語\";",
        "",
    ];
    let dir = std::env::temp_dir().join(format!("jw-jwc-cases-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (i, c) in cases.iter().enumerate() {
        let p = dir.join(format!("case{}.jw", i));
        std::fs::write(&p, c).unwrap();
        same_tokens(&p);
    }
}
