//! 文字列のメソッドと入力（`read_line()` / `args()`）のテスト

use ward::{run_source, run_source_with, run_source_with_input};

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

fn main_ok(body: &str) -> String {
    ok(&format!("fn main() {{\n{}\n}}", body))
}

fn err(body: &str) -> ward::diag::Diagnostic {
    run_source(&format!("fn main() {{\n{}\n}}", body), false).error.expect("expected an error")
}

#[test]
fn string_methods_count_characters() {
    let o = main_ok("
    let s = \"日本語テキスト\";
    print(s.len(), s.slice(0, 3), s.slice(3, 7), s.find(\"テ\"), s.find(\"x\"));
    print(s.chars());
    print(\"  a b  \".trim(), \"Hi\".to_upper(), \"Hi\".to_lower(), \"ab\".repeat(3));
    print(s.contains(\"語\"), s.starts_with(\"日本\"), s.ends_with(\"本\"));
    print(\"a-b-c\".replace(\"-\", \"+\"));");
    assert_eq!(o, "7 日本語 テキスト Some(3) None\n[日, 本, 語, テ, キ, ス, ト]\na b HI hi ababab\ntrue true false\na+b+c\n");
}

#[test]
fn split_lines_and_to_int() {
    let o = main_ok("
    let words = \"the cat  sat\".split(\" \");
    print(words, words.len());
    print(\"one\\ntwo\\n\".lines());
    print(\" 42 \".to_int(), \"-7\".to_int(), \"x\".to_int());
    let mut total = 0;
    for w in \"1,2,3\".split(\",\") {
        match w.to_int() {
            case Some(n) -> total += n;
            case None -> {}
        }
    }
    print(total);");
    assert_eq!(o, "[the, cat, , sat] 4\n[one, two]\nSome(42) Some(-7) None\n6\n");
}

#[test]
fn methods_on_borrowed_strings() {
    let o = ok("
fn i64 count(ref str s) {
    s.len()
}

fn main() {
    let s = \"abc\";
    let r = ref s;
    print(count(s), r.len(), r.to_upper());
}");
    assert_eq!(o, "3 3 ABC\n");
}

#[test]
fn string_errors() {
    assert!(err("print(\"a\".nope());").title.contains("メソッド `nope` はありません"));
    assert!(err("print(\"a\".split(1));").title.contains("型が合いません"));
    assert!(err("print(\"abc\".slice(1, 5));").title.contains("範囲の外"));
    assert!(err("print(\"abc\".split(\"\"));").title.contains("空の文字列"));
    assert!(err("print(\"abc\".len(1));").title.contains("引数"));
}

// ---------------------------------------------------------------- 入力

#[test]
fn read_line_until_the_end() {
    let src = "
fn main() {
    let mut n = 0;
    loop {
        match read_line() {
            case None -> break;
            case Some(line) -> {
                n += 1;
                print(\"{n}: {line}\");
            }
        }
    }
    print(read_line());
}";
    let o = run_source_with_input(src, "first\r\nsecond\nlast", &[]);
    assert!(o.error.is_none(), "{:?}", o.error.map(|d| d.title));
    assert_eq!(o.output, "1: first\n2: second\n3: last\nNone\n");
    assert_eq!(o.live_objects, 0);
}

#[test]
fn args_are_a_list_of_strings() {
    let src = "
fn main() {
    let a = args();
    print(a.len(), a);
}";
    let o = run_source_with_input(src, "", &["x", "日本"]);
    assert_eq!(o.output, "2 [x, 日本]\n");
    // 引数がなければ空のリスト、入力がなければ None
    assert_eq!(ok("fn main() {\n    print(args(), read_line());\n}"), "[] None\n");
}

#[test]
fn builtin_input_errors() {
    assert!(err("print(read_line(1));").title.contains("引数"));
    let d = run_source("fn args() {\n}\nfn main() {\n}", false).error.expect("error");
    assert!(d.title.contains("組み込み関数"), "{}", d.title);
}
