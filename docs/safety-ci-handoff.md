# 作業メモ・引き継ぎ: 安全性の検証

## 現在の判断と次の作業（2026-10-25追記）

作業先は [PR #7](https://github.com/sharphin/jabberwock/pull/7) の元ブランチ
`safety-ci-resume`。このメモの更新と今回のテストはこのブランチで行う。
標準ライブラリの設計は [PR #8](https://github.com/sharphin/jabberwock/pull/8) に分離してあり、
ライブラリ本体はまだ実装していない。

### コンパイラの合格範囲

- 項目1〜7は [合格基準と実行結果](safety-validation.md) に記録した実装・機能・構成について合格した。
  現在のコンパイラの基礎検証として維持し、変更時には攻撃ケース・生成入力・負例を拡充する。
- 「7項目だけでコンパイラ全体の安全性が十分」とは判定しない。
  [既存の残課題8・9](safety-testing.md#項目1〜7の検証完了と残る課題) は未解決のまま。
- **8: メモリ不足・資源枯渇時の扱い。** メモリ上限、確保失敗の扱い、時間の使い切りを別途整理する。
- **9: 巨大な配列型のサイズ制限。** ジェネリックな型の引数を通じて上限をすり抜ける経路を修正・検証する。
- この追記は作業計画の記録であり、8・9を修正済み・検証済みにするものではない。

### ライブラリ実装時に追加する検証案（10〜14は未実施）

| 項目 | 確認内容 |
|---|---|
| 10. C ABI・資源管理 | 型と配置、allocator、所有権、二重解放、古いハンドル、別context・別スレッドでの使用、終了との競合 |
| 11. 途中失敗 | 通信切断、timeout、ディスク満杯、確保失敗を注入し、資源回収と部分結果を確認 |
| 12. 外部入力・認証 | 壊れた画像・応答、不正な長さ・文字列、TLS証明書・名前検証、SQLパラメータを確認 |
| 13. 外部への副作用 | debug比較や再試行による二重書込み・二重送信を防ぎ、DBの既知の確定結果を保持 |
| 14. 配布・実環境 | DLL・ABI・version不一致の拒否、移動先起動、Windowsの実API・GPU・IME・UIAを確認 |

各項目に合格条件と対象OS・実行器・依存ライブラリの計装範囲を定める。
新しい操作を追加するたびに1〜7にも対応する攻撃を追加し、該当する10〜14を実行する。
未実施・Unsupported・skipを合格数へ含めず、模擬backendの成功と実OS・実機の成功を分ける。
既存の1〜7の合格を、新ライブラリの合格へ流用しない。

### 今回の確認

今回の変更対象はこの作業メモだけ。機能実装・コンパイラ本体は変更しない。
以下は `safety-ci-resume`、実装が `7fc6c7e` と同一の状態で実行した結果。

- `cargo test`: 成功。runner集計は351件成功・失敗0件。
  ローカルにclangがなく、clang依存の試験は早期returnするため、351件すべてを実処理の検証数とは扱わない。
- `cargo +1.99.1 clippy --all-targets`: 成功、警告0件。
- `git diff --check`、このメモのローカルリンク検査: 成功。
- native・jwc生成物・sanitizerの実行検証は今回ローカルでは未実施。
  1〜7の既存の合格根拠は `safety-validation.md` のCI結果と区別し、
  この更新に対するCIの実行状況はPR #7のChecksで確認する。

## 以下は一時停止時点の履歴（現在の合否は上記とsafety-validation.mdを参照）

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
