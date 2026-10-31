# jwbrowser（エンジンと本体のつなぎ）

jabberwock からはまだ C の関数を呼べないので、エンジン（jw_engine.h）を動かす小さな C のプログラム `jwbrowser` が、
jabberwock で書いた本体（browser/app を `jabberwock build` したもの。名前は `jwbrowser-app`）を子プロセスとして起動し、
標準入出力の 1 行ずつでつなぐ。std.browser ができたら要らなくなる（本体は app/engine.jw だけを置き換える）。

```
jwbrowser.exe ──(イベント 1 行ずつ: 子の標準入力)──▶ jwbrowser-app.exe（jabberwock）
              ◀──(操作 1 行ずつ: 子の標準出力)──
```

## 行の形

区切りはタブ文字。文字の中の `\` タブ 改行は `\\` `\t` `\n` にする。タブの番号は本体が決めた番号（jwbrowser がエンジンの番号と対応させる）。

| 向き | 行 |
|---|---|
| エンジン → 本体 | 最初に 1 回 `info 名前 caps`（caps は jw_engine.h の JWE_CAP_* の和）。あとは `ready` / `close` / `nav_started 番号 URL` / `nav_finished 番号 code URL` / `url 番号 URL` / `title 番号 題` / `history 番号 code` / `new_tab 番号 URL` / `chrome 文字` / `key code` / `permission 番号 何か` / `error code 文字` |
| 本体 → エンジン | `tab_new 番号` / `tab_close 番号` / `tab_show 番号` / `navigate 番号 URL` / `back 番号` / `forward 番号` / `reload 番号` / `stop 番号` / `chrome_load 高さ HTML` / `chrome_post 文字` / `set_title 題` / `window op`（jw_engine.h の JWE_WINDOW_*）/ `clear_site_data` / `close` / `wait` |

- `wait` は本体が次のイベントを待ち始めた印。偽エンジンのときは、jwbrowser はこれを見てから次のイベントを渡す（毎回同じ順になる）。
- 本体が `close` を出すか、本体が終わったら、jwbrowser も終わる。窓の × は `close` イベントとして本体に渡し、閉じるかは本体が決める。
- データの置き場所は Windows では `%LOCALAPPDATA%\jabberwock-browser`（履歴・ブックマーク・WebView2 の Cookie など。ここ 1 か所だけ）。

## Windows でビルドする（llvm-mingw）

リポジトリのフォルダで。WebView2 の SDK の取り方は browser/README.md。

```
jabberwock build -o jwbrowser-app.exe browser\app\main.jw
clang -O2 -c browser\host\jwbrowser.c -o host.o
clang++ -std=c++17 -O2 -c browser\engine\webview2\jwe_webview2.cpp -I wv2\build\native\include -o webview2.o
clang++ -mwindows -o jwbrowser.exe host.o webview2.o -lole32 -luser32 -lshell32
```

`jwbrowser.exe` と `jwbrowser-app.exe` を同じフォルダに置き、`jwbrowser.exe` を起動する。

## 偽エンジンで通しで動かす（Linux など）

`cargo test --test browser host_and_app_over_pipes`（clang と cc が要る）。手で動かすなら:

```
jabberwock build -o /tmp/jwbrowser-app browser/app/main.jw
cc -o /tmp/jwbrowser browser/host/jwbrowser.c browser/engine/mock/jwe_mock.c -pthread
JWE_MOCK_SCRIPT=browser/app/testdata/basic.txt JWE_MOCK_LOG=/tmp/mock.log JWBROWSER_APP=/tmp/jwbrowser-app /tmp/jwbrowser --data /tmp
```
