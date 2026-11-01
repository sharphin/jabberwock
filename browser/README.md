# jabberwock の軽量ブラウザ（試作）

方針と理由は [docs/browser-plan.md](../docs/browser-plan.md)。

```
browser/
  app/main.jw                   ブラウザの入口（引数を読んで Browser を動かす）
  app/browser.jw                タブ・アドレス欄・戻る/進む・キー操作・外枠とのやりとり。エンジンとは std.browser だけで話す
  app/address.jw                打った文字 → 開く URL（URL か検索語かの判定・%エンコード）
  app/store.jw                  履歴（見た時刻つき）・ブックマーク・前回のタブをファイルに残す（1 か所だけ）
  app/settings.jw               設定の項目の一覧と値（settings.tsv）。項目は items() に 1 行足すだけで増やせる
  app/chrome.html               外枠（タブ列・アドレス欄・ブックマーク・設定と履歴の画面）。chrome.jw は gen_chrome.jw がここから作る
  app/vault_main.jw             パスワードの金庫（別のプログラム。ブラウザの 🔑 で、ブラウザが子プロセスとして起こす）
  app/vault.jw                  金庫のファイル vault.dat（作る・開ける・足す・消す・マスターパスワードの変更・パスワード作り）。暗号は std.hash・std.crypto
  app/vault_engine.jw           金庫の窓（std.browser）・ブラウザとの 1 行ずつのやりとり・クリップボード・自動で閉じる。テスト用の偽エンジンも
  app/vault_link.jw             ブラウザの側の、金庫とのやりとり（std.process で起こし、パイプの 1 行ずつで話す）
  app/vault.html                金庫の画面。vault_ui.jw は gen_chrome.jw がここから作る
  app/test_*.jw, app/testdata/  テスト（cargo test --test browser）
  engine/webview2/              WebView2（Windows）での実装（jw_engine.h の関数）
  engine/tests/mock_test.c      偽エンジンが決まりどおりに動くかのテスト
  engine/tests/smoke.c          手で動かして確かめる小さなブラウザ（jabberwock なし）
  build-windows.sh              Windows 版を Linux から作る
```

エンジンとの境目（C の関数だけ）は `runtime/engine/jw_engine.h`、試験用の偽エンジンは `runtime/engine/mock/jwe_mock.c`、
出どころの規則は `runtime/engine/site.h`（どれも std.browser と共通）。エンジンを差し替えるときは、
`jw_engine.h` の関数を実装したファイルを 1 つ足し、`jwc build --link` で渡すファイルを替えるだけ。

## 偽エンジンで動かす（どの OS でも）

ブラウザは jwc で実行ファイルにする（std.browser は jwc だけにある）。偽エンジンは環境変数の台本で操作を受け、呼ばれた操作を記録する。

```
jwc build --link runtime/engine/mock/jwe_mock.c -o /tmp/jb browser/app/main.jw
JWE_MOCK_SCRIPT=browser/app/testdata/basic.txt JWE_MOCK_LOG=/tmp/mock.log /tmp/jb --data /tmp/jbdata --ephemeral
```

- 台本の書き方は runtime/engine/mock/jwe_mock.c の先頭（`chrome go example.com`・`key 7` など）
- `--data フォルダ`: 履歴（history.tsv）・ブックマーク（bookmarks.tsv）・設定（settings.tsv）・前回のタブ（session.tsv）・Cookie の置き場所
  （既定は Windows なら `%LOCALAPPDATA%\jabberwock-browser`、ほかは `./browser-data`）
- `--ephemeral`: 何も残さない
- 環境変数 `JW_BROWSER_CLOCK`（試験用）: 履歴の時刻を決まった時計にする（unix のミリ秒から始めて 1 件ごとに 1 分進む）

金庫は jabberwock で書いた偽エンジンも持つ: `jwc build --link runtime/engine/mock/jwe_mock.c -o /tmp/jv browser/app/vault_main.jw` のあと
`/tmp/jv --data /tmp/v --iterations 2 --fake browser/app/testdata/vault.txt --log`（`--iterations` は鍵を作る回数。テストを速くするためだけ）。

テストは `cargo test --test browser`（jwc を作るので数分かかる。作ってある jwc を使うなら `JW_SAFETY_JWC=jwc の場所`）。
出力を直したときは `JW_BLESS=1 cargo test --test browser` で testdata/*.out を作り直す。
外枠を直したら gen_chrome.jw を jwc で作って `gen_chrome browser/app` を動かし、chrome.jw / vault_ui.jw を作り直す。

## Windows 版を作る

Linux から: `WV2_EXTRA_INC="-I <足りないヘッダー>" browser/build-windows.sh <jwc> <WebView2.h のフォルダ> <出力フォルダ>`。
jabberwock-browser.exe と jwvault-app.exe ができる。2 つを同じフォルダに置き、jabberwock-browser.exe を起動する。

## 偽エンジンのテスト（C。Linux / macOS / Windows）

```
cc -Wall -o mock_test browser/engine/tests/mock_test.c runtime/engine/mock/jwe_mock.c
./mock_test                      # ok と出れば合格
```

## WebView2 版を Windows で動かす（smoke）

clang は docs/native.md の「Windows で clang を入れる」の A（llvm-mingw）を使う想定。PowerShell で、リポジトリのフォルダから:

1. WebView2 の SDK を取ってくる（zip を展開するだけ）
   ```
   Invoke-WebRequest https://www.nuget.org/api/v2/package/Microsoft.Web.WebView2 -OutFile wv2.zip
   Expand-Archive wv2.zip wv2
   ```
2. ビルド
   ```
   clang++ -std=c++17 -O2 -c browser\engine\webview2\jwe_webview2.cpp -I wv2\build\native\include -o jwe_webview2.o
   clang -O2 -c browser\engine\tests\smoke.c -o smoke.o
   clang++ -o smoke.exe smoke.o jwe_webview2.o -lole32 -luser32 -lshell32
   ```
   WebView2Loader.dll は要らない（なければ、入っている WebView2 の実行環境を直接探して使う。横に置けばそちらを使う）。
   （ARM の PC なら `x64` を `arm64` に）
3. `.\smoke.exe` で窓が開き、example.com が出る。上の欄に URL を入れて Enter で移動。Alt+← / Alt+→ で戻る・進む、F5 で再読み込み。
   PowerShell にはエンジンから届いたイベントが 1 行ずつ出る。

| 症状 | 直し方 |
|---|---|
| 「WebView2 runtime not found」と出る | WebView2 の実行環境がない（Windows 11 には最初から入っている。Windows 10 なら https://developer.microsoft.com/microsoft-edge/webview2/ の Evergreen Bootstrapper） |
| `EventToken.h` が見つからない | llvm-mingw が古い。新しい版を入れ直す |
| `vcruntime.h` / `excpt.h` が見つからない | 公式の LLVM の clang で、Visual Studio の C++ 部品がない。llvm-mingw を使うか、Build Tools の「C++ によるデスクトップ開発」を入れる |

試作なので、データ（Cookie など）は `smoke-data` の一時フォルダに置き、終わると消す。
