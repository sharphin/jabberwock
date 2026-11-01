//! 軽量ブラウザの試作（browser/）のテスト。本体（browser/app の jabberwock）は jwc で実行ファイルにして動かす
//! （std.browser・std.crypto は jwc だけにある）。ブラウザは C の偽エンジン（runtime/engine/mock）とリンクし、
//! 台本（testdata/*.txt）を流したときにエンジンへ出す操作（偽エンジンの記録）が testdata/*.out と同じかを見る。
//! 出力を直すときは `JW_BLESS=1 cargo test --test browser` で .out を作り直す。clang がなければ何もしない。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use jabberwock::{llvm_build, native};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn app() -> PathBuf {
    root().join("browser/app")
}

fn have_clang() -> bool {
    Command::new(native::clang()).arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn have_cc() -> bool {
    Command::new("cc").arg("--version").output().is_ok()
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("jw-browser-test-{}-{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// jwc の実行ファイル（テストの中で 1 回だけ作る。JW_SAFETY_JWC があればそれを使う）
fn jwc() -> PathBuf {
    if let Ok(path) = std::env::var("JW_SAFETY_JWC") {
        return PathBuf::from(path);
    }
    static EXE: Mutex<Option<PathBuf>> = Mutex::new(None);
    let mut g = EXE.lock().unwrap_or_else(|e| e.into_inner());
    g.get_or_insert_with(|| {
        let dir = root().join("jwc");
        let src = std::fs::read_to_string(dir.join("main.jw")).unwrap();
        let read = move |name: &str| std::fs::read_to_string(dir.join(format!("{}.jw", name))).ok();
        // デバッグビルドは長い式でスタックがあふれるので、大きなスタックのスレッドで書き出す
        let ll = std::thread::Builder::new()
            .stack_size(256 << 20)
            .spawn(move || llvm_build(&src, &read, "jwc/main.jw", false))
            .unwrap()
            .join()
            .unwrap()
            .expect("jwc を LLVM に書き出せる");
        let out = std::env::temp_dir().join(format!("jw-browser-jwc-{}", std::process::id()));
        native::compile(&ll, &out, "jwc", "-O1").expect("jwc を clang でコンパイルできる")
    })
    .clone()
}

/// browser/app の name.jw を jwc で実行ファイルにする（1 回だけ）。mock なら偽エンジンとリンクする
fn build(name: &str, mock: bool) -> PathBuf {
    static BUILT: Mutex<Option<HashMap<String, PathBuf>>> = Mutex::new(None);
    let mut g = BUILT.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(p) = g.get_or_insert_with(HashMap::new).get(name) {
        return p.clone();
    }
    let dir = std::env::temp_dir().join(format!("jw-browser-bin-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let exe = dir.join(name);
    let mut cmd = Command::new(jwc());
    cmd.args(["build", "--clang", &native::clang()]);
    if mock {
        cmd.arg("--link").arg(root().join("runtime/engine/mock/jwe_mock.c"));
    }
    let o = cmd.arg("-o").arg(&exe).arg(app().join(format!("{}.jw", name))).output().unwrap();
    assert!(o.status.success(), "{}.jw を作れない:\n{}", name, String::from_utf8_lossy(&o.stderr));
    let exe = if exe.exists() { exe } else { exe.with_extension("exe") };
    g.as_mut().unwrap().insert(name.to_string(), exe.clone());
    exe
}

/// 作った name を動かし、標準出力を返す（失敗なら止める）。金庫は std.browser を使うので偽エンジンとリンクする
fn run(name: &str, args: &[&str]) -> String {
    let o = Command::new(build(name, name == "vault_main")).current_dir(app()).args(args).output().unwrap();
    let out = String::from_utf8_lossy(&o.stdout).into_owned();
    assert!(o.status.success(), "{} {:?} が失敗した:\n{}\n{}", name, args, out, String::from_utf8_lossy(&o.stderr));
    out
}

/// ブラウザを偽エンジンで動かし、偽エンジンの記録（呼ばれた操作）を返す
fn browse(script: &Path, args: &[&str]) -> String {
    // テストは並んで動くので、記録の置き場所は毎回別にする
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let n = COUNT.fetch_add(1, Ordering::SeqCst);
    let dir = scratch(&format!("log-{}-{}", script.file_stem().unwrap().to_string_lossy(), n));
    let log = dir.join("mock.log");
    let o = Command::new(build("main", true))
        .current_dir(app())
        .args(args)
        .env("JWE_MOCK_SCRIPT", script)
        .env("JWE_MOCK_LOG", &log)
        .env("JW_BROWSER_CLOCK", "1793500000000") // 履歴の時刻を毎回同じにする（1 件ごとに 1 分進む）
        .output()
        .unwrap();
    assert!(o.status.success(), "{:?} が失敗した:\n{}", args, String::from_utf8_lossy(&o.stderr));
    std::fs::read_to_string(&log).unwrap()
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

fn slash(p: &Path) -> String {
    p.to_str().unwrap().replace('\\', "/")
}

#[test]
fn parts() {
    if !have_clang() {
        return;
    }
    for f in ["test_address", "test_store", "test_chrome"] {
        assert_eq!(run(f, &[]), "ok\n", "{}", f);
    }
    let dir = scratch("vault-part");
    let file = dir.join("v.dat");
    assert_eq!(run("test_vault", &[file.to_str().unwrap()]), "ok\n");
}

#[test]
fn scripts() {
    if !have_clang() {
        return;
    }
    for name in ["basic", "tabs", "settings", "history", "menu"] {
        let data = scratch(&format!("script-{}", name));
        let got = browse(&app().join(format!("testdata/{}.txt", name)), &["--data", &slash(&data), "--ephemeral"]);
        same_as_file(&got, &format!("{}.out", name));
    }
}

#[test]
fn keeps_history_and_bookmarks_in_one_folder() {
    if !have_clang() {
        return;
    }
    let dir = scratch("data");
    let d = slash(&dir);
    std::fs::write(dir.join("bookmarks.tsv"), "https://saved.example/\t保存した\n").unwrap();
    // 時刻のない前の形の履歴も読める（時刻は 0 として外枠に出す）
    std::fs::write(dir.join("history.tsv"), "https://old.example/\t前の形\n").unwrap();
    let log = browse(&app().join("testdata/tabs.txt"), &["--data", &d]);
    assert!(log.contains("\nbm\thttps://saved.example/\t保存した\n"), "前のブックマークが外枠に出る:\n{}", log);
    browse(&app().join("testdata/basic.txt"), &["--data", &d]);
    let history = std::fs::read_to_string(dir.join("history.tsv")).unwrap();
    assert!(history.starts_with("https://old.example/\t前の形\t0\n"), "{}", history);
    assert!(history.contains("https://example.com\ttitle:https://example.com\t1793500060000\n"), "{}", history);
    assert!(!history.contains("fail://x"), "読み込めなかったページは残さない:\n{}", history);
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2, "置くのは history.tsv と bookmarks.tsv だけ");
    // --ephemeral なら何も書かない
    let eph = scratch("ephemeral");
    browse(&app().join("testdata/basic.txt"), &["--data", &slash(&eph), "--ephemeral"]);
    assert_eq!(std::fs::read_dir(&eph).unwrap().count(), 0);
}

/// 設定の「起動したときに開くページ」と、settings.tsv の読み書き
#[test]
fn startup_page_and_settings_file() {
    if !have_clang() {
        return;
    }
    // ホームページ（打った形のまま書いてあっても URL にする）
    let dir = scratch("home");
    let d = slash(&dir);
    std::fs::write(dir.join("settings.tsv"), "startup\thome\nhome\texample.org\nbogus\tx\n").unwrap();
    let log = browse(&app().join("testdata/basic.txt"), &["--data", &d]);
    assert!(log.contains("tab_new 1\ntab_show 1\nset_title 0 jabberwock\nnavigate 1 https://example.org\n"), "{}", log);
    // 設定を変えると settings.tsv に書く（項目の順。既定から変えたものだけ。読めない行は捨てる）
    browse(&app().join("testdata/settings.txt"), &["--data", &d]);
    let text = std::fs::read_to_string(dir.join("settings.tsv")).unwrap();
    assert_eq!(text, "startup\thome\nhome\texample.org\nnewtab\tblank\nsearch\tgoogle\n");

    // 前回開いていたタブ
    let dir = scratch("last");
    let d = slash(&dir);
    std::fs::write(dir.join("settings.tsv"), "startup\tlast\n").unwrap();
    std::fs::write(dir.join("session.tsv"), "https://a.example/\tA\nhttps://b.example/\tB\n").unwrap();
    let log = browse(&app().join("testdata/tabs.txt"), &["--data", &d]);
    assert!(
        log.contains("navigate 1 https://a.example/\n") && log.contains("navigate 2 https://b.example/\n") && log.contains("tab_show 1\n"),
        "{}",
        log
    );
    // 終わるときに開いていたタブを書く（tabs.txt は最後に b.example... を閉じて空のタブだけ残す）
    let session = std::fs::read_to_string(dir.join("session.tsv")).unwrap();
    assert!(!session.contains("about:blank"), "{}", session);
}

/// パスワードの金庫（vault_main.jw）を偽エンジンの台本で動かす（鍵を作る回数は 2 にして速くする）
#[test]
fn vault_script() {
    if !have_clang() {
        return;
    }
    let dir = scratch("vault");
    let d = slash(&dir);
    // vault.txt は「金庫の窓を閉じたら閉じる」の設定で（しばらく使わなければ閉じる）
    std::fs::write(dir.join("settings.tsv"), "vault_remember\toff\n").unwrap();
    let out = run("vault_main", &["--data", &d, "--iterations", "2", "--fake", "testdata/vault.txt", "--log"]);
    same_as_file(&out, "vault.out");
    // 既定の「ブラウザを開いている間」: 窓を閉じても隠すだけ・PIN・ブラウザが終われば終わる
    let keep = scratch("vault-keep");
    let out = run("vault_main", &["--data", &slash(&keep), "--iterations", "2", "--fake", "testdata/vault_keep.txt", "--log"]);
    same_as_file(&out, "vault_keep.out");
    // この PC で覚える（Linux・macOS では std.keystore が Err なので、覚えられないと知らせて、鍵のファイルは作らない）
    if !cfg!(windows) {
        let pc = scratch("vault-pc");
        std::fs::write(pc.join("settings.tsv"), "vault_remember\tpc\n").unwrap();
        let out = run("vault_main", &["--data", &slash(&pc), "--iterations", "2", "--fake", "testdata/vault_pc.txt", "--log"]);
        assert!(out.contains("この PC で覚えられませんでした: この OS ではまだ秘密を OS に守らせられません"), "{}", out);
        assert!(!pc.join("vault.key").exists());
    }
    let text = std::fs::read_to_string(dir.join("vault.dat")).unwrap();
    assert!(text.starts_with("jwvault 1\nkdf pbkdf2-sha256 2 "), "{}", text);
    // パスワード・ユーザー名・サイトは平文で残さない
    for plain in ["pw2", "example.com", "まつ", "メモ"] {
        assert!(!text.contains(plain), "{} が vault.dat にそのまま入っている", plain);
    }
}

#[test]
fn c_mock_engine() {
    if !have_cc() {
        return;
    }
    let dir = scratch("cmock");
    let exe = dir.join("mock_test");
    let built = Command::new("cc")
        .arg("-Wall")
        .arg("-o")
        .arg(&exe)
        .arg(root().join("browser/engine/tests/mock_test.c"))
        .arg(root().join("runtime/engine/mock/jwe_mock.c"))
        .output()
        .unwrap();
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    let o = Command::new(&exe).output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(String::from_utf8_lossy(&o.stdout), "ok\n");
}

/// ブラウザが金庫（子プロセス）とやりとりする。金庫の代わりに、決まった行を出して届いた行を書き留める小さなシェルの台本を
/// ブラウザと同じフォルダの jwvault-app に置く。ブラウザは偽エンジンなので、台本の後ろに何もしない行を並べて、金庫の返事が届くまで回す
#[cfg(unix)]
#[test]
fn browser_talks_to_vault() {
    if !have_clang() {
        return;
    }
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch("link");
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let got = dir.join("got.txt");
    let fake = bin.join("jwvault-app");
    std::fs::write(
        &fake,
        format!(
            "#!/bin/sh\necho \"$@\" > '{0}'\nprintf 'ask_site\\nfill\\thttps://a.example\\tu\\\\tser\\tp\\\\\\\\w\\nfill\\thttps://b.example\\tu\\tpw\\n'\ncat >> '{0}'\n",
            got.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    let script = dir.join("script.txt");
    let mut text = String::from("chrome go https://a.example/login\nchrome vault\n");
    for _ in 0..3000 {
        text.push_str("chrome noop\n");
    }
    std::fs::write(&script, text).unwrap();
    let data = dir.join("data");
    std::fs::create_dir_all(&data).unwrap();
    let log = dir.join("mock.log");
    // 金庫はブラウザと同じフォルダから起こす（PATH は探さない）ので、ブラウザを台本の横へハードリンクで置く
    // （写すと、書き込み中の口がほかのテストの子プロセスに受け継がれ、起こすときに "Text file busy" になることがある）
    let main = bin.join("main");
    std::fs::hard_link(build("main", true), &main).unwrap();
    let o = Command::new(&main)
        .args(["--data", data.to_str().unwrap()])
        .env("JWE_MOCK_SCRIPT", &script)
        .env("JWE_MOCK_LOG", &log)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let calls = std::fs::read_to_string(&log).unwrap();
    // 同じサイトなら入れ（ユーザー名・パスワードの \t \\ は戻す）、違うサイトはエンジンが断る
    assert!(calls.contains("fill 1 https://a.example u\tser (3)\n"), "{}", calls);
    assert!(calls.contains("fill_refused 1 https://b.example\n"), "{}", calls);
    assert!(calls.contains("window 0 4\n"), "入れたらブラウザを手前に出す:\n{}", calls);
    let said = std::fs::read_to_string(&got).unwrap();
    assert!(said.starts_with(&format!("--data {}\n", data.display())), "{}", said);
    assert!(said.contains("site\thttps://a.example/login\n"), "{}", said);
    assert!(said.contains("filled\t0\nfilled\t3\n"), "{}", said);
}

/// 金庫（std.browser の偽エンジン）を、テストがブラウザの代わりになって標準入出力でつないで動かす
#[test]
fn vault_talks_to_browser() {
    if !have_clang() {
        return;
    }
    use std::io::Write;
    let dir = scratch("vault-link");
    let script = dir.join("script.txt");
    let mut text = String::from("chrome create correct horse\nchrome save https://a.example\tu\tsecret-pw\t\t\t\n");
    // ブラウザからの site の行が先に届くよう、何もしない行をはさむ
    for _ in 0..200 {
        text.push_str("chrome noop\n");
    }
    text.push_str("chrome fill https://a.example\tu\nchrome copy https://a.example\tu\nchrome lock\nclose\n");
    std::fs::write(&script, text).unwrap();
    let log = dir.join("mock.log");
    let mut child = Command::new(build("vault_main", true))
        .args(["--data", dir.to_str().unwrap(), "--iterations", "2"])
        .env("JWE_MOCK_SCRIPT", &script)
        .env("JWE_MOCK_LOG", &log)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    input.write_all(b"site\thttps://a.example/login\n").unwrap();
    input.flush().unwrap();
    let o = child.wait_with_output().unwrap();
    drop(input);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let said = String::from_utf8_lossy(&o.stdout);
    assert!(said.contains("ask_site\n"), "{}", said);
    assert!(said.contains("fill\thttps://a.example\tu\tsecret-pw\n"), "{}", said);
    let calls = std::fs::read_to_string(&log).unwrap();
    assert!(calls.contains("chrome_height 0 full"), "{}", calls);
    assert!(calls.contains("msg\tok\t金庫を作りました"), "{}", calls);
    assert!(calls.contains("site\thttps://a.example\n"), "ブラウザのページの出どころを画面に出す:\n{}", calls);
    assert!(calls.contains("mode\tlocked"), "{}", calls);
    assert!(!calls.contains("secret-pw"), "パスワードは画面に送らない（見るを押していない）:\n{}", calls);
    let text = std::fs::read_to_string(dir.join("vault.dat")).unwrap();
    assert!(!text.contains("secret-pw") && !text.contains("a.example"));
}
