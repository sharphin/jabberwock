//! パッケージ（jabberwock.toml・`jabberwock new` / `build` / `run` / `test`）

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use jabberwock::project::{parse_manifest, Manifest, Project};

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("jw-project-test-{}-{}", std::process::id(), name));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn write(p: &Path, s: &str) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, s).unwrap();
}

fn jw(dir: &Path, args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_jabberwock")).args(args).current_dir(dir).output().expect("runs");
    (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stdout).to_string(), String::from_utf8_lossy(&o.stderr).to_string())
}

#[test]
fn manifest() {
    let m = parse_manifest(
        "# 注釈\n[package]\nname = \"app\"  # 名前\nversion = \"0.1.0\"\n\n[dependencies]\ngeom = { path = \"../geom\" }\nutil = \"lib/util\"\n",
    )
    .unwrap();
    assert_eq!(
        m,
        Manifest { name: "app".into(), deps: vec![("geom".into(), "../geom".into()), ("util".into(), "lib/util".into())] }
    );
    let e = |t: &str| parse_manifest(t).unwrap_err();
    assert!(e("[dependencies]\n").contains("name"));
    assert!(e("[package]\nname = \"1a\"\n").contains("使えません"));
    assert!(e("[package]\nname = \"a\"\n[dependencies]\nx = { git = \"u\" }\n").contains("手元のフォルダだけ"));
    assert!(e("[package]\nname = \"a\"\n[dependencies]\nx = \"a\"\nx = \"b\"\n").contains("2 回"));
    assert!(e("[package\n").contains("1 行目"));
}

#[test]
fn modules_come_from_src_and_dependencies() {
    let d = tmp("load");
    write(&d.join("app/jabberwock.toml"), "[package]\nname = \"app\"\n[dependencies]\nshapes = { path = \"../shapes\" }\n");
    write(&d.join("app/src/main.jw"), "use shapes;\nuse util;\nfn main() { print(shapes.area(2), util.one()); }\n");
    write(&d.join("app/src/util.jw"), "pub fn i64 one() { 1 }\n");
    // 依存先の依存先も読む
    write(&d.join("shapes/jabberwock.toml"), "[package]\nname = \"shapes\"\n[dependencies]\nnums = { path = \"../nums\" }\n");
    write(&d.join("shapes/src/lib.jw"), "use nums;\nuse sq;\npub fn i64 area(i64 x) { sq.sq(x) + nums.zero() }\n");
    write(&d.join("shapes/src/sq.jw"), "pub fn i64 sq(i64 x) { x * x }\n");
    write(&d.join("nums/jabberwock.toml"), "[package]\nname = \"nums\"\n");
    write(&d.join("nums/src/lib.jw"), "pub fn i64 zero() { 0 }\n");
    let p = Project::load(&d.join("app")).unwrap();
    let names: Vec<&str> = p.modules.keys().map(|s| s.as_str()).collect();
    assert_eq!(names, ["nums", "shapes", "sq", "util"]);
    assert_eq!(Project::find(&d.join("app/src")).as_deref(), Some(d.join("app").as_path()));
    let read = p.reader();
    let src = fs::read_to_string(p.main_file()).unwrap();
    let o = jabberwock::run_program(&src, &read, false, true, Box::new(std::io::Cursor::new(Vec::new())), Vec::new());
    assert!(o.error.is_none(), "{:?}", o.error.map(|d| d.title));
    assert_eq!(o.output, "4 1\n");

    // 同じ名前のモジュールが 2 か所・名前の違う依存先・ライブラリでない依存先はエラー
    write(&d.join("app/src/sq.jw"), "pub fn i64 sq(i64 x) { x }\n");
    assert!(Project::load(&d.join("app")).unwrap_err().contains("モジュール `sq` が 2 か所"));
    fs::remove_file(d.join("app/src/sq.jw")).unwrap();
    write(&d.join("nums/jabberwock.toml"), "[package]\nname = \"numbers\"\n");
    assert!(Project::load(&d.join("app")).unwrap_err().contains("名前が `numbers`"));
    write(&d.join("nums/jabberwock.toml"), "[package]\nname = \"nums\"\n");
    fs::remove_file(d.join("nums/src/lib.jw")).unwrap();
    assert!(Project::load(&d.join("app")).unwrap_err().contains("src/lib.jw がありません"));
    write(&d.join("app/jabberwock.toml"), "[package]\nname = \"app\"\n[dependencies]\nnone = { path = \"../none\" }\n");
    assert!(Project::load(&d.join("app")).unwrap_err().contains("フォルダがありません"));
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn new_run_and_test_commands() {
    let d = tmp("cli");
    let (c, _, err) = jw(&d, &["new", "app"]);
    assert_eq!(c, 0, "{}", err);
    let (c, _, err) = jw(&d, &["new", "mathx", "--lib"]);
    assert_eq!(c, 0, "{}", err);
    assert_eq!(jw(&d, &["new", "app"]).0, 1);
    let app = d.join("app");
    let (c, out, _) = jw(&app.join("src"), &["run"]);
    assert_eq!((c, out.as_str()), (0, "Hello, world!\n"));

    // ライブラリの tests は最初から通る
    let (c, out, _) = jw(&d.join("mathx"), &["test"]);
    assert_eq!(c, 0, "{}", out);
    assert!(out.contains("test tests/add.jw ... ok") && out.contains("1 本成功・0 本失敗"), "{}", out);

    // 依存先を足して、引数も渡す
    write(&app.join("jabberwock.toml"), "[package]\nname = \"app\"\n\n[dependencies]\nmathx = { path = \"../mathx\" }\n");
    write(&app.join("src/main.jw"), "use mathx;\nfn main() {\n    print(mathx.add(2, 3), args());\n}\n");
    let (c, out, err) = jw(&app, &["run", "--", "x", "y"]);
    assert_eq!((c, out.as_str()), (0, "5 [x, y]\n"), "{}", err);

    // エラーの位置はパッケージからの道筋で出す
    write(&d.join("mathx/src/lib.jw"), "pub fn i64 add(i64 a, i64 b) {\n    a + true\n}\n");
    let (c, _, err) = jw(&app, &["run"]);
    assert_eq!(c, 1);
    assert!(err.contains("mathx/src/lib.jw:2:5") || err.contains("lib.jw:2:5"), "{}", err);
    write(&d.join("mathx/src/lib.jw"), "pub fn i64 add(i64 a, i64 b) {\n    a + b\n}\n");

    // test: 出力の違い・実行時エラーは失敗
    write(&app.join("tests/good.jw"), "use mathx;\nfn main() { print(mathx.add(1, 1)); }\n");
    write(&app.join("tests/good.out"), "2\n");
    write(&app.join("tests/wrong.jw"), "fn main() { print(1); }\n");
    write(&app.join("tests/wrong.out"), "2\n");
    write(&app.join("tests/boom.jw"), "fn main() { let v = [1]; let i = 3; print(v[i]); }\n");
    let (c, out, _) = jw(&app, &["test"]);
    assert_eq!(c, 1);
    assert!(out.contains("test tests/good.jw ... ok"), "{}", out);
    assert!(out.contains("test tests/wrong.jw ... 失敗") && out.contains("出力が違います"), "{}", out);
    assert!(out.contains("test tests/boom.jw ... 失敗") && out.contains("tests/boom.jw:1:"), "{}", out);
    assert!(out.contains("1 本成功・2 本失敗"), "{}", out);

    // パッケージの外
    let (c, _, err) = jw(&std::env::temp_dir(), &["test"]);
    assert_eq!(c, 1);
    assert!(err.contains("jabberwock.toml が見つかりません"), "{}", err);
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn build_makes_an_executable_once() {
    if Command::new(std::env::var("JW_CLANG").unwrap_or_else(|_| "clang".into())).arg("--version").output().is_err() {
        return;
    }
    let d = tmp("build");
    assert_eq!(jw(&d, &["new", "hello"]).0, 0);
    let p = d.join("hello");
    let (c, _, err) = jw(&p, &["build"]);
    assert_eq!(c, 0, "{}", err);
    assert!(err.contains("作りました"), "{}", err);
    let exe = jabberwock::project::Project::load(&p).unwrap().exe_path(false);
    let out = Command::new(&exe).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout), "Hello, world!\n");
    // 変わっていなければ作り直さない
    let (_, _, err) = jw(&p, &["build"]);
    assert!(err.contains("最新です"), "{}", err);
    let (c, out, _) = jw(&p, &["run", "--native"]);
    assert_eq!((c, out.as_str()), (0, "Hello, world!\n"));
    let _ = fs::remove_dir_all(&d);
}
