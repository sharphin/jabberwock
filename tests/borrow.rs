//! own でない値を借りる（変数・フィールド・配列の要素）と、`ref` の引数を自動で借りるテスト

use jabberwock::{run_source, run_source_with, Outcome};

fn run_ok(src: &str) -> Outcome {
    let o = run_source(src, false);
    if let Some(d) = &o.error {
        panic!("unexpected error:\n{}", d.render("test.jw", src));
    }
    assert_eq!(o.live_objects, 0, "heap objects leaked");
    let off = run_source_with(src, false, false);
    assert_eq!(o.output, off.output, "elision changed the output");
    o
}

fn ok(src: &str) -> String {
    run_ok(src).output
}

fn err(src: &str) -> jabberwock::diag::Diagnostic {
    let o = run_source(src, false);
    assert_eq!(o.live_objects, 0, "heap objects leaked");
    o.error.expect("expected an error")
}

fn dangling(src: &str) -> jabberwock::diag::Diagnostic {
    let d = err(src);
    assert!(d.title.contains("dangling"), "unexpected error: {}", d.title);
    d
}

const SHAPES: &str = "
struct Point { i64 x; i64 y; }
struct Line { Point a; Point b; }
";

#[test]
fn borrow_plain_variables_and_fields() {
    let src = format!("{SHAPES}
fn shift(ref mut Point p, i64 d) {{
    p.x += d;
    p.y += d;
}}

fn main() {{
    let mut n = 1;
    let r = ref mut n;
    *r += 5;
    print(n);
    let mut l = Line {{ a: Point {{ x: 0, y: 0 }}, b: Point {{ x: 1, y: 1 }} }};
    shift(ref mut l.b, 10);
    print(l.b.x, l.b.y);
    let i64[3] mut arr = [1, 2, 3];
    let e = ref mut arr[1];
    *e = 20;
    print(arr);
    let o = own Line {{ a: Point {{ x: 2, y: 2 }}, b: Point {{ x: 3, y: 3 }} }};
    let pa = ref o.a;
    print(pa.x);
}}");
    assert_eq!(ok(&src), "6\n11 11\n[1, 20, 3]\n2\n");
}

#[test]
fn a_borrowed_variable_sees_reassignment() {
    // 変数を借りた ref は、その変数の場所を指す。再代入した値が見える
    let src = "
fn main() {
    let mut s = \"a\";
    let r = ref s;
    s = \"b\";
    print(*r);
}";
    assert_eq!(ok(src), "b\n");
}

#[test]
fn lists_can_be_borrowed_whole() {
    let src = "
fn add(ref mut i64[] v, i64 x) {
    v.push(x);
}

fn main() {
    let i64[] mut v = [];
    add(ref mut v, 1);
    add(ref mut v, 2);
    let r = ref v;
    print(r.len(), r[1], v);
}";
    assert_eq!(ok(src), "2 2 [1, 2]\n");
}

#[test]
fn match_on_a_borrowed_enum_field() {
    let src = "
struct Node {
    i64 key;
    Option<own Node> next;
}

fn i64 length(ref Option<own Node> list) {
    match list {
        case None -> 0;
        case Some(n) -> 1 + length(n.next);
    }
}

fn main() {
    let mut head = Node { key: 1, next: None };
    head.next = Some(own Node { key: 2, next: None });
    print(length(head.next) + 1);
}";
    assert_eq!(ok(src), "2\n");
}

#[test]
fn borrowed_variables_dangle_when_they_die() {
    let d = dangling("
fn main() {
    let z = 0;
    let mut r = ref z;
    if true {
        let x = 5;
        r = ref x;
    }
    print(*r);
}");
    assert!(d.labels.iter().any(|l| l.text.contains("スコープを抜けた")));
    let d = dangling("
struct S { i64 a; }
fn main() {
    let s = S { a: 1 };
    let r = ref s;
    let t = s;
    print(r.a);
}");
    assert!(d.labels.iter().any(|l| l.text.contains("ムーブされた")));
    dangling("
fn ref i64 bad() {
    let x = 1;
    ref x
}
fn main() {
    let r = bad();
    print(*r);
}");
}

#[test]
fn borrow_errors() {
    let d = err("struct S { i64 a; }\nfn main() {\n    let S[str] mut m = [\"k\": S { a: 1 }];\n    let r = ref m[\"k\"];\n}");
    assert!(d.title.contains("辞書の中の値は借りられません"), "{}", d.title);
    let d = err("fn main() {\n    let i64[str] m = [\"k\": 1];\n    let r = ref m[\"k\"];\n}");
    assert!(d.title.contains("辞書の中の値は借りられません"), "{}", d.title);
    let d = err("fn main() {\n    let x = 1;\n    let r = ref mut x;\n}");
    assert!(d.title.contains("mut ではない"), "{}", d.title);
    let d = err("fn main() {\n    let r = ref 5;\n}");
    assert!(d.title.contains("借用できない式"), "{}", d.title);
}

// ---------------------------------------------------------------- `ref` の引数は自動で借りる

#[test]
fn ref_params_borrow_automatically() {
    let src = "
interface Shape {
    fn i64 area();
}

struct Sq {
    i64 s;

    fn i64 area() { s * s }
}

fn show<T: Shape>(ref T x) {
    print(x.area());
}

fn show_any(ref Shape x) {
    print(x.area());
}

fn i64 sum(ref i64[] v) {
    let mut t = 0;
    for x in v {
        t += x;
    }
    t
}

fn main() {
    let a = Sq { s: 3 };
    let b = own Sq { s: 4 };
    show(a);
    show(b);
    show_any(b);
    print(a.s, b.s); // どちらもムーブされていない
    let i64[] w = [4, 5];
    print(sum(w), w);
}";
    assert_eq!(ok(src), "9\n16\n16\n3 4\n9 [4, 5]\n");
}

#[test]
fn ref_mut_params_still_need_ref_mut() {
    let d = err("fn inc(ref mut i64 x) {\n    *x += 1;\n}\nfn main() {\n    let mut n = 1;\n    inc(n);\n}");
    assert!(d.title.contains("型が合いません"), "{}", d.title);
    assert!(d.note.as_deref().unwrap_or("").contains("ref mut x"));
}

#[test]
fn auto_borrowed_own_arguments_keep_their_checks_elided() {
    // `get(a)` は `get(ref a)` と同じ。own 変数を直接借りたので、関数の中の検査は消せる
    let o = run_ok("
fn i64 get(ref i64 p) {
    *p
}

fn main() {
    let a = own 5;
    print(get(a), get(a));
}");
    assert_eq!(o.output, "5 5\n");
    assert_eq!(o.ref_checks, 0);
}

#[test]
fn methods_on_struct_variables_borrow_the_variable() {
    let src = "
struct Counter {
    i64 n;

    fn mut bump() { n += 1; }
    fn ref Counter me() { this }
}

fn main() {
    let mut c = Counter { n: 0 };
    c.bump();
    c.bump();
    let r = c.me();
    print(r.n);
}";
    assert_eq!(ok(src), "2\n");
}

// ---------------------------------------------------------------- リストの要素を借りる

#[test]
fn list_elements_can_be_borrowed() {
    let src = format!("{SHAPES}
fn shift(ref mut Point p, i64 d) {{
    p.x += d;
}}

fn i64 sum(ref Point p) {{
    p.x + p.y
}}

fn main() {{
    let Point[] mut ps = [Point {{ x: 1, y: 2 }}, Point {{ x: 3, y: 4 }}];
    let r = ref mut ps[1];
    r.y = 40;
    shift(ref mut ps[0], 10);
    print(sum(ps[0]), sum(ps[1]));
    let px = ref ps[0].x;
    print(*px);
    let Line[] mut ls = [Line {{ a: Point {{ x: 0, y: 0 }}, b: Point {{ x: 5, y: 6 }} }}];
    let b = ref mut ls[0].b;
    b.x = 50;
    print(ls[0].b.x);
    print(ps);
}}");
    assert_eq!(ok(&src), "13 43\n11\n50\n[Point { x: 11, y: 2 }, Point { x: 3, y: 40 }]\n");
}

#[test]
fn methods_on_list_elements_borrow_the_element() {
    let src = "
struct Counter {
    i64 n;

    fn mut bump() { n += 1; }
    fn i64 get() { n }
}

fn main() {
    let Counter[] mut cs = [Counter { n: 0 }, Counter { n: 10 }];
    cs[1].bump();
    cs[1].bump();
    cs[0].bump();
    print(cs[0].get(), cs[1].get());
    for i in range(cs.len()) {
        cs[i].bump();
    }
    print(cs[0].n, cs[1].n);
}";
    assert_eq!(ok(src), "1 12\n2 13\n");
}

#[test]
fn element_refs_survive_moves_and_element_writes() {
    // 要素を借りた ref は、リストを別の変数へムーブしても有効。要素への代入は新しい値が見える
    let src = "
struct P { i64 x; }

fn main() {
    let P[] mut a = [P { x: 1 }, P { x: 2 }];
    let r = ref a[1];
    let mut b = a;
    print(r.x);
    b[1] = P { x: 20 };
    print(r.x);
    b[1].x = 30;
    print(r.x);
    let i64[][] mut g = [[1, 2], [3]];
    let e = ref g[0][1];
    g.push([4]);
    print(*e, g);
}";
    assert_eq!(ok(src), "2\n20\n30\n2 [[1, 2], [3], [4]]\n");
}

#[test]
fn changing_the_length_invalidates_element_refs() {
    for (op, extra) in [
        ("a.push(P { x: 3 });", ""),
        ("let _ = a.pop();", ""),
        ("let _ = a.remove(0);", ""),
        ("a.insert(0, P { x: 0 });", ""),
        ("a.reverse();", ""),
        ("a.sort_by((p, q) -> p.x < q.x);", ""),
        ("let d = a.drain();", "print(d.len());"),
    ] {
        let src = format!("
struct P {{ i64 x; }}

fn main() {{
    let P[] mut a = [P {{ x: 1 }}, P {{ x: 2 }}];
    let r = ref a[1];
    print(r.x);
    {op}
    {extra}
    print(r.x);
}}");
        let d = dangling(&src);
        let name = op.split('.').nth(1).and_then(|m| m.split('(').next()).expect("a method");
        assert!(
            d.labels.iter().any(|l| l.text.contains(&format!("`{}` でリストの要素が動いた", name))),
            "{}: {:?}",
            op,
            d.labels
        );
    }
}

#[test]
fn element_refs_dangle_when_the_list_dies() {
    let d = dangling("
struct P { i64 x; }

fn ref P first(ref P[] a) {
    ref a[0]
}

fn ref P bad() {
    let P[] a = [P { x: 1 }];
    first(a)
}

fn main() {
    let r = bad();
    print(r.x);
}");
    assert!(d.labels.iter().any(|l| l.text.contains("スコープを抜けた")), "{:?}", d.labels);
    // リストに入れ直すと、古いリストの要素を借りた ref は無効
    let d = dangling("
fn main() {
    let i64[] mut a = [1, 2];
    let r = ref a[0];
    a = [3];
    print(*r);
}");
    assert!(d.labels.iter().any(|l| l.text.contains("再代入")), "{:?}", d.labels);
}

#[test]
fn element_refs_dangle_after_sending_the_list() {
    let d = dangling("
fn main() {
    let i64[] a = [1, 2];
    let r = ref a[0];
    let t = spawn(() -> a.len());
    print(t.join());
    print(*r);
}");
    assert!(d.labels.iter().any(|l| l.text.contains("ほかのスレッドへ送られた")), "{:?}", d.labels);
}
