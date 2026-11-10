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

/// std.fs の例（tests/fs_example.jw。`DIR` を作業用のフォルダに置き換えて使う。最後にすべて消すので、何回動かしても同じ）
pub const FS: &str = include_str!("fs_example.jw");

/// テストごとのフォルダ（テストは並んで走る）
pub fn fs_src(name: &str) -> String {
    let d = std::env::temp_dir().join(format!("jw-fs-{}-{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&d);
    FS.replace("DIR", &d.to_str().unwrap().replace('\\', "/"))
}

#[test]
fn fs_module() {
    assert_eq!(ok(&fs_src("stdlib")), "a/b a/b /b b\na/b [] /\nc.tar.gz gz c.tar [] .env y.txt\nfalse\nhello world\n[a.txt, new.log, x]\n/a.txt\n/new.log\n/x/y/b.jw\n11 true false false true\ntrue false false\nfalse true\n消せません: D/x\nディレクトリを作れません: D\n消せません: D/nope\n名前を変えられません: D/nope\nパスに NUL 文字があります: a\0b\nディレクトリを読めません: D/nope\nディレクトリを読めません: D/c.txt\nfalse\nfalse\nok\n");
}

/// テスト用の小さな HTTP サーバーを別のスレッドで動かし、そのポートを返す（std.http のテスト。native も使う）
pub fn serve_http() -> u16 {
    use std::io::{Read, Write};
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for s in l.incoming() {
            let Ok(mut s) = s else { continue };
            let mut buf = Vec::new();
            let mut chunk = [0u8; 4096];
            // ヘッダーの終わりまで読み、Content-Length の分だけ本文を読む
            let head_end = loop {
                let n = s.read(&mut chunk).unwrap_or(0);
                if n == 0 { break None; }
                buf.extend_from_slice(&chunk[..n]);
                if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") { break Some(i + 4); }
            };
            let Some(head_end) = head_end else { continue };
            let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
            let len = head.lines().find_map(|l| {
                let (k, v) = l.split_once(':')?;
                if k.eq_ignore_ascii_case("content-length") { v.trim().parse::<usize>().ok() } else { None }
            }).unwrap_or(0);
            while buf.len() < head_end + len {
                let n = s.read(&mut chunk).unwrap_or(0);
                if n == 0 { break; }
                buf.extend_from_slice(&chunk[..n]);
            }
            let body = String::from_utf8_lossy(&buf[head_end..]).to_string();
            let mut words = head.split_whitespace();
            let (method, path) = (words.next().unwrap_or("").to_string(), words.next().unwrap_or("").to_string());
            let name = head.lines().find_map(|l| l.strip_prefix("X-Name: ")).unwrap_or("").to_string();
            let ctype = head.lines().find_map(|l| l.strip_prefix("Content-Type: ")).unwrap_or("").to_string();
            let (status, extra, out) = match path.as_str() {
                "/x.json" => ("200 OK", "Content-Type: application/json\r\n".to_string(), "{\"a\": 1}".to_string()),
                "/echo" => ("201 Created", format!("X-Type: {}\r\n", ctype), format!("{} {} {}", method, body, name)),
                "/moved" => ("302 Found", "Location: /x.json\r\n".to_string(), String::new()),
                _ => ("404 Not Found", String::new(), "なし".to_string()),
            };
            let reply = format!("HTTP/1.1 {}\r\n{}Content-Length: {}\r\nConnection: close\r\n\r\n{}", status, extra, out.len(), out);
            let _ = s.write_all(reply.as_bytes());
        }
    });
    port
}

/// curl があるか（std.http は curl を呼ぶ）
pub fn have_curl() -> bool {
    std::process::Command::new("curl").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

pub const HTTP_OUT: &str = "200 true {\"a\": 1} application/json true\n1.0\n404 false なし\n201 POST {\"b\": [1, 2]}  application/json\n201 PUT  ジャバ\n200 {\"a\": 1}\nURL は http:// か https:// で始めてください: ftp://x\n方法の名前がおかしいです: -x\n";

#[test]
fn http_module() {
    if !have_curl() {
        return;
    }
    let src = include_str!("http_example.jw").replace("PORT", &serve_http().to_string());
    assert_eq!(ok(&src), HTTP_OUT);
}

/// std.time（tests/time_example.jw）
#[test]
fn time_module() {
    assert_eq!(ok(include_str!("time_example.jw")), "1970-01-01T00:00:00Z 3 木\n2025-11-10T06:35:43.123Z 2025-11-10 06:35:43 月 1762756543123\n1969-12-31T23:59:59.999Z -1\n2024-02-29 2024-03-01 2023-12-31 2024-02-28T01:02:03.004Z\n2026-11-10T06:35:43.500Z 2026-11-10T06:35:43Z\n月が 1〜12 ではありません: 13\n日がその月にありません: 2026-2-30\n日時を読めません: 2026/01/01\n時刻がおかしいです: 25:0:0\n日時を読めません: 2026-01-01T00:00:00.Z\n日時を読めません: x\ntrue false 29\n9999-12-31T23:59:59.999Z 0001-01-01T00:00:00Z\ntrue\n");
}

/// std.csv と std.encoding（tests/text_formats_example.jw）
#[test]
fn csv_and_encoding_modules() {
    assert_eq!(ok(include_str!("text_formats_example.jw")), "5\n2 [name, age]\n2 [Smith, J, 42]\n2 [say \"hi\", ]\n2 [two\nlines, x]\n2 [, ]\nname,age\n\"Smith, J\",42\n\"say \"\"hi\"\"\",\n\"two\nlines\",x\n,\n\n[a: 1, b: 2]\n[a: 3, b: ]\n0 1\nCSV の `\"` が閉じていません（1 行目）\nCSV の `\"` のあとに `,` がありません（1 行目）\n[]   \n[Zg==] f 66 f\n[Zm8=] fo 666f fo\n[Zm9v] foo 666f6f foo\n[Zm9vYg==] foob 666f6f62 foob\n[Zm9vYmE=] fooba 666f6f6261 fooba\n[Zm9vYmFy] foobar 666f6f626172 foobar\n[5pel5pys6Kqe] 日本語 e697a5e69cace8aa9e 日本語\nfoob foobar\nbase64 として読めない文字があります（5 文字目）\nbase64 の長さがおかしいです\nbase64 として読めない文字があります（4 文字目）\n16 進の長さが奇数です\n16 進として読めない文字があります（1 文字目）\n16 進の中身が UTF-8 ではありません\nAP+AAQ== 00ff8001 [0, 255, 128, 1] [0, 255, 128, 1]\n");
}

/// std.sets（tests/sets_example.jw）
#[test]
fn sets_module() {
    assert_eq!(ok(include_str!("sets_example.jw")), "true true false 2 false\ntrue false [b, a]\n[a, c] 2\n[b, a, c] [a] [b] [c]\ntrue false [b] true false\n[3, 1, 2, -5] true 4\n[h, e, l, o] true false\ndup 5\ndup 3\n");
}

/// std.cli（tests/cli_example.jw）
#[test]
fn cli_module() {
    assert_eq!(ok(include_str!("cli_example.jw")), "true false true x None [a.txt, b] false
true true true file Some(12) [-v, --x] false
true true true y None [-, -5] false
Err: 知らないオプションです: --nope
Err: 知らないオプションです: -z
Err: --out に値がありません
Err: --verbose は値をとりません
false false false - None [] true
false false false - None [] true
使い方: wc [オプション] <file>
行を数える

引数:
  <file>            読むファイル

オプション:
  -v, --verbose     詳しく出す
  -o, --out <値>    出力先（既定: -）
      --count <値>  回数
  -h, --help        この説明を出す
使い方: x

オプション:
  -h, --help  この説明を出す
");
}

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
    assert!(e.contains("cli, csv, encoding, fs, http, json, lists, rand, sets, strings, time"), "{}", e);
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
