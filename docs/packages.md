# パッケージ（jabberwock new / build / run / test）

cargo を小さくしたもの。今は手元のフォルダの依存先だけを扱う（ネットからの取得・版の指定はまだ）。

## 作る

```
jabberwock new app          # 実行ファイルのパッケージ（src/main.jw）
jabberwock new mathx --lib  # ライブラリ（src/lib.jw と tests/add.jw）
```

```
app/
  jabberwock.toml
  .gitignore        /target
  src/main.jw       入口（ライブラリなら src/lib.jw）
  src/*.jw          モジュール（`use geom;` で src/geom.jw）
  tests/*.jw        jabberwock test で 1 本ずつ動かす
  target/           build の出力
```

パッケージの名前はフォルダの名前（英字・数字・_ で、英字か _ から始める）。

## jabberwock.toml

```toml
[package]
name = "app"

[dependencies]
mathx = { path = "../mathx" }   # 手元のフォルダ（jabberwock.toml からの道筋）
```

- 読むのは `[package]` の `name` と、`[dependencies]` の `名前 = { path = "..." }`（`名前 = "..."` も path とみなす）だけ。ほかの節とキーは読み飛ばす。`#` から行末は注釈。
- 依存先の名前は、依存先の jabberwock.toml の `name` と同じにする。依存先は `src/lib.jw` を持つライブラリであること。
- 依存先の依存先も読む。同じ名前の依存先が 2 つの場所にあるとエラー。

## モジュールの名前

- 自分の `src/*.jw` は、ファイル名がモジュール名（`main` と `lib` は入口なので除く）。
- 依存先 `mathx` の `src/lib.jw` がモジュール `mathx`（`use mathx;` → `mathx.add(1, 2)`）。依存先の `src` のほかのファイルも同じ名前の空間に入るので、
  ライブラリの中で `use sq;` と書ける。
- すべてのモジュールが 1 つの名前の空間に入る。同じ名前のモジュールが 2 か所にあるとエラー（どれを使うか決まらないため）。

## コマンド

| コマンド | すること |
|---|---|
| `jabberwock build [--release]` | src/main.jw から target/debug/名前（`--release` は target/release）を作る。実行ファイルがソース・依存先・jabberwock.toml より新しければ作り直さない |
| `jabberwock run [-- 引数...]` | src/main.jw をインタプリタで動かす（clang がいらない） |
| `jabberwock run --native [-- 引数...]` | build してから実行ファイルを動かす（`--release` も使える） |
| `jabberwock test` | tests/*.jw を名前の順にインタプリタで動かす。エラーか 0 でない `exit` で失敗。tests/名前.out があれば、出力が同じでなければ失敗 |

どれも、今のフォルダから上へたどって jabberwock.toml を探す。`jabberwock build ファイル.jw` と `jabberwock ファイル.jw` は今までどおり（パッケージを使わない）。
エラーの位置は、パッケージの中のファイルならパッケージからの道筋で出す（`src/util.jw:3:5`）。

## まだやっていないこと

- git やネット上の置き場からの取得、版の指定と依存の解決、lockfile
- パッケージごとの名前の空間（今はすべてのモジュールが 1 つの空間）
- `jabberwock test` のテストを関数ごとに書く形（今は 1 ファイル 1 本）
- jwc（自己ホスト版）のパッケージ
