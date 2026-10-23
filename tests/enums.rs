//! enum と Option のテスト

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

const SHAPE: &str = "enum Shape {
    Circle(i64 r);
    Rect(i64 w, i64 h);
    Empty;
}
";

// ---------------------------------------------------------------- enum

#[test]
fn enums_carry_data_and_match_takes_it_out() {
    let src = format!("{SHAPE}
fn i64 area(Shape s) {{
    match s {{
        case Circle(r) -> 3 * r * r;
        case Rect(w, h) -> w * h;
        case Empty -> 0;
    }}
}}

fn main() {{
    print(area(Shape.Circle(2)), area(Shape.Rect(3, 4)), area(Shape.Empty));
    let s = Shape.Rect(5, 6);
    print(s, Shape.Empty);
    print(area(s), area(s));
}}");
    // 中身が数だけの enum はコピーされる（2 回渡せる）
    assert_eq!(ok(&src), "12 12 0\nRect(5, 6) Empty\n30 30\n");
}

#[test]
fn match_must_cover_every_variant() {
    let d = err(&format!("{SHAPE}fn main() {{\n    let s = Shape.Empty;\n    match s {{\n        case Circle(r) -> print(r);\n    }}\n}}"));
    assert!(d.title.contains("Rect・Empty の場合が書かれていません"), "{}", d.title);
    assert!(d.note.unwrap().contains("default"));
    // default でまとめてよい
    let src = format!("{SHAPE}fn main() {{\n    let s = Shape.Empty;\n    match s {{\n        case Circle(r) -> print(r);\n        default -> print(\"other\");\n    }}\n}}");
    assert_eq!(ok(&src), "other\n");
    // 種類をまとめた case
    let src = format!("{SHAPE}fn main() {{\n    let s = Shape.Rect(1, 2);\n    let n = match s {{\n        case Circle(_), Rect(_, _) -> 1;\n        case Empty -> 0;\n    }};\n    print(n);\n}}");
    assert_eq!(ok(&src), "1\n");
}

#[test]
fn enum_errors() {
    let e = |body: &str| err(&format!("{SHAPE}fn main() {{\n{}\n}}", body));
    assert!(e("let s = Shape.Square(1);").title.contains("種類 `Square` はありません"));
    assert!(e("let s = Shape.Circle(1, 2);").title.contains("引数の数"));
    assert!(e("let s = Shape.Circle(true);").title.contains("型が合いません"));
    let d = e("let s = Shape.Empty;\nmatch s {\n case Circle -> print(1);\n default -> print(2);\n}");
    assert!(d.title.contains("中身の数が違います"), "{}", d.title);
    assert!(e("let s = Shape.Empty;\nmatch s {\n case Box -> print(1);\n default -> print(2);\n}").title.contains("種類 `Box` はありません"));
    assert!(e("let s = Shape.Empty;\nmatch s {\n case 1 -> print(1);\n default -> print(2);\n}").title.contains("種類の名前を書きます"));
    assert!(e("match 1 {\n case Empty -> print(1);\n default -> print(2);\n}").title.contains("enum ではない値"));
    assert!(e("let s = Shape.Empty;\nmatch s {\n case Empty -> print(1);\n case Empty -> print(2);\n default -> print(3);\n}").title.contains("同じ値の case"));
    assert!(err("enum A { X; X; }\nfn main() {\n}").title.contains("二重"));
    assert!(err("enum A { X(i64); }\nfn main() {\n}").title.contains("型と名前"));
    assert!(err("enum A { X, Y }\nfn main() {\n}").title.contains("区切りは `;`"));
    assert!(err("enum A { X(A a); }\nfn main() {\n}").title.contains("自分自身をじかに含んで"));
    assert!(err("struct A { i64 x; }\nenum A { X; }\nfn main() {\n}").title.contains("二重"));
}

// ---------------------------------------------------------------- 所有権

#[test]
fn enums_holding_own_move_and_are_freed() {
    let src = "
enum Slot {
    Full(own i64 v);
    Vacant;
}

fn main() {
    let a = Slot.Full(own 5);
    let b = a;
    match b {
        case Full(v) -> print(*v);
        case Vacant -> print(\"vacant\");
    }
    let c = Slot.Full(own 6);
    match c {
        case Full(_) -> print(\"dropped\");
        case Vacant -> print(\"vacant\");
    }
    let d = Slot.Full(own 7);
    match d {
        case Vacant -> print(\"vacant\");
        default -> print(\"default\");
    }
    let e = Slot.Full(own 8);
}";
    assert_eq!(ok(src), "5\ndropped\ndefault\n");
    let d = err("enum Slot {\n    Full(own i64 v);\n    Vacant;\n}\nfn main() {\n    let a = Slot.Full(own 5);\n    let b = a;\n    print(a);\n}");
    assert!(d.title.contains("use after move"), "{}", d.title);
}

#[test]
fn match_ref_borrows_the_contents() {
    let src = "
enum Slot {
    Full(own i64 v, i64 tag);
    Vacant;
}

fn main() {
    let mut s = own Slot.Full(own 5, 1);
    match ref mut s {
        case Full(v, tag) -> {
            *v += 10;
            print(*v, tag);
        }
        case Vacant -> print(\"vacant\");
    }
    match ref s {
        case Full(v, _) -> print(*v);
        case Vacant -> print(\"vacant\");
    }
    match s {
        case Full(v, _) -> print(*v + 1);
        case Vacant -> print(\"vacant\");
    }
}";
    assert_eq!(ok(src), "15 1\n15\n16\n");
}

#[test]
fn a_ref_taken_in_match_ref_is_checked_after_the_owner_is_replaced() {
    let src = "
enum Slot {
    Full(own i64 v);
    Vacant;
}

fn main() {
    let mut s = own Slot.Full(own 5);
    let r = match ref s {
        case Full(v) -> Some(v);
        case Vacant -> None;
    };
    *s = Slot.Vacant;
    match r {
        case Some(v) -> print(*v);
        case None -> print(\"none\");
    }
}";
    let d = err(src);
    assert!(d.title.contains("dangling"), "{}", d.title);
    let out = d.render("t.ward", src);
    assert!(out.contains("`s の中身 v` を借りた"), "{}", out);
}

#[test]
fn match_ref_needs_own_contents() {
    let src = "struct P { i64 x; }\nenum E {\n    A(P p);\n}\nfn main() {\n    let e = own E.A(P { x: 1 });\n    match ref e {\n        case A(p) -> print(1);\n    }\n}";
    assert!(err(src).title.contains("借りて match するとき"));
}

// ---------------------------------------------------------------- Option

#[test]
fn option_replaces_null() {
    let src = "
fn Option<i64> find(i64 n) {
    for i in range(10) {
        if i * i == n { return Some(i); }
    }
    None
}

fn main() {
    for n in range(8, 10) {
        match find(n) {
            case Some(i) -> print(n, \"の平方根は\", i);
            case None -> print(n, \"は平方数ではない\");
        }
    }
    let Option<i64> x = None;
    let y = Some(3);
    print(x, y);
    let z = if true { Some(\"a\") } else { None };
    print(z);
}";
    assert_eq!(ok(src), "8 は平方数ではない\n9 の平方根は 3\nNone Some(3)\nSome(a)\n");
}

#[test]
fn option_errors() {
    let e = |body: &str| err(&format!("fn main() {{\n{}\n}}", body));
    let d = e("let x = None;");
    assert!(d.title.contains("None の型が決まりません"), "{}", d.title);
    assert!(d.note.unwrap().contains("Option<i64>"));
    assert!(e("let Option<i64> x = Some(\"a\");").title.contains("型が合いません"));
    assert!(e("let Option<i64> x = None;\nprint(x + 1);").title.contains("型が合わない演算"));
    assert!(e("let Option<i64> x = None;\nmatch x {\n case Some(v) -> print(v);\n}").title.contains("None の場合が書かれていません"));
    assert!(e("let Option x = None;").title.contains("型の引数が必要"));
    assert!(e("let Vec<i64> x = None;").title.contains("未定義の型"));
}

#[test]
fn option_of_own_moves_and_frees() {
    let src = "
fn Option<own i64> make(bool b) {
    if b { Some(own 42) } else { None }
}

fn main() {
    let a = make(true);
    let b = make(false);
    match a {
        case Some(p) -> print(*p);
        case None -> print(\"none\");
    }
    print(b);
    let c = make(true);
}";
    assert_eq!(ok(src), "42\nNone\n");
}

#[test]
fn integer_payloads_follow_the_declared_type() {
    let src = "
enum Px {
    Gray(u8 v);
}

fn main() {
    let p = Px.Gray(250);
    match p {
        case Gray(v) -> print(v + 10);
    }
    let Option<u8> o = Some(255);
    match o {
        case Some(v) -> print(v + 1);
        case None -> print(0);
    }
}";
    assert_eq!(ok(src), "4\n0\n");
}
