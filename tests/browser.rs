//! 軽量ブラウザの試作（browser/）のテスト。本体（browser/app の jabberwock）を偽エンジンで動かし、
//! 台本（testdata/*.txt）を流したときにエンジンへ出す操作が testdata/*.out と同じかを見る。
//! 出力を直すときは `JW_BLESS=1 cargo test --test browser` で .out を作り直す。
//! C の偽エンジン（browser/engine/mock）のテストは cc があるときだけ。

use std::path::{Path, PathBuf};
use std::process::Command;

fn app() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("browser/app")
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("jw-browser-test-{}-{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// browser/app で jabberwock を動かし、標準出力を返す（失敗なら止める）
fn run(args: &[&str]) -> String {
    let o = Command::new(env!("CARGO_BIN_EXE_jabberwock")).current_dir(app()).args(args).output().unwrap();
    let out = String::from_utf8_lossy(&o.stdout).into_owned();
    assert!(o.status.success(), "{:?} が失敗した:\n{}\n{}", args, out, String::from_utf8_lossy(&o.stderr));
    out
}

fn same_as_file(got: &str, name: &str) {
    let path = app().join("testdata").join(name);
    if std::env::var("JW_BLESS").is_ok() {
        std::fs::write(&path, got).unwrap();
        return;
    }
    let want = std::fs::read_to_string(&path).unwrap().replace("\r\n", "\n");
    assert!(got == want, "{} と違う:\n--- 出力 ---\n{}", name, got);
}

#[test]
fn parts() {
    for f in ["test_address.jw", "test_store.jw", "test_chrome.jw"] {
        assert_eq!(run(&[f]), "ok\n", "{}", f);
    }
}

#[test]
fn scripts() {
    for name in ["basic", "tabs"] {
        let script = format!("testdata/{}.txt", name);
        let out = run(&["main.jw", "--ephemeral", "--fake", &script, "--log"]);
        same_as_file(&out, &format!("{}.out", name));
    }
}

// デバッグビルドの jabberwock は IR でもう一度動かすので、ファイルへの書き込みが 2 回起きる。
// なので、何回動かしても同じになることだけを確かめる
#[test]
fn keeps_history_and_bookmarks_in_one_folder() {
    let dir = scratch("data");
    let d = dir.to_str().unwrap().replace('\\', "/");
    std::fs::write(dir.join("bookmarks.tsv"), "https://saved.example/\t保存した\n").unwrap();
    let out = run(&["main.jw", "--data", &d, "--fake", "testdata/tabs.txt", "--log"]);
    assert!(out.contains("\nbm\thttps://saved.example/\t保存した\n"), "前のブックマークが外枠に出る:\n{}", out);
    run(&["main.jw", "--data", &d, "--fake", "testdata/basic.txt"]);
    let history = std::fs::read_to_string(dir.join("history.tsv")).unwrap();
    assert!(history.contains("https://example.com\ttitle:https://example.com\n"), "{}", history);
    assert!(!history.contains("fail://x"), "読み込めなかったページは残さない:\n{}", history);
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2, "置くのは history.tsv と bookmarks.tsv だけ");
    // --ephemeral なら何も書かない
    let eph = scratch("ephemeral");
    let e = eph.to_str().unwrap().replace('\\', "/");
    run(&["main.jw", "--data", &e, "--ephemeral", "--fake", "testdata/basic.txt"]);
    assert_eq!(std::fs::read_dir(&eph).unwrap().count(), 0);
}

#[test]
fn c_mock_engine() {
    if Command::new("cc").arg("--version").output().is_err() {
        return;
    }
    let dir = scratch("cmock");
    let exe = dir.join("mock_test");
    let engine = Path::new(env!("CARGO_MANIFEST_DIR")).join("browser/engine");
    let built = Command::new("cc")
        .arg("-Wall")
        .arg("-o")
        .arg(&exe)
        .arg(engine.join("tests/mock_test.c"))
        .arg(engine.join("mock/jwe_mock.c"))
        .output()
        .unwrap();
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    let o = Command::new(&exe).output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(String::from_utf8_lossy(&o.stdout), "ok\n");
}

/// つなぎ（browser/host）を C の偽エンジンとリンクし、`jabberwock build` で作った本体を子プロセスにして通しで動かす。
/// clang と cc がなければ何もしない
#[test]
fn host_and_app_over_pipes() {
    if Command::new("cc").arg("--version").output().is_err() || Command::new("clang").arg("--version").output().is_err() {
        return;
    }
    let dir = scratch("host");
    let app_exe = dir.join("jwbrowser-app");
    let o = Command::new(env!("CARGO_BIN_EXE_jabberwock"))
        .current_dir(app())
        .args(["build", "-o", app_exe.to_str().unwrap(), "main.jw"])
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("browser");
    let host = dir.join("jwbrowser");
    let built = Command::new("cc")
        .arg("-o")
        .arg(&host)
        .arg(root.join("host/jwbrowser.c"))
        .arg(root.join("engine/mock/jwe_mock.c"))
        .arg("-pthread")
        .output()
        .unwrap();
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    let data = dir.join("data");
    std::fs::create_dir_all(&data).unwrap();
    let log = dir.join("mock.log");
    let o = Command::new(&host)
        .args(["--data", data.to_str().unwrap()])
        .env("JWE_MOCK_SCRIPT", app().join("testdata/basic.txt"))
        .env("JWE_MOCK_LOG", &log)
        .env("JWBROWSER_APP", &app_exe)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let calls: Vec<String> = std::fs::read_to_string(&log)
        .unwrap()
        .lines()
        .filter(|l| ["navigate", "tab_", "back", "close"].iter().any(|p| l.starts_with(p)))
        .map(|l| l.to_string())
        .collect();
    assert_eq!(
        calls.join("\n"),
        "tab_new 1\ntab_show 1\nnavigate 1 about:blank\nnavigate 1 https://example.com\n\
         navigate 1 https://duckduckgo.com/?q=rust+%E8%A8%80%E8%AA%9E\nback 1\ntab_new 2\ntab_show 2\n\
         navigate 2 https://a.example/\ntab_show 1\ntab_close 2\nnavigate 1 fail://x\nclose 0"
    );
    let history = std::fs::read_to_string(data.join("history.tsv")).unwrap();
    assert!(history.starts_with("https://example.com\t"), "about: は残さない:\n{}", history);
}
