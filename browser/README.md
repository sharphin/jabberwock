# jabberwock の軽量ブラウザ（試作）

方針と理由は [docs/browser-plan.md](../docs/browser-plan.md)。

```
browser/
  app/main.jw                   本体の入口（引数を読んで Browser を動かす）
  app/browser.jw                タブ・アドレス欄・戻る/進む・キー操作・外枠とのやりとり
  app/address.jw                打った文字 → 開く URL（URL か検索語かの判定・%エンコード）
  app/store.jw                  履歴・ブックマークをファイルに残す（1 か所だけ）
  app/chrome.html               外枠（タブ列・アドレス欄・ブックマーク）。chrome.jw は gen_chrome.jw がここから作る
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

今は jabberwock から C の関数を呼べないので、本体は jabberwock で書いた偽エンジン（app/engine.jw）とつないである。
std.browser ができたら engine.jw だけを置き換えれば、WebView2 で動く（ほかのファイルは変えない）。

```
cd browser/app
jabberwock main.jw --ephemeral --fake testdata/basic.txt --log
```

- `--fake 台本`: 利用者の操作の代わり（`chrome go example.com`・`key 7` など。書き方は engine.jw の先頭）
- `--log`: エンジンに出した操作を表示する
- `--data フォルダ`: 履歴（history.tsv）・ブックマーク（bookmarks.tsv）の置き場所。あるフォルダを指定する（既定は今のフォルダ）
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
   copy wv2\build\native\x64\WebView2Loader.dll .
   ```
   （ARM の PC なら `x64` を `arm64` に）
3. `.\smoke.exe` で窓が開き、example.com が出る。上の欄に URL を入れて Enter で移動。Alt+← / Alt+→ で戻る・進む、F5 で再読み込み。
   PowerShell にはエンジンから届いたイベントが 1 行ずつ出る。

| 症状 | 直し方 |
|---|---|
| `jwe_open: 1` と出てすぐ終わる | `WebView2Loader.dll` が smoke.exe と同じフォルダにない。または WebView2 の実行環境がない（Windows 11 には最初から入っている。Windows 10 なら https://developer.microsoft.com/microsoft-edge/webview2/ の Evergreen Bootstrapper） |
| `EventToken.h` が見つからない | llvm-mingw が古い。新しい版を入れ直す |

試作なので、データ（Cookie など）は `smoke-data` の一時フォルダに置き、終わると消す。
