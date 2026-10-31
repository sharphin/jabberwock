# jwvault（金庫の窓のつなぎ）

ブラウザ本体はもう std.browser でエンジンを直接使う。金庫は、クリップボード・ブラウザとのやりとりがまだ std にないので、
エンジン（runtime/engine/jw_engine.h）を動かす小さな C のプログラム `jwvault` が、jabberwock で書いた金庫の本体
（browser/app/vault_main.jw を jwc で作ったもの。名前は `jwvault-app`）を子プロセスとして起動し、標準入出力の 1 行ずつでつなぐ。
std.process（子プロセスとのパイプ）と std.clipboard ができたら要らなくなる。

```
jwvault.exe ──(イベント 1 行ずつ: 子の標準入力)──▶ jwvault-app.exe（jabberwock）
            ◀──(操作 1 行ずつ: 子の標準出力)──
```

## 行の形

区切りはタブ文字。文字の中の `\` タブ 改行は `\\` `\t` `\n` にする。

| 向き | 行 |
|---|---|
| つなぎ → 本体 | 最初に 1 回 `info 名前 caps`。あとは `ready` / `close` / `chrome 文字` / `error code 文字` / `site URL`（ブラウザの今のページ）/ `filled 結果`（0 は入れた、-1 はブラウザがない）/ `lock`（5 分触らなかった） |
| 本体 → つなぎ | `chrome_load 高さ HTML` / `chrome_post 文字` / `chrome_height 高さ`（`full` なら窓全体）/ `set_title 題` / `window op`（jw_engine.h の JWE_WINDOW_*）/ `copy 文字`（30 秒で消す）/ `fill サイト ユーザー パスワード`（ブラウザへ頼む）/ `ask_site` / `close` / `wait` |

- `wait` は本体が次のイベントを待ち始めた印。偽エンジンのときは、jwvault はこれを見てから次のイベントを渡す（毎回同じ順になる）。
- 乱数は本体が std.random で OS から取る。
- ブラウザとは Windows のメッセージ（WM_COPYDATA）で話す。ブラウザ側は engine/webview2/vault_link.c。
- データの置き場所は Windows では `%LOCALAPPDATA%\jabberwock-browser`（vault.dat。金庫の WebView2 は `vault` の下）。

## 偽エンジンで通しで動かす（Linux など）

`cargo test --test browser vault_over_pipes`（clang と cc が要る）。
