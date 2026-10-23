//! クロージャ（関数を値として渡す）と sort / sort_by のテスト

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
fn closures_capture_values() {
    let src = "
fn i64 apply(fn i64(i64) f, i64 x) { f(x) }

fn fn i64(i64) adder(i64 n) { (x) -> x + n }

fn main() {
    let k = 10;
    let add = (i64 x) -> x + k;
    print(add(1), add(2));
    print(apply((x) -> x * 2, 21));
    print(apply(add, 5));
    let a3 = adder(3);
    print(a3(4));
    let g = () -> { print(\"k = {k}\"); };
    g();
    print(g);
}";
    assert_eq!(ok(src), "11 12\n42\n15\n7\nk = 10\n<関数>\n");
}

#[test]
fn own_is_moved_into_the_closure() {
    let src = "
fn i64 twice(fn i64(i64) f, i64 x) { f(f(x)) }

fn main() {
    let big = own 3;
    let f = (i64 x) -> x * *big;
    print(f(1), twice(f, 2));
}";
    // f は own を持つのでムーブ。twice に渡したあとで解放される
    assert_eq!(ok(src), "3 18\n");
    let e = err("fn main() {\nlet a = own 1;\nlet f = () -> *a;\nprint(*a);\n}");
    assert!(e.title.contains("ムーブ済み"));
    let e = err("fn main() {\nlet a = own 1;\nlet f = () -> a;\nlet b = f();\n}");
    assert!(e.title.contains("取り出せません"));
}

#[test]
fn named_functions_are_values() {
    let src = "
fn bool desc(i64 a, i64 b) { a > b }
fn i64 inc(i64 x) { x + 1 }

fn main() {
    let mut v = [3, 1, 2];
    v.sort_by(desc);
    print(v);
    let fn i64(i64)[] fs = [inc, (x) -> x * 10];
    for f in fs { print(f(5)); }
}";
    assert_eq!(ok(src), "[3, 2, 1]\n6\n50\n");
}

#[test]
fn captured_refs_are_checked() {
    let o = run_source(
        "
fn main() {
    let mut f = () -> 0;
    if true {
        let x = 5;
        let r = ref x;
        f = () -> *r;
    }
    print(f());
}",
        false,
    );
    assert!(o.error.unwrap().title.contains("dangling"));
}

#[test]
fn sorting() {
    let src = "
struct P {
    str name;
    i64 age;
}

fn main() {
    let mut v = [3, 1, 2];
    v.sort();
    let mut ss = [\"b\", \"c\", \"a\"];
    ss.sort();
    let mut xs = [1.5, -0.5];
    xs.sort();
    print(v, ss, xs);
    // 同じ値の要素は元の順のまま（安定）
    let mut ps = [P { name: \"a\", age: 30 }, P { name: \"b\", age: 20 }, P { name: \"c\", age: 30 }];
    ps.sort_by((x, y) -> x.age < y.age);
    for p in ps { print(p.name); }
    let mut os = [own 3, own 1, own 2];
    os.sort_by((a, b) -> *a < *b);
    print(os);
}";
    assert_eq!(ok(src), "[1, 2, 3] [a, b, c] [-0.5, 1.5]\nb\na\nc\n[1, 2, 3]\n");
}

#[test]
fn closure_errors() {
    let e = |body: &str| err(&format!("fn main() {{\n{}\n}}", body));
    assert!(e("let mut n = 1;\nlet f = () -> { n = 2; };").title.contains("書き換えられません"));
    assert!(e("let f = (x) -> x;").title.contains("型が分かりません"));
    assert!(e("let f = () -> { return 1; };").title.contains("return"));
    assert!(e("let x = 1;\nlet f = (i64 x) -> x;").title.contains("すでにある"));
    assert!(e("let n = 1;\nn(2);").title.contains("関数ではありません"));
    assert!(e("let mut w = [true];\nw.sort();").title.contains("sort"));
    assert!(e("let mut v = [3, 1];\nv.sort_by((i64 a, i64 b) -> 1);").title.contains("型が合いません"));
    assert!(e("let v = [3, 1];\nv.sort();").title.contains("mut"));
    let d = err("fn f() {}\nfn main() {\nlet f = 1;\n}");
    assert!(d.title.contains("関数の名前"));
}
