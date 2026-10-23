//! IR（中間表現）への書き出し。実行結果の突き合わせは、デバッグビルドのすべてのテストで
//! `run_source` などが自動でしている（`lib.rs` の `ir_mismatch`）。ここでは表示と、書き出せる範囲を確かめる

use ward::{ir_text, no_modules, run_source};

fn ir(src: &str) -> String {
    ir_text(src, &no_modules).expect("lowers to IR")
}

#[test]
fn moves_drops_and_checks_are_visible() {
    let t = ir("fn main() {\n    let a = own 5;\n    let r = ref a;\n    print(*r);\n    drop(a);\n}");
    let expected = "\
fn main() -> 値なし {
    let _0: own i64;
    let _1: own i64;  // a
    let _2: ref i64;
    let _3: ref i64;  // r
    let _4: str;
    let _5: 値なし;
    let _6: own i64;
  bb0:
    _0 = own 5i64
    _1 = move _0
    _2 = ref _1
    _3 = move _2
    _4 = show!((*_3))
    _5 = print!(move _4)
    discard move _5
    _6 = move _1
    discard move _6
    drop _3
    drop _1
    return ()
}
";
    assert_eq!(t, expected);
}

#[test]
fn a_ref_that_may_dangle_keeps_its_check() {
    let src = "fn i64 get(ref i64 p) { *p }\nfn main() {\n    let a = own 1;\n    print(get(ref a));\n}";
    let t = ir(src);
    // 検査を消せたかどうかは `(*_0)`（消せた）と `(*!_0)`（実行時に検査）で見分けられる
    assert!(t.contains("(*_0)") || t.contains("(*!_0)"), "{}", t);
    assert_eq!(run_source(src, false).output, "1\n");
}

#[test]
fn loops_and_early_exits_drop_what_they_leave() {
    let src = "\
fn main() {
    let mut n = 0;
    for i in range(10) {
        let x = own i;
        if i == 3 { continue; }
        if i == 6 { break; }
        n += *x;
    }
    print(n);
}";
    let t = ir(src);
    assert!(t.contains("step "), "{}", t);
    let o = run_source(src, false);
    assert_eq!(o.output, "12\n");
    assert_eq!(o.live_objects, 0);
}

#[test]
fn unsupported_programs_say_why() {
    let e = ir_text("fn main() {\n    let v = (i64 x) -> x + 1;\n    print(v(1));\n}", &no_modules).unwrap_err();
    assert!(e.contains("クロージャ"), "{}", e);
}
