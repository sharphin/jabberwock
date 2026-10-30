# 軽量ブラウザの試作: 実現可能性と構成案

状態: 方針を決めるための案（2026-10-30）。まだ何も実装していない。
標準ライブラリの状況は 2026-10-30 時点（jwc のみ、ブランチ safety-ci-resume）。

## 結論（Rust 製エンジンを使う前提に改訂）

- **作れる。ただし Rust 製エンジンには C の API がない**ので、間に Rust で書いた小さな「橋」（C の関数として呼べるライブラリ）を 1 枚置く。
  jabberwock 側は今は C の部品を呼ぶ手段もないので、**ブラウザ用の能力（`browser`）を 1 つだけ足す**形にする
  （標準ライブラリの設計は「任意の FFI は初期版の外」なので、なんでも呼べる FFI にはしない）。
- エンジンの候補は 2 つ。**JS が動く Servo** と、**JS なしでとても軽い Blitz**。
- アカウントの概念は「作らない」で消せる。Servo / Blitz にはもともとログイン・同期・プロファイルの仕組みがない。
- 後で jabberwock でエンジンを書き直すときは、Blitz と同じ分け方（HTML 解析・CSS・レイアウト・文字・描画）で 1 つずつ置き換えていける。

## エンジンの候補（Rust 製）

| 候補 | JS | 今のサイト | 大きさ・ビルド | 状態 |
|---|---|---|---|---|
| **Servo** | 動く（SpiderMonkey。ここだけ C++） | 多くが動く。崩れるサイトもある | 大きい。初回ビルドは重い（SpiderMonkey のビルドに Python・clang が要る） | 2026-04 に crates.io で `servo` 0.1.0 を公開。組み込み用の API と長期サポート版あり |
| **Blitz**（Dioxus） | 動かない | 静的なページ・記事・ドキュメント向け | 小さい。中身は html5ever（HTML）・Stylo（Servo の CSS）・Taffy（レイアウト）・Parley（文字）・Vello（描画） | アルファ版。部品ごとに crate が分かれている |

- Servo を勧める場合: 「ふつうにサイトを見られる」ことを優先するなら。
- Blitz を勧める場合: 「ごく軽量」と「あとで jabberwock で書き直す」を優先するなら。部品の分け方がそのまま書き直しの順番になる。
- どちらでも作りは同じ（下の図）なので、橋の API を共通にしておけば途中で差し替えられる。

出典: [Servo 0.1.0 の公開](https://servo.org/blog/2026/04/13/servo-0.1.0-release/)、[Blitz](https://github.com/DioxusLabs/blitz)。
Servo の組み込み API の細かい形は実装時に確かめる。

## 構成

```
jabberwock のプログラム（browser.jw）
  ├─ タブ・URL の補完・履歴・ブックマーク・設定 … ここを jabberwock で書く
  └─ std の browser 能力（新しく足す）
        └─ 橋 jw_engine（Rust。staticlib / cdylib。C の関数だけを出す）
              ├─ ウィンドウ・入力（winit）
              └─ Servo または Blitz
```

- **橋はこのリポジトリのコンパイラとは別のパッケージ**にする（コンパイラは外部クレートを使わない決まりなので、
  `browser/engine/` に別の Cargo プロジェクトとして置き、コンパイラの依存には入れない）。
- **呼び返しはしない**。橋がイベント（読み込み完了・タイトル変更・リンクで移動など）を溜め、
  jabberwock が `poll_events()` / `wait_events(期限)` で取りに行く。GUI 設計の Window と同じ形。
- 橋の関数の例: `jw_engine_open(幅, 高さ)` / `navigate(url)` / `back()` / `forward()` / `reload()` /
  `poll_event(&ev)` / `set_chrome_text(...)` / `close()`。文字列は UTF-8 のバイト列と長さで渡す。
- アドレスバーとタブ列: 最初はキー操作（Ctrl+L で入力欄）と、エンジンで表示する小さな HTML（内部ページ）で作る。
  標準ライブラリの GUI ができたら置き換える。
- インタプリタでは動かさず、`jabberwock build` で作った実行ファイルだけで動く。
- Windows でも Linux でも動く（winit と Servo / Blitz が両方に対応しているため）。この作業環境（Linux）でも試せる。

### アカウントを持たない、の具体的な中身

| 項目 | 扱い |
|---|---|
| ブラウザへのログイン・同期・プロファイル | 画面も保存先も作らない |
| パスワード保存・自動入力 | 作らない（エンジン側にもない） |
| 保存場所 | `%LOCALAPPDATA%\jabberwock-browser\` の 1 か所だけ。ブックマーク・履歴は読めるテキストファイル |
| サイトの Cookie（サイトへのログイン） | 下の「決めたいこと」 |

## jabberwock 側に足りないもの（標準ライブラリのスレッドへ渡す一覧）

第 1 段階（Rust 製エンジンで動かす）に必要:

| 種類 | 足りないもの | 今あるもの・設計との関係 |
|---|---|---|
| 言語・ビルド | C の部品をつなぐ手段（`browser` 能力として登録し、Rust の橋をリンク） | 設計の alice-native「能力」の仕組みに乗せる。未実装 |
| イベント | `poll_events` / `wait_events(期限)` の形 | GUI 設計に同じ形がある。未実装 |
| 時刻 | 今の時刻（履歴の日時）・待つ | std.time（単調時計・Duration・Deadline）で入れる予定。履歴には壁時計の時刻も要る |
| ファイル | 安全な上書き（一時ファイル → rename） | std.fs に exists・list_dir・make_dir・remove は実装済み（jwc、safety-ci-resume）。安全な上書きはまだ |
| 環境 | 環境変数（`LOCALAPPDATA`）・アプリ用フォルダの場所 | ない。設計になく、入れるか未定 |
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

## 進め方（案）

1. 橋 jw_engine の最小形: 窓を開く・URL を開く・イベントを取る・閉じる（Rust 単体で動くことを確かめる）。
2. jabberwock の `browser` 能力: 橋をリンクして `jabberwock build` で呼べるようにする。
3. browser.jw: アドレス入力・戻る/進む・再読み込み・タブ 1 枚。
4. 履歴・ブックマーク（ファイル保存）・複数タブ。
5. （余裕があれば）Blitz の部品を 1 つずつ jabberwock で書き直す（HTML 解析 → CSS → レイアウト → 描画の順）。

注意: Linux のこの作業環境でもビルドと起動は試せる（画面は仮想ディスプレイでスクリーンショットを撮って確かめる）。
Windows での見た目と動作は、まつさんの手元で確かめてもらう必要がある。
