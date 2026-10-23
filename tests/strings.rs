//! 文字列の埋め込み `"x = {x}"` のテスト

use ward::{run_source, run_source_with};

fn ok(src: &str) -> String {
    let o = run_source(src, false);
    if let Some(d) = &o.error {
        panic!("unexpected error:\n{}", d.render("test.ward", src));
    }
    assert_eq!(o.live_objects, 0, "heap objects leaked");
    let off = run_source_with(src, false, false);
    assert_eq!(o.output, off.output, "elision changed the output");
    o.output
}

fn err(src: &str) -> ward::diag::Diagnostic {
    run_source(src, false).error.expect("expected an error")
}

#[test]
fn embeds_expressions() {
    let src = "
struct Point {
    i64 x;
    i64 y;

    fn i64 len2() { x * x + y * y }
    fn str show() { \"({x}, {y})\" }
}

fn main() {
    let x = 3;
    let name = \"ward\";
    print(\"x = {x}, x+1 = {x + 1}, {name}!\");
    let p = own Point { x: 1, y: 2 };
    print(\"p = {p.show()}, len2 = {p.len2()}, ok = {p.x < p.y}\");
    let o = Some(5);
    print(\"o = {o}\");
    let u8 big = 250;
    print(\"{big + 10}\");
    let s = \"a{x}b\" + \"c\";
    print(s);
    print(\"{{x}} はそのまま {x}\");
}";
    assert_eq!(
        ok(src),
        "x = 3, x+1 = 4, ward!\np = (1, 2), len2 = 5, ok = true\no = Some(5)\n4\na3bc\n{x} はそのまま 3\n"
    );
}

#[test]
fn embedding_does_not_move() {
    let src = "
fn main() {
    let a = own 7;
    print(\"a = {*a}, {a}\");
    print(*a);
    let t = \"{own 1}\";
}";
    let out = ok(src);
    assert!(out.starts_with("a = 7, <own"), "{}", out);
}

#[test]
fn embedding_errors() {
    let e = |body: &str| err(&format!("fn main() {{\n{}\n}}", body));
    assert!(e("print(\"{x}\");").title.contains("未定義の変数"));
    assert!(e("print(\"{}\");").title.contains("空"));
    assert!(e("print(\"{1 + }\");").title.contains("構文エラー"));
    assert!(e("print(\"a { b\");").title.contains("閉じていません"));
    assert!(e("print(\"a } b\");").title.contains("対応する"));
    let d = e("let s = \"x\";\nprint(\"{s + \"y\"}\");");
    assert!(d.note.unwrap().contains("文字列を書けない"));
    // エラーの位置は元のファイルの中の位置
    let d = e("let a = 1;\nprint(\"value: {a + nope}\");");
    assert!(d.render("t.ward", "fn main() {\nlet a = 1;\nprint(\"value: {a + nope}\");\n}").contains("^^^^"));
}
