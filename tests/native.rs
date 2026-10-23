//! LLVM に書き出して clang で作った実行ファイル。clang がなければ何もしない。
//! テスト全体を実行ファイルでも突き合わせるには `WARD_NATIVE=1 cargo test`（`lib.rs` の `native_mismatch`）

use std::process::Command;

use ward::{llvm_text, native, no_modules};

fn have_clang() -> bool {
    Command::new(native::clang()).arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

/// 実行ファイルを作って走らせる（終了コード, 標準出力, 標準エラー）
fn run(name: &str, src: &str) -> (Option<i32>, String, String) {
    let ll = llvm_text(src, &no_modules, "main.ward").expect("lowers to LLVM");
    let dir = std::env::temp_dir().join(format!("ward-native-test-{}", std::process::id()));
    let exe = native::compile(&ll, &dir, name, "-O1").expect("clang compiles");
    let out = Command::new(&exe).env("WARD_LEAKS", "1").output().expect("runs");
    (out.status.code(), String::from_utf8_lossy(&out.stdout).to_string(), String::from_utf8_lossy(&out.stderr).to_string())
}

#[test]
fn compiled_programs_print_the_same() {
    if !have_clang() {
        return;
    }
    let src = "struct P { i64 x; own i64 y; }\n\
               fn i64 fib(i64 n) { if n < 2 { return n; } fib(n - 1) + fib(n - 2) }\n\
               fn main() {\n    let p = P { x: 1, y: own 2 };\n    let r = ref p;\n    let xs = [1.5, 0.1, 100000000000000000000.0];\n    \
               print(fib(20), r.x, *r.y, xs, \"{p}\");\n}";
    let (code, out, err) = run("print", src);
    assert_eq!(code, Some(0), "{}", err);
    assert_eq!(out, "6765 1 2 [1.5, 0.1, 1e20] P { x: 1, y: 2 }\n");
    assert!(err.contains("[live] 0"), "{}", err);
}

#[test]
fn compiled_programs_catch_dangling_refs() {
    if !have_clang() {
        return;
    }
    let src = "fn main() {\n    let a = own 5;\n    let r = ref a;\n    drop(a);\n    print(*r);\n}";
    let (code, out, err) = run("dangling", src);
    assert_eq!(code, Some(1));
    assert_eq!(out, "");
    assert!(err.contains("エラー: 無効になった参照を使いました (dangling reference)"), "{}", err);
    assert!(err.contains("==> main.ward:5:11"), "{}", err);
}
