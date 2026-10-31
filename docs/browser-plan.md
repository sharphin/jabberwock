# 軽量ブラウザの試作: 実現可能性と構成案

状態: 方針を決めるための案（2026-10-30）。まだ何も実装していない。
標準ライブラリの状況は 2026-10-30 時点（jwc のみ、ブランチ safety-ci-resume）。

## 結論（2026-10-30 に決定）

- エンジンは **WebView2**（Windows に入っている Edge のエンジン）で始める。Rust 製（Servo / Blitz）も比べたが、
  Servo は崩れるサイトが残り、Blitz は JavaScript が動かないため見送った。
- ただし **エンジンを差し替えやすくする**のが条件。ブラウザ本体（jabberwock）とエンジンの間に
  C の関数だけの境目 `browser/engine/jw_engine.h` を置き、本体はこれ以外に触らない。
  WebView2・試験用の偽エンジン・あとで Servo や jabberwock 製のエンジンは、それぞれこの関数を 1 組実装するだけ。
- 今の jabberwock には C の部品を呼ぶ手段がないので、そこを足す必要がある。標準ライブラリ側の方針（2026-10-30）:
  任意の extern は入れず、`std.browser` の資源型 `Engine`（世代つきの札。ムーブだけ・送れない）と std の中だけで呼べる内部の操作を足し、
  実行時ライブラリから jw_engine.h の関数を呼ぶ。イベントは C 側で jabberwock の構造体か enum に写す。リンクの指定は alice-native の能力選択で行う。着手時期は未定。
- アカウントの概念は「作らない」で消す。境目にもログイン・同期・プロファイル・パスワード保存の関数を置かない。

## 差し替えやすくするための決まり（jw_engine.h）

| 決まり | 理由 |
|---|---|
| C の関数だけ。COM・Rust の型・HWND は出さない | どの言語で書いたエンジンでも実装できる。jabberwock から呼ぶのも C の関数だけで済む |
| 呼び返しはしない。イベントを溜めて `jwe_poll` / `jwe_wait` で取りに行く | jabberwock の安全の決まり（関数の値を外へ渡さない）と合う。標準ライブラリの GUI 設計とも同じ形 |
| 文字列は UTF-8 と長さ。イベントの文字列は次の poll まで有効 | エンジンごとの文字の形（WebView2 は UTF-16）を中に閉じこめる |
| エンジンができることは `jwe_info` の `caps` で知らせる | JS が動かないエンジン（Blitz・自前）や外枠を HTML で出せないエンジンにも差し替えられる |
| タブは番号だけで指す。閉じた番号は使い回さない | 古い番号で別のタブを触る取り違えを防ぐ（jabberwock の世代つき ref と同じ考え方） |
| 偽エンジン（mock）でテストする | 画面なしでどの OS でも本体の動きを確かめられる。新しいエンジンも同じテストに通す |

## 構成

```
jabberwock のプログラム（browser.jw）
  ├─ タブ・URL の補完・履歴・ブックマーク・設定 … ここを jabberwock で書く
  ├─ 外枠（アドレスバー・タブ列）… 小さな HTML。エンジンで表示し、操作は CHROME_MESSAGE で届く
  └─ jw_engine.h（境目）
        ├─ webview2/jwe_webview2.cpp（WebView2。WebView2Loader.dll を実行時に読む）
        ├─ mock/jwe_mock.c（試験用）
        └─ （あとで）Servo / Blitz / jabberwock 製
```

- インタプリタでは動かさず、`jabberwock build` で作った実行ファイルだけで動く想定。
- 外枠を HTML にすると、標準ライブラリの GUI（まだ未実装）を待たずに作り始められる。
  外枠を HTML で出せないエンジンに替えるときは、GUI ができてから外枠をそちらで描く。

### アカウントを持たない、の具体的な中身

| 項目 | 扱い |
|---|---|
| ブラウザへのログイン・同期・プロファイル | 画面も保存先も作らない |
| パスワード保存・自動入力 | WebView2 自身の保存・自動入力は切ったまま。代わりに自前の金庫（下の「パスワードの金庫」）を別のプログラムとして置く（2026-10-31 に方針を変えた） |
| 保存場所 | `%LOCALAPPDATA%\jabberwock-browser\` の 1 か所だけ。ブックマーク・履歴は読めるテキストファイル |
| サイトの Cookie（サイトへのログイン） | 「進め方」の決めていないことを参照 |

## パスワードの金庫（2026-10-31）

まつさんの指示: 「パスワード本体は web から侵入できないようスタンドアロンで管理」。ページへの入れ方は「ボタンで入力」と「コピー」の両方（決定カードで選んだ）。

| 決まり | 中身 |
|---|---|
| 別のプログラム | `jwbrowser.exe --vault` が別の窓で動き、jabberwock の本体 `jwvault-app.exe`（browser/app/vault_main.jw）を子プロセスにする。ブラウザ本体（jwbrowser-app）はパスワードを一切持たない |
| ページとは別の WebView | 金庫の画面（vault.html）はタブを作らない WebView に出し、WebView2 の置き場所も分ける（`<データ>\vault\webview2`）。CSP で外への読み込みを全部止める |
| 暗号 | マスターパスワード → PBKDF2-HMAC-SHA256（60 万回）→ 32 バイトの鍵 → ChaCha20-Poly1305。すべて jabberwock で書き（crypto.jw）、RFC / NIST の値で確かめる。ファイルの 1・2 行目（回数・塩）も印に入れる |
| 乱数 | jabberwock にまだないので、つなぎが OS から取る（Windows は BCryptGenRandom）。塩 16 バイト・nonce 12 バイト（保存のたびに新しく） |
| 画面に出さない | パスワードは「見る」「編集」を押したときだけ 1 回画面に送る |
| 入力 | 利用者が「入力」を押したときだけ。金庫 → ブラウザは Windows のメッセージ（WM_COPYDATA）。ブラウザは見えているタブの出どころ（scheme://host[:port]）が保存したサイトと完全に同じときだけ `jwe_fill_login` で入れ、ページの中でも入れる直前に `location.origin` を確かめる |
| コピー | クリップボードの履歴・クラウドには残さない印を付け、30 秒たって変わっていなければ消す |
| 自動で閉じる | 5 分触らなければ閉じる（鍵と中身を手放す） |

まだしていないこと: マスターパスワードの文字列をメモリから消すこと（jabberwock の文字列は書き換えられない）、ページで打ったパスワードを保存するかたずねること（ページ → 金庫の道を作らないため、今はしない）、iframe の中のログイン欄。

## jabberwock 側に足りないもの（標準ライブラリのスレッドへ渡す一覧）

第 1 段階（WebView2 で動かす）に必要:

| 種類 | 足りないもの | 今あるもの・設計との関係 |
|---|---|---|
| 言語・ビルド | C の部品をつなぐ手段（`jw_engine.h` の関数を呼び、エンジンの .o と DLL をリンク） | 設計の alice-native「能力」の仕組みに乗せる。未実装 |
| イベント | `poll_events` / `wait_events(期限)` の形 | GUI 設計に同じ形がある。未実装 |
| 時刻 | 今の時刻（履歴の日時）・待つ | std.time（単調時計・Duration・Deadline）で入れる予定。履歴には壁時計の時刻も要る |
| ファイル | 安全な上書き（一時ファイル → rename） | std.fs に exists・list_dir・make_dir・remove は実装済み（jwc、safety-ci-resume）。安全な上書きはまだ |
| 環境 | 環境変数（`LOCALAPPDATA`）・アプリ用フォルダの場所 | ない。設計になく、入れるか未定 |
| 乱数 | OS の乱数（金庫の塩・nonce・パスワード作り） | ない。今はつなぎ（jwbrowser.c）から 1 行でもらう |
| データ形式 | JSON の読み書き（外枠の HTML とのやりとり・保存ファイル） | ない。設計にもない（推測: 追加が要る） |
| 文字列 | URL の分解・%エンコード、検索語か URL かの判定 | URL の分解は std.http の中にあるが非公開。%エンコードはない |

書き直し（自前エンジン）の段階で必要:

| 種類 | 足りないもの | 設計との関係 |
|---|---|---|
| 通信 | HTTPS・リダイレクト・Windows の TCP | std.http（HTTP/1.1、http:// の get/post、chunked）と std.net の TCP は Linux・macOS で実装済み。TLS と Windows はまだ |
| 圧縮 | deflate/gzip の展開 | ない |
| 描画 | 2D 描画・文字の配置（DirectWrite） | 設計済み・未実装 |
| 画像 | PNG/JPEG の展開 | WIC で設計済み・未実装 |
| 解析 | HTML の字句・木構造、CSS の解析・カスケード、レイアウト | jabberwock で書く本体 |
| JavaScript | なし（入れない前提を勧める） | — |

## 進め方

1. ✓ 境目 `jw_engine.h`、偽エンジンとそのテスト、WebView2 版。2026-10-31 に Windows で起動し、example.com の読み込みとイベント（ready・nav_started・url・history・title・nav_finished）を確認した。
2. ✓ WebView2 版を Windows で動かして確かめる（smoke。WebView2Loader.dll なしで起動）。
3. jabberwock から境目の関数を呼べるようにする（標準ライブラリ側で std.browser として。時期は相談）。
   それまでのつなぎとして、browser/host/jwbrowser.c がエンジンを動かし、本体を子プロセスにして標準入出力の 1 行ずつでつなぐ（2026-10-31）。
   これでタブ型の本体が WebView2 で動く（jwbrowser.exe + jwbrowser-app.exe）。
   できたら browser/app/engine.jw を std.browser を呼ぶものに置き換える。
4. ✓ 本体（browser/app）: アドレス入力・検索・戻る/進む・再読み込み・タブ・キー操作・履歴・ブックマーク。
   今は jabberwock で書いた偽エンジンにつなぎ、台本でテストする（`cargo test --test browser`）。
5. ✓ 設定のタブ（2026-10-31）。外枠が窓全体を覆って設定の画面を出す（`jwe_chrome_height` の `JWE_CHROME_FULL`）。
   項目は browser/app/settings.jw の `items()` の一覧で、外枠はそこから画面を作る（項目を足すのに HTML は直さない）。
   今の項目: 起動したときのページ（新しいタブ / ホームページ / 前回のタブ）・ホームページ・検索エンジン・ブックマークバー・サイトのアイコン・Cookie と履歴を消す。
   サイトのアイコンは外枠が各サイトの `/favicon.ico` を読む（読めなければ頭文字。設定で切れる）。
6. 履歴の一覧ページ・ダウンロード・ズームなど。
7. （余裕があれば）別のエンジン（Servo / Blitz / jabberwock 製）を同じ境目で足す。

決めていないこと: サイトの Cookie を終了時に消すか（今の案は「1 か所に残し、ボタン 1 つで全部消せる」）。
