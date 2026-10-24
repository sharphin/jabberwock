//! LLVM に書き出して clang で作った実行ファイル。clang がなければ何もしない。
//! テスト全体を実行ファイルでも突き合わせるには `JW_NATIVE=1 cargo test`（`lib.rs` の `native_mismatch`）

use std::process::Command;

use jabberwock::{llvm_build, native, no_modules};

fn have_clang() -> bool {
    Command::new(native::clang()).arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

/// 実行ファイルを作って走らせる（終了コード, 標準出力, 標準エラー）
fn run(name: &str, src: &str) -> (Option<i32>, String, String) {
    run_with(name, src, false)
}

fn run_with(name: &str, src: &str, release: bool) -> (Option<i32>, String, String) {
    let ll = llvm_build(src, &no_modules, "main.jw", release).expect("lowers to LLVM");
    // テストは並んで走るので、フォルダはテストごとに分ける（Windows は使用中のファイルを書き換えられない）
    let dir = std::env::temp_dir().join(format!("jw-native-test-{}-{}", std::process::id(), name));
    let exe = native::compile(&ll, &dir, name, "-O1").expect("clang compiles");
    let out = Command::new(&exe).env("JW_LEAKS", "1").output().expect("runs");
    (out.status.code(), String::from_utf8_lossy(&out.stdout).to_string(), String::from_utf8_lossy(&out.stderr).to_string())
}

#[test]
fn compiled_programs_print_the_same() {
    if !have_clang() {
        return;
    }
    let src = "struct P { i64 x; own i64 y; }\n\
               fn i64 fib(i64 n) { if n < 2 { return n; } fib(n - 1) + fib(n - 2) }\n\
               fn main() {\n    let p = P { x: 1, y: own 2 };\n    let r = ref p;\n    let xs = [1.5, 0.1, 100000000000000000000.0];\n    \
               print(fib(20), r.x, *r.y, xs, \"{p}\");\n}";
    let (code, out, err) = run("print", src);
    assert_eq!(code, Some(0), "{}", err);
    assert_eq!(out, "6765 1 2 [1.5, 0.1, 1e20] P { x: 1, y: 2 }\n");
    assert!(err.contains("[live] 0"), "{}", err);
}

#[test]
fn compiled_programs_catch_dangling_refs() {
    if !have_clang() {
        return;
    }
    let src = "fn main() {\n    let a = own 5;\n    let r = ref a;\n    drop(a);\n    print(*r);\n}";
    let (code, out, err) = run("dangling", src);
    assert_eq!(code, Some(1));
    assert_eq!(out, "");
    // インタプリタと同じ、時系列のエラー表示
    let want = "error: 無効になった参照を使いました (dangling reference)
 --> main.jw:5:11
  |
3 |     let r = ref a;
  |             ----- ここで `a` を借りた
4 |     drop(a);
  |     ------- ここで解放された: `a` が drop で明示的に解放された
5 |     print(*r);
  |           ^^ ここで使ったが、借りた先はもう存在しない
  |
  = note: 参照は、借りた元の所有者より長くは生きられません
";
    assert_eq!(err, want);
    // --release では使った位置だけ
    let (code, _, err) = run_with("dangling_release", src, true);
    assert_eq!(code, Some(1));
    assert_eq!(
        err,
        "error: 無効になった参照を使いました (dangling reference)\n --> main.jw:5:11\n  = ここで使ったが、借りた先はもう存在しない\n"
    );
}

#[test]
fn compiled_programs_run_threads() {
    if !have_clang() {
        return;
    }
    let src = "fn i64 fib(i64 n) { if n < 2 { return n; } fib(n - 1) + fib(n - 2) }\n\
               fn main() {\n    let chan<str> c = chan();\n    let task<i64>[] mut ts = [];\n    \
               for i in range(4) { ts.push(spawn(() -> { c.send(\"done {i}\"); fib(20) })); }\n    \
               let mut total = 0;\n    for t in ts { total += t.join(); }\n    \
               let mut n = 0;\n    for i in range(4) { match c.recv() { case Some(m) -> { n += 1; } case None -> {} } }\n    \
               print(total, n);\n    print(c.recv());\n}";
    let (code, out, err) = run("threads", src);
    // 最後の recv は、送るスレッドがもうないのでデッドロック（main の位置で知らせる）
    assert_eq!(code, Some(1), "{}", err);
    assert_eq!(out, "27060 4\n");
    assert!(err.starts_with("error: すべてのスレッドが待ちのまま止まりました (deadlock)\n  --> main.jw:11:11\n"), "{}", err);
}

// ---------------------------------------------------------------- 区画の確保（実行時ライブラリ）
// 小さい区画は塊から切り出して使い回し、頭は世代だけ。解放の位置は解放した区画の中身・使い回しの輪の表・送った区画の輪の表にある。
// どれもインタプリタと同じ出力・同じエラー表示・残った区画 0 になることを、-O1 と、jabberwock build と同じ -O2（LTO）で確かめる

/// インタプリタと実行ファイルの結果が同じか（出力・エラー表示の全文・終了コード・残った区画の数）
fn same_as_interp(name: &str, src: &str) {
    if !have_clang() {
        return;
    }
    let o = jabberwock::run_source(src, false);
    let want_err = o.error.as_ref().map(|d| jabberwock::render(d, "main.jw", &o.files));
    for opt in ["-O1", "-O2"] {
        let ll = llvm_build(src, &no_modules, "main.jw", false).expect("lowers to LLVM");
        // テストは並んで走るので、フォルダはテストごと（実行時ライブラリのソースを同じ場所に書く）
        let dir = std::env::temp_dir().join(format!("jw-native-test-{}-{}", std::process::id(), name));
        let exe = native::compile(&ll, &dir, &format!("{}{}", name, opt), opt).expect("clang compiles");
        let out = Command::new(&exe).env("JW_LEAKS", "1").output().expect("runs");
        let _ = std::fs::remove_dir_all(&dir);
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        assert_eq!(stdout, o.output, "{} {}: 出力", name, opt);
        match &want_err {
            Some(e) => {
                assert_eq!(out.status.code(), Some(1), "{} {}", name, opt);
                assert_eq!(&stderr, e, "{} {}: エラー表示", name, opt);
            }
            None => {
                assert_eq!(out.status.code(), Some(0), "{} {}: {}", name, opt, stderr);
                assert_eq!(o.live_objects, 0, "{}: interp のリーク", name);
                assert!(stderr.contains("[live] 0"), "{} {}: {}", name, opt, stderr);
            }
        }
    }
}

#[test]
fn reused_blocks_keep_detecting_old_refs() {
    // 解放した区画は同じ大きさの次の own に使い回される。古い ref は世代で見つかり、「解放された」位置も正しい
    let src = "fn main() {\n    let a = own 5;\n    let r = ref a;\n    drop(a);\n    let b = own 6;\n    let c = own 7;\n    print(*b, *c);\n    print(*r);\n}";
    same_as_interp("reuse", src);
    // 何度も使い回したあと（解放の位置は使い回しの輪の表から探す）
    let src = "fn main() {\n    let a = own 1;\n    let r = ref a;\n    drop(a);\n    let mut s = 0;\n    for i in range(1000) {\n        let x = own i;\n        s += *x;\n    }\n    print(s);\n    print(*r);\n}";
    same_as_interp("reuse_many", src);
}

#[test]
fn blocks_of_mixed_sizes_are_kept_apart() {
    // 大きさの違う区画（8 バイト刻みの小さいもの・2048 バイトを超える大きいもの）を混ぜて確保・解放・使い回す
    let src = "struct Big { i64[60][5] xs; i64 tag; }\n\
               struct Pair { i64 a; own i64 b; }\n\
               fn main() {\n    let mut total = 0;\n    for round in range(50) {\n        \
               let (own i64)[] mut small = [];\n        let (own Pair)[] mut mid = [];\n        let (own Big)[] mut big = [];\n        \
               for i in range(20) {\n            small.push(own i);\n            mid.push(own Pair { a: i, b: own i * 2 });\n            \
               if i % 5 == 0 { big.push(own Big { xs: [[round; 60]; 5], tag: i }); }\n        }\n        \
               for x in small { total += *x; }\n        for p in mid { total += p.a + *p.b; }\n        for b in big { total += b.xs[4][59] + b.tag; }\n        \
               if round % 2 == 0 { small.pop(); big.pop(); }\n    }\n    print(total);\n    \
               let keep = own Big { xs: [[7; 60]; 5], tag: 1 };\n    let r = ref keep;\n    drop(keep);\n    let other = own Big { xs: [[8; 60]; 5], tag: 2 };\n    \
               print(other.tag);\n    print(r.tag);\n}";
    same_as_interp("mixed_sizes", src);
}

#[test]
fn blocks_freed_on_other_threads() {
    // 送った区画は受け取ったスレッドで解放される（そのスレッドの空き区画の鎖に入る）。送った側の古い ref は「ほかのスレッドへ送られた」
    let src = "struct Job { i64 id; own i64 load; }\n\
               fn main() {\n    let chan<own Job> jobs = chan();\n    let chan<i64> done = chan();\n    \
               let w = spawn(() -> {\n        loop {\n            match jobs.recv() {\n                \
               case Some(j) -> { let x = own *j.load + j.id; done.send(*x); }\n                case None -> break;\n            }\n        }\n    });\n    \
               for i in range(200) { jobs.send(own Job { id: i, load: own i }); }\n    jobs.close();\n    \
               let mut s = 0;\n    for i in range(200) { match done.recv() { case Some(v) -> s += v; case None -> {} } }\n    w.join();\n    print(s);\n    \
               let mut extra = 0;\n    for i in range(100) { let y = own i; extra += *y; }\n    print(extra);\n    \
               let last = own Job { id: 1, load: own 2 };\n    let r = ref last;\n    let chan<own Job> c2 = chan();\n    c2.send(last);\n    print(*r.load);\n}";
    same_as_interp("other_threads", src);
}

#[test]
fn option_of_own_is_one_pointer() {
    // Option<own T> と、同じ形の enum（中身なし・own 1 つ）はポインタ 1 つで表す。作る・取り出す・表示・解放・中身の型が決まらない None
    let src = "struct Node { i64 v; Option<own Node> next; }\n\
               enum Slot { Empty; Full(own i64 v); }\n\
               fn i64 sum(ref Option<own Node> l) {\n    let mut s = 0;\n    let mut cur = l;\n    loop {\n        \
               match cur {\n            case None -> return s;\n            case Some(n) -> { s += n.v; cur = ref n.next; }\n        }\n    }\n}\n\
               fn Option<own Node> build(i64 n) {\n    let Option<own Node> mut head = None;\n    \
               for i in range(n) { head = Some(own Node { v: i, next: head }); }\n    head\n}\n\
               fn main() {\n    let l = build(100);\n    print(sum(l));\n    \
               let Option<own i64> mut o = None;\n    print(o);\n    o = Some(own 3);\n    print(o, o.is_some());\n    \
               match o {\n        case Some(x) -> print(*x + 1);\n        case None -> print(\"none\");\n    }\n    \
               let Slot[] mut slots = [Slot.Empty, Slot.Full(own 4), Slot.Empty];\n    slots[2] = Slot.Full(own 5);\n    \
               for s in slots {\n        match s {\n            case Empty -> print(\"empty\");\n            case Full(v) -> print(*v);\n        }\n    }\n    \
               let short = build(1);\n    match short {\n        case Some(n) -> print(n.v, n.next);\n        case None -> {}\n    }\n}";
    same_as_interp("niche", src);
    // 取り出した中身を借りた ref は、Some を None で上書きすると無効になる
    let src = "fn main() {\n    let Option<own i64> mut o = Some(own 9);\n    let a = own 0;\n    let mut r = ref a;\n    \
               match ref o {\n        case Some(x) -> r = x;\n        case None -> {}\n    }\n    o = None;\n    print(*r);\n}";
    same_as_interp("niche_dangling", src);
}

#[test]
fn strings_of_many_lengths() {
    // 文字列は 16 バイト刻みの空き領域の鎖から取る（1024 バイトを超えれば malloc）。組み立て中の文字は 48 バイトを超えると広げる
    let src = "fn main() {\n    let str[] mut keep = [];\n    let mut total = 0;\n    for i in range(300) {\n        \
               let s = \"x\".repeat(i * 7) + \"{i}\";\n        let t = \"[{s}|{i}|{-i}]\";\n        total += t.len();\n        \
               if i % 3 == 0 { keep.push(t); }\n    }\n    print(total, keep.len(), keep[keep.len() - 1].len());\n    \
               let mut joined = \"\";\n    for k in keep { if k.len() < 40 { joined = joined + k; } }\n    print(joined);\n    \
               print(\"{9223372036854775807} {-9223372036854775807 - 1} {0}\");\n    let u64 big = 18446744073709551615;\n    print(\"{big}\");\n}";
    same_as_interp("strings", src);
}

#[test]
fn freed_site_is_forgotten_after_4096_reuses() {
    // 同じ区画を 4096 回より多く使い回すと「解放された」の行は出せない（docs/native.md に書いた決まり）。止まり方と残りの表示は同じ
    if !have_clang() {
        return;
    }
    let src = "fn main() {\n    let a = own 1;\n    let r = ref a;\n    drop(a);\n    for i in range(5000) {\n        let x = own i;\n    }\n    print(*r);\n}";
    let (code, out, err) = run("forgotten", src);
    assert_eq!(code, Some(1));
    assert_eq!(out, "");
    let want = "error: 無効になった参照を使いました (dangling reference)
 --> main.jw:8:11
  |
3 |     let r = ref a;
  |             ----- ここで `a` を借りた
  ...
8 |     print(*r);
  |           ^^ ここで使ったが、借りた先はもう存在しない
  |
  = note: 参照は、借りた元の所有者より長くは生きられません
";
    assert_eq!(err, want);
}

#[test]
fn own_inside_a_for_element_cannot_be_taken_out() {
    // for の要素の中の own は読むだけの写し。取り出すと、型は own なのに写しが外へ出てしまう（検査を消す解析・spawn・書き換えの穴）
    let base = "struct P { i64 x; }\nstruct S { own P b; }\n";
    let freed = "fn main() {\n    let S[] ss = [S { b: own P { x: 1 } }];\n    let mut ss2 = ss;\n    for s in ss2 {\n        \
                 let mut q = s.b;\n        ss2.pop();\n        let n = own P { x: 777 };\n        print(q.x);\n        q.x = 4242;\n        \
                 print(n.x);\n    }\n}";
    same_as_interp("view_freed", &format!("{}{}", base, freed));
    let write = "fn main() {\n    let ss = [S { b: own P { x: 1 } }];\n    for s in ss {\n        let mut q = s.b;\n        q.x = 4242;\n    }\n    \
                 print(ss[0].b.x);\n}";
    same_as_interp("view_write", &format!("{}{}", base, write));
    let send = "fn main() {\n    let ss = [S { b: own P { x: 1 } }];\n    for s in ss {\n        let q = s.b;\n        \
                let t = spawn(() -> q.x);\n        print(t.join());\n    }\n}";
    same_as_interp("view_send", &format!("{}{}", base, send));
    let closure = "fn main() {\n    let s = S { b: own P { x: 1 } };\n    let f = () -> { let q = s.b; q.x };\n    print(f());\n}";
    same_as_interp("view_closure", &format!("{}{}", base, closure));
    let array = "struct Q { (own P)[2] a; }\nfn main() {\n    let (own P)[2] arr = [own P { x: 1 }, own P { x: 2 }];\n    \
                 let qs = [Q { a: arr }];\n    for q in qs {\n        let a = q.a;\n        print(a[0].x);\n    }\n}";
    same_as_interp("view_array", &format!("{}{}", base, array));
    // 借りるのはよい
    let borrow = "fn main() {\n    let ss = [S { b: own P { x: 5 } }];\n    for s in ss {\n        let q = ref s.b;\n        print(q.x, s.b.x);\n    }\n}";
    same_as_interp("view_borrow", &format!("{}{}", base, borrow));
}
