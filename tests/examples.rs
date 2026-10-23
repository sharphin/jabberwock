use std::fs;

use ward::run_source;

fn example(name: &str) -> String {
    let path = format!("{}/examples/{}.ward", env!("CARGO_MANIFEST_DIR"), name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {}", path, e))
}

fn ok(src: &str) -> String {
    let o = run_source(src, false);
    if let Some(d) = &o.error {
        panic!("unexpected error:\n{}", d.render("test.ward", src));
    }
    assert_eq!(o.live_objects, 0, "heap objects leaked");
    o.output
}

fn err(src: &str) -> ward::diag::Diagnostic {
    run_source(src, false).error.expect("expected an error")
}

// ---------------------------------------------------------------- 正常系

#[test]
fn counter_example() {
    assert_eq!(ok(&example("counter")), "count = 3\n");
}

#[test]
fn move_example() {
    assert_eq!(ok(&example("move")), "a = 42\nconsume: 42\n");
}

#[test]
fn fib_example() {
    let out = ok(&example("fib"));
    assert!(out.ends_with("fib( 15 ) = 610\n"));
}

#[test]
fn reassigning_an_own_frees_the_old_value() {
    let src = "
fn main() {
    let mut a = own 1;
    a = own 2;
    print(*a)
}";
    assert_eq!(ok(src), "2\n");
}

#[test]
fn nested_own_is_freed_recursively() {
    let src = "
fn main() {
    let outer = own own 7;
    print(**outer)
}";
    assert_eq!(ok(src), "7\n");
}

#[test]
fn early_return_inside_loop_frees_locals() {
    let src = "
fn find() {
    let mut i = 0;
    while true {
        let tmp = own i;
        if *tmp == 3 { return i; }
        i = i + 1;
    }
}
fn main() {
    print(find())
}";
    assert_eq!(ok(src), "3\n");
}

#[test]
fn temporaries_do_not_leak() {
    let src = "
fn main() {
    print(*own 5);
    own 6;
    let x = own 1;
    drop(x)
}";
    assert_eq!(ok(src), "5\n");
}

#[test]
fn ref_of_ref_points_at_the_same_cell() {
    let src = "
fn main() {
    let mut a = own 5;
    let r = ref mut a;
    let q = ref mut r;
    *q = 9;
    print(*a)
}";
    assert_eq!(ok(src), "9\n");
}

#[test]
fn else_if_chain() {
    let src = "
fn sign(n) {
    if n < 0 {
        return -1;
    } else if n == 0 {
        return 0;
    } else {
        return 1;
    }
}
fn main() {
    print(sign(-5), sign(0), sign(7))
}";
    assert_eq!(ok(src), "-1 0 1\n");
}

// ---------------------------------------------------------------- 安全性

#[test]
fn dangling_via_return_is_detected_with_timeline() {
    let src = example("dangling_return");
    let d = err(&src);
    assert!(d.title.contains("dangling"));
    let text = d.render("t.ward", &src);
    assert!(text.contains("ここで `a` を借りた"), "{}", text);
    assert!(text.contains("ここで解放された"), "{}", text);
    assert!(text.contains("ここで使ったが"), "{}", text);
}

#[test]
fn dangling_via_scope_end_has_ordered_timeline() {
    let src = example("dangling_scope");
    let d = err(&src);
    assert!(d.title.contains("dangling"));
    let primary = d.labels.iter().find(|l| l.primary).unwrap().span.line;
    let mut others: Vec<usize> = d.labels.iter().filter(|l| !l.primary).map(|l| l.span.line).collect();
    others.sort();
    // 借りた(7行) → 解放された(8行) → 使った(9行)
    assert_eq!(others, vec![7, 8]);
    assert_eq!(primary, 9);
}

#[test]
fn stale_ref_is_still_caught_after_slot_reuse() {
    // a のスロットは b に再利用される。世代がなければ r は b の中身を読んでしまう。
    let src = "
fn main() {
    let a = own 1;
    let r = ref a;
    drop(a);
    let b = own 2;
    print(*r)
}";
    let d = err(src);
    assert!(d.title.contains("dangling"));
}

#[test]
fn use_after_move_is_detected() {
    let src = example("use_after_move");
    let o = run_source(&src, false);
    assert_eq!(o.output, "1\n"); // consume までは実行される
    let d = o.error.expect("error");
    assert!(d.title.contains("use after move"));
    assert_eq!(d.labels.len(), 2);
}

#[test]
fn moving_inside_a_loop_is_detected_on_second_iteration() {
    let src = "
fn eat(c) {
}
fn main() {
    let a = own 1;
    let mut i = 0;
    while i < 2 {
        eat(a);
        i = i + 1;
    }
}";
    let d = err(src);
    assert!(d.title.contains("use after move"));
    assert_eq!(d.labels.len(), 1);
}

#[test]
fn cannot_move_an_own_out_through_a_pointer() {
    let src = "
fn main() {
    let outer = own own 1;
    let inner = *outer;
}";
    let d = err(src);
    assert!(d.title.contains("取り出せません"));
}

// ---------------------------------------------------------------- その他

#[test]
fn runaway_recursion_is_an_error_not_a_crash() {
    let src = "
fn f(n) {
    return f(n + 1);
}
fn main() {
    f(0)
}";
    let d = err(src);
    assert!(d.title.contains("深すぎ"));
}

#[test]
fn integer_overflow_and_div_zero_are_errors() {
    assert!(err("fn main() {\n print(1 / 0)\n}").title.contains("0 で割"));
    assert!(err("fn main() {\n print(9223372036854775807 + 1)\n}")
        .title
        .contains("オーバーフロー"));
}

#[test]
fn parse_error_points_at_the_right_line() {
    let d = err("fn main() {\n    let = 5;\n}");
    let p = d.labels.iter().find(|l| l.primary).unwrap();
    assert_eq!(p.span.line, 2);
}

#[test]
fn renders_japanese_source_without_panicking() {
    let src = "fn main() {\n    let あ = own 1;\n    drop(あ);\n    print(*あ)\n}";
    let d = err(src);
    let text = d.render("t.ward", src);
    assert!(text.contains("use after move"));
}
