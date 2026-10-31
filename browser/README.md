# jabberwock の軽量ブラウザ（試作）

方針と理由は [docs/browser-plan.md](../docs/browser-plan.md)。

```
browser/
  host/jwbrowser.c              エンジンを動かし、本体（jwbrowser-app）を子プロセスにして 1 行ずつつなぐ（説明は host/README.md）
  app/main.jw                   本体の入口（引数を読んで Browser を動かす）
  app/browser.jw                タブ・アドレス欄・戻る/進む・キー操作・外枠とのやりとり
  app/address.jw                打った文字 → 開く URL（URL か検索語かの判定・%エンコード）
  app/store.jw                  履歴・ブックマーク・前回のタブをファイルに残す（1 か所だけ）
  app/settings.jw               設定の項目の一覧と値（settings.tsv）。項目は items() に 1 行足すだけで増やせる
  app/chrome.html               外枠（タブ列・アドレス欄・ブックマーク・設定の画面）。chrome.jw は gen_chrome.jw がここから作る
  app/engine.jw                 エンジンとの境目の jabberwock 側。今は jabberwock で書いた偽エンジン
  app/test_*.jw, app/testdata/  テスト（cargo test --test browser）
  engine/jw_engine.h            エンジンとの境目（C の関数だけ）。ブラウザ本体はこれだけを使う
  engine/webview2/              WebView2（Windows）での実装
  engine/mock/                  試験用の偽エンジン（画面なし。どの OS でも動く）
  engine/tests/mock_test.c      偽エンジンが決まりどおりに動くかのテスト
  engine/tests/smoke.c          手で動かして確かめる小さなブラウザ（jabberwock なし）
```

エンジンを差し替えるときは、`jw_engine.h` の関数を実装したファイルを 1 つ足し、リンクするファイルを替えるだけ。

## 本体を偽エンジンで動かす（どの OS でも）

今は jabberwock から C の関数を呼べないので、本体の app/engine.jw は 2 通りに動く。
ふつうは標準入出力の向こうのエンジン（host/jwbrowser が起動する。WebView2 で動くのはこちら）、`--fake` なら jabberwock で書いた偽エンジン。
std.browser ができたら engine.jw だけを置き換える（ほかのファイルは変えない）。

```
cd browser/app
jabberwock main.jw --ephemeral --fake testdata/basic.txt --log
```

- `--fake 台本`: 利用者の操作の代わり（`chrome go example.com`・`key 7` など。書き方は engine.jw の先頭）
- `--log`: エンジンに出した操作を表示する
- `--data フォルダ`: 履歴（history.tsv）・ブックマーク（bookmarks.tsv）・設定（settings.tsv）・前回のタブ（session.tsv）の置き場所。あるフォルダを指定する（既定は今のフォルダ）
- `--ephemeral`: 何も残さない

`jabberwock build main.jw` で実行ファイルにしても同じ動き。テストは `cargo test --test browser`
（出力を直したときは `JW_BLESS=1 cargo test --test browser` で testdata/*.out を作り直す）。
外枠を直したら `jabberwock gen_chrome.jw .`（browser/app で）で chrome.jw を作り直す。

## 偽エンジンのテスト（C。Linux / macOS / Windows）

```
cc -Wall -o mock_test browser/engine/tests/mock_test.c browser/engine/mock/jwe_mock.c
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
