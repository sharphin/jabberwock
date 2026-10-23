//! match（Java の switch の矢印形式）のテスト

use ward::{run_source, run_source_with};

fn out(body: &str) -> String {
    let src = format!("fn main() {{\n{}\n}}", body);
    let o = run_source(&src, false);
    if let Some(d) = &o.error {
        panic!("unexpected error:\n{}", d.render("test.ward", &src));
    }
    assert_eq!(o.live_objects, 0, "heap objects leaked");
    assert_eq!(o.output, run_source_with(&src, false, false).output);
    o.output
}

fn err_in_main(body: &str) -> ward::diag::Diagnostic {
    run_source(&format!("fn main() {{\n{}\n}}", body), false).error.expect("expected an error")
}

#[test]
fn match_as_a_statement() {
    let o = out("
    for n in range(5) {
        match n {
            case 0 -> print(\"zero\");
            case 1, 2 -> print(\"small\");
            case 3 -> {
                let d = n * 2;
                print(\"double\", d)
            }
        }
    }");
    // 4 はどれにも当たらず、default もないので何もしない
    assert_eq!(o, "zero\nsmall\nsmall\ndouble 6\n");
}

#[test]
fn match_as_a_value() {
    let o = out("
    for n in range(-1, 4) {
        let name = match n {
            case -1 -> \"minus\";
            case 0 -> \"zero\";
            case 1, 2 -> \"small\";
            default -> {
                let s = \"many\";
                s + \"!\"
            }
        };
        print(name)
    }");
    assert_eq!(o, "minus\nzero\nsmall\nsmall\nmany!\n");
}

#[test]
fn match_on_strings_and_bools() {
    let o = out("
    let cmd = \"stop\";
    match cmd {
        case \"go\" -> print(\"進む\");
        case \"stop\" -> print(\"止まる\");
        default -> print(\"?\");
    }
    let b = 3 > 2;
    let s = match b {
        case true -> \"はい\";
        case false -> \"いいえ\";
    };
    print(s);");
    assert_eq!(o, "止まる\nはい\n");
}

#[test]
fn statement_match_passes_break_and_return_through() {
    let src = "
fn i64 first_even(i64 limit) {
    for i in range(1, limit) {
        match i % 2 {
            case 0 -> return i;
            default -> continue;
        }
    }
    return -1;
}

fn main() {
    print(first_even(10));
    let mut i = 0;
    loop {
        i = i + 1;
        match i {
            case 3 -> break;
            default -> print(i);
        }
    }
}";
    let o = run_source(src, false);
    assert!(o.error.is_none());
    assert_eq!(o.output, "2\n1\n2\n");
}

#[test]
fn value_match_needs_default() {
    let d = err_in_main("let x = match 1 {\n case 1 -> 2;\n};");
    assert!(d.title.contains("default が必要"), "{}", d.title);
}

#[test]
fn no_return_inside_a_value_match() {
    let d = run_source("fn f() {\n let x = match 1 {\n case 1 -> return 2;\n default -> 3;\n };\n}\nfn main() {\n}", false)
        .error
        .unwrap();
    assert!(d.title.contains("return できません"), "{}", d.title);
}

#[test]
fn duplicate_cases_and_defaults() {
    let d = err_in_main("match 1 {\n case 1 -> print(1);\n case 2, 1 -> print(2);\n}");
    assert!(d.title.contains("二回"), "{}", d.title);
    assert_eq!(d.labels.len(), 2);
    let d = err_in_main("match 1 {\n default -> print(1);\n default -> print(2);\n}");
    assert!(d.title.contains("default が二回"), "{}", d.title);
}

#[test]
fn case_type_mismatch_and_unsupported_values() {
    let d = err_in_main("match 1 {\n case \"a\" -> print(1);\n}");
    assert!(d.title.contains("型が合いません"), "{}", d.title);
    let d = err_in_main("let p = own 1;\nmatch p {\n default -> print(1);\n}");
    assert!(d.title.contains("match できるのは"), "{}", d.title);
    // 名前は enum の種類として読む。変数の値とは比べない
    let d = err_in_main("let x = 1;\nmatch 1 {\n case x -> print(1);\n}");
    assert!(d.title.contains("enum ではない値"), "{}", d.title);
    let d = err_in_main("match 1 {\n case (1) -> print(1);\n}");
    assert!(d.title.contains("case に書けるのは"), "{}", d.title);
}

#[test]
fn drop_in_one_case_keeps_the_check() {
    let src = "fn main() {
    let a = own 1;
    let r = ref a;
    match 2 {
        case 1 -> print(0);
        case 2 -> drop(a);
    }
    print(*r)
}";
    let d = run_source(src, false).error.expect("dangling");
    assert!(d.title.contains("dangling"), "{}", d.title);
}
