//! 標準ライブラリ（`use std.名前;`。ソースはリポジトリの std/）のテスト

use jabberwock::run_with_modules;

fn ok(src: &str) -> String {
    ok_with(src, &[])
}

fn ok_with(src: &str, mods: &[(&str, &str)]) -> String {
    let o = run_with_modules(src, mods);
    if let Some(d) = &o.error {
        panic!("unexpected error:\n{}", jabberwock::render(d, "main.jw", &o.files));
    }
    assert_eq!(o.live_objects, 0, "heap objects leaked");
    o.output
}

fn err(src: &str) -> String {
    let o = run_with_modules(src, &[]);
    let d = o.error.expect("expected an error");
    jabberwock::render(&d, "main.jw", &o.files)
}

#[test]
fn strings_module() {
    let src = r#"
use std.strings;

fn main() {
    print(strings.pad_left("42", 5, "0"));
    print("[" + strings.pad_right("ab", 4, ".") + "]");
    print("[" + strings.center("ab", 7, "*") + "]");
    print("[" + strings.pad_left("abcdef", 3, " ") + "]");
    print("[" + strings.trim_start("  hi ") + "]");
    print("[" + strings.trim_end("  hi \n") + "]");
    let ws = strings.words("  a bc\tdef  ");
    let j = ws.join(",");
    print("{ws.len()} {j}");
    print(strings.rfind("abcabc", "bc").unwrap_or(-1), strings.rfind("abc", "x").is_none());
    print(strings.count("aaaa", "aa"), strings.count("abc", ""));
    print(strings.hex(255), strings.to_base(-10, 2), strings.to_base(0, 7));
    print(strings.parse_int("-ff", 16).unwrap_or(0), strings.parse_int("Z", 36).unwrap_or(0));
    print(strings.parse_int("12a", 10).is_none(), strings.parse_int("-", 10).is_none());
}
"#;
    assert_eq!(
        ok(src),
        "00042\n[ab..]\n[**ab***]\n[abcdef]\n[hi ]\n[  hi]\n3 a,bc,def\n4 true\n2 0\nff -1010 0\n-255 35\ntrue true\n"
    );
}

#[test]
fn lists_module() {
    let src = r#"
use std.lists;

fn bool big(i64 x) { x > 8 }

fn main() {
    let i64[] mut xs = [3, 1, 4, 1, 5];
    print(lists.sum(xs), lists.max(xs).unwrap_or(0), lists.min(xs).unwrap_or(0));
    let i64[] empty = [];
    print(lists.max(empty).is_none(), lists.sum(empty));
    let i64[] mut ys = [9, 2];
    lists.extend(ref mut xs, ref mut ys);
    print("{xs.len()} {ys.len()} {xs[5]} {xs[6]}");
    let part = lists.slice(xs, 1, 3);
    print(part[0], part[1], lists.slice(xs, -5, 100).len());
    print(lists.any(xs, big), lists.all(xs, (x) -> x > 0));
    let s = lists.range_of(0, 10);
    print(lists.binary_search(s, 7).unwrap_or(-1));
    match lists.binary_search(s, 70) { case Ok(i) -> print(i); case Err(i) -> print("ins {i}"); }
    let f64[] fs = [0.5, 1.25];
    print(lists.sum_f64(fs));
    // own の要素も extend で動かせる
    let (own str)[] mut a = [own "x"];
    let (own str)[] mut b = [own "y", own "z"];
    lists.extend(ref mut a, ref mut b);
    print(*a[2], b.len());
}
"#;
    assert_eq!(ok(src), "14 5 1\ntrue 0\n7 0 9 2\n1 4 7\ntrue true\n7\nins 10\n1.75\nz 0\n");
}

pub const JSON: &str = "use std.json;
fn main() {
    let text = \"\"\"{{\"name\": \"jab\\\\\"ber\", \"n\": [1, 2.5, -3e2, true, null], \"o\": {{}}, \"u\": \"\\\\u00e9\\\\ud83d\\\\ude00\", \"e\": []}}\"\"\";
    match json.parse(text) {
        case Ok(v) -> {
            print(json.stringify(v));
            print(json.pretty(v, 2));
            match ref *v {
                case Obj(f) -> print(f.len(), f.contains(\"n\"));
                default -> print(\"?\");
            }
            print(json.get_str(v, \"name\").unwrap_or(\"-\"), json.get_num(v, \"n\").is_none(), json.get_str(v, \"zz\").is_none());
        }
        case Err(e) -> print(e);
    }
    for bad in [\"\", \"[1,\", \"{{\\\"a\\\" 1}}\", \"tru\", \"\\\"abc\", \"[1] x\", \"01\", \"-\", \"\\\"\\\\x\\\"\"] {
        match json.parse(bad) {
            case Ok(v) -> print(\"ok?\", json.stringify(v));
            case Err(e) -> print(e);
        }
    }
    let own json.Value v = own json.Value.Arr(own [own json.Value.Str(\"a\\nb\"), own json.Value.Num(0.5)]);
    print(json.stringify(v));
}
";

#[test]
fn json_module() {
    assert_eq!(ok(JSON), "{\"name\":\"jab\\\"ber\",\"n\":[1,2.5,-300,true,null],\"o\":{},\"u\":\"é😀\",\"e\":[]}\n{\n  \"name\": \"jab\\\"ber\",\n  \"n\": [\n    1,\n    2.5,\n    -300,\n    true,\n    null\n  ],\n  \"o\": {},\n  \"u\": \"é😀\",\n  \"e\": []\n}\n5 true\njab\"ber true true\nJSON を読めません（1 文字目）: 値がありません\nJSON を読めません（4 文字目）: 値がありません\nJSON を読めません（6 文字目）: `:` が要ります\nJSON を読めません（4 文字目）: `true` と書くところです\nJSON を読めません（5 文字目）: 文字列が閉じていません\nJSON を読めません（5 文字目）: 値のあとに余分な文字があります\nJSON を読めません（3 文字目）: 数の先頭に 0 は書けません\nJSON を読めません（2 文字目）: 数の書き方がおかしいです\nJSON を読めません（4 文字目）: 未知のエスケープ `\\x` です\n[\"a\\nb\",0.5]\n");
}

pub const RAND: &str = "use std.rand;
fn main() {
    let mut r = rand.Rng.seeded(42);
    print(r.next(), r.next());
    let mut counts = [0, 0, 0, 0, 0, 0];
    for i in range(6000) { let k = r.below(6); counts[k] += 1; }
    print(counts);
    let mut lo = 100;
    let mut hi = -100;
    for i in range(1000) {
        let x = r.range(-5, 5);
        lo = lo.min(x);
        hi = hi.max(x);
    }
    print(lo, hi, r.range(-9223372036854775807, 9223372036854775807) != 0);
    let f = r.float();
    print(f >= 0.0 && f < 1.0, r.chance(0.0), r.chance(1.0));
    let str[] mut a = [\"a\", \"b\", \"c\", \"d\", \"e\"];
    r.shuffle(ref mut a);
    print(a, a.len());
    let mut t = rand.Rng.from_time();
    print(t.below(10) < 10);
}
";

/// 同じ種なら同じ並び（splitmix64 の最初の値は、知られている値と同じ）
#[test]
fn rand_module() {
    assert_eq!(ok(RAND), "13679457532755275413 2949826092126892291\n[985, 991, 1014, 1026, 985, 999]\n-5 4 true\ntrue false true\n[d, a, b, e, c] 5\ntrue\n");
}

/// 同じ名前のファイルがあっても、標準ライブラリはコンパイラの中のものを使う。ふつうの `use strings;` はそのファイル
#[test]
fn std_is_not_overridden_by_local_files() {
    let local = "pub fn str pad_left(str s, i64 w, str f) { \"local\" }";
    let src = "use std.strings;\nfn main() { print(strings.pad_left(\"1\", 3, \"0\")); }";
    assert_eq!(ok_with(src, &[("strings", local), ("std/strings", local)]), "001\n");
    let src = "use strings;\nfn main() { print(strings.pad_left(\"1\", 3, \"0\")); }";
    assert_eq!(ok_with(src, &[("strings", local)]), "local\n");
}

/// ふつうのモジュールの中からも使える
#[test]
fn std_from_a_module() {
    let fmt = "use std.strings;\npub fn str money(i64 c) { strings.pad_left(\"{c}\", 6, \" \") }";
    let src = "use fmt;\nuse std.strings;\nfn main() { print(\"[\" + fmt.money(42) + \"]\", strings.hex(16)); }";
    assert_eq!(ok_with(src, &[("fmt", fmt)]), "[    42] 10\n");
}

/// 別のスレッドでも同じ標準ライブラリを読み直せる
#[test]
fn std_in_threads() {
    let src = r#"
use std.strings;
fn main() {
    let t = spawn(() -> strings.pad_left("7", 3, "0"));
    print(t.join());
}
"#;
    assert_eq!(ok(src), "007\n");
}

#[test]
fn errors() {
    let e = err("use std.nope;\nfn main() {}");
    assert!(e.contains("標準ライブラリに `std.nope` はありません"), "{}", e);
    assert!(e.contains("json, lists, rand, strings"), "{}", e);
    let e = err("use std.strings;\nfn main() { print(strings.nothing(1)); }");
    assert!(e.contains("モジュール `strings` に `nothing` はありません"), "{}", e);
    // 標準ライブラリの中のエラーは std/strings.jw の位置で出す
    let e = err("use std.strings;\nfn main() { print(strings.hex(\"x\")); }");
    assert!(e.contains("main.jw:2"), "{}", e);
    let e = err("use std.strings;\nfn main() { let strings = 1; }");
    assert!(e.contains("strings"), "{}", e);
    let e = err("use std;\nfn main() {}");
    assert!(e.contains("モジュール `std` が見つかりません"), "{}", e);
    let e = err("use std.strings;\nuse std.strings;\nfn main() {}");
    assert!(e.contains("二回"), "{}", e);
}
