# 引き継ぎ: 安全の確かめを CI で流す仕組み（途中）

2026-10-25 にオーナーの指示で一時停止した作業。ブランチ `claude/project-thread-yo9do9` にある（main には入れていない）。
全体の考え方と項目 1〜9 は docs/safety-testing.md。

## できていること（このブランチ）

- `.github/workflows/safety.yml`: 項目 1〜7 を流す CI。push・PR は短く、毎晩（日本時間 3:00）は 20 倍の本数。
  仕事は robust / fuzz-asan / tsan / msan / jwc の 5 つ。**まだ GitHub で一度も動かしていない**（apt で入れる
  `libclang-rt-<版>-dev`・`lld`、`sysctl vm.mmap_rnd_bits=28` が runner で通るかは未確認）。
- `src/native.rs` の `instrument`: `JW_CFLAGS` の `-fsanitize=` が address / thread / memory なら、書き出した関数に
  `sanitize_address` / `sanitize_thread` / `sanitize_memory` を付ける。
- `tests/robust.rs` の `generated_code_guards_undefined_operations`（項目 4）: 書き出した LLVM の割り算・ずらしの前に守りがあるかを見る。
  llvm.rs の守りをわざと外すと 55 箇所で失敗することを確かめた。
- `runtime/jw_rt.c`（`jwc/runtime.jw` は作り直し済み）:
  - エラー・`exit`・内部エラーで終わるときは `quit`（`fflush` → `_Exit`）。片づけずに終わるので、LeakSanitizer が
    「漏れ」と報告していた（fuzz.rs をASan で流したときの 130 本の失敗はすべてこれ。エラーで止まったプログラムの辞書など）。
    atexit の処理（`llvm.global_dtors` の `@jw.count.report`）はエラー・exit のときは動かなくなる。
  - `jw_view_target_nocheck` は写しでなければ v のまま返す。llvm.rs の `place` の中（写しかどうかを見る前に呼んで select で選ぶ書き出し）が、
    own の中身の最初の 8 バイトを読んでいた。MSan が calc の例で「初期化していない値」と報告した。

## まだのこと

1. 上の jw_rt.c の変更のあとで、手元でもう一度流す（止めたので結果がない）:
   - `JW_NATIVE=1 JW_CFLAGS="-fsanitize=thread" TSAN_OPTIONS=halt_on_error=1 cargo test --no-fail-fast --test threads --test native --test system`
   - `JW_NATIVE=1 JW_CFLAGS="-fsanitize=memory -fno-omit-frame-pointer" MSAN_OPTIONS=halt_on_error=1 cargo test --no-fail-fast`
   - `JW_CFLAGS="-fsanitize=address,undefined -fno-sanitize-recover=undefined -fno-omit-frame-pointer" JW_FUZZ=300 cargo test --test fuzz`
2. TSan の「thread leak」（`blocks_freed_on_other_threads`・`compiled_programs_run_threads`・`channel_misuse_is_an_error`）。
   join も detach もしていないスレッドが終わりに残っていた。エラーで終わる道なら `_Exit` で出なくなるかもしれない。
   正常に終わる道で出るなら、task を捨てずに終わる道がある（`jw_task_drop` が呼ばれていない）ので調べる。
3. ふつうの確認: `cargo test --no-fail-fast`・`JW_NATIVE=1 cargo test --no-fail-fast`・`cargo clippy --all-targets`。
4. main に入れて、safety.yml が GitHub で通るのを見る（落ちたら runner の準備の手順を直す）。
5. CLAUDE.md の不変条件に、エラーで終わるときは `_Exit`（`quit`）であることを足す。
