# jabberwock の軽量ブラウザ（試作）

方針と理由は [docs/browser-plan.md](../docs/browser-plan.md)。

```
browser/
  engine/jw_engine.h            エンジンとの境目（C の関数だけ）。ブラウザ本体はこれだけを使う
  engine/webview2/              WebView2（Windows）での実装
  engine/mock/                  試験用の偽エンジン（画面なし。どの OS でも動く）
  engine/tests/mock_test.c      偽エンジンが決まりどおりに動くかのテスト
  engine/tests/smoke.c          手で動かして確かめる小さなブラウザ（jabberwock なし）
```

エンジンを差し替えるときは、`jw_engine.h` の関数を実装したファイルを 1 つ足し、リンクするファイルを替えるだけ。

## 偽エンジンのテスト（Linux / macOS / Windows）

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
