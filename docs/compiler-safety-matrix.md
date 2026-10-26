# コンパイラ別の実用検査と形式証明

安全性の確認対象は jwc を最優先とする。Windows と Linux それぞれで、
jwc の12項目と Rust 版の12項目を独立に実行・集計する。
さらに各コンパイラに対して最終目標5項目の追加テストを実行する。
既定では OS ごとに24項目と10目標を一つの JSON に保存する。
形式証明は別ツール・別JSONであり、実用テストから Lean を呼び出さない。

## 実行

Windows（x64の環境初期化・リンク確認を含む）:

```powershell
.\scripts\safety_windows.cmd
# jwcだけを先に確認する場合
.\scripts\safety_windows.cmd --compiler jwc
```

Linux:

```sh
python3 scripts/safety_linux.py
python3 scripts/safety_linux.py --compiler jwc
```

準備のRustビルドはjwcのブートストラップに必要。準備成功をjwc検査の合格として数えない。
jwc検査が終わってからRust版を検査する。各プローブの開始・経過時間・ログパスを表示する。
実行するテストは利用者側で行う。この変更時にエージェントによるテスト実行はしていない。

## 12項目の検査対象

以下は各コンパイラに適用する。ケース・構成・実行対象を混在集計しない。

|項目|現在の実行経路|残る確認範囲の例|
|---|---|---|
|1 検査省略|当該コンパイラのchecked/elided LLVM、仕様期待値、fuzz|全IR命令と制御フロー|
|2 型の通る攻撃|無効参照・境界・型拒否・ASan・退役負例|全公開機能の攻撃パターン|
|3 データ競合|スレッド/所有権移動/channel、Linux TSanと検出器負例|全共有経路、Windows検出器|
|4 LLVM未定義動作|数値・境界期待値、Linux UBSan|LLVM poison/undef・属性・alias（UBSanでは網羅しない）|
|5 最適化|O1/O2+LTO、検査省略あり/なし、ASan fuzz|サポートする全構成|
|6 未初期化|Linux MSanと当該コンパイラLLVMの負例|Windows検出器、全生成経路|
|7 自己ホスト|jwc stage1→2→3、LLVM一致、生成物実行。Rust側はbootstrap元の回帰|stage2/3への全検査適用|
|8 資源枯渇|runtime malloc/calloc/reallocのN回目失敗注入、Linux資源制限|コンパイラ内部・スレッド・スタック・OS資源|
|9 巨大配列型|各コンパイラの型境界・上限内/外|全入れ子型・レイアウト演算|
|10 仕様/型/副作用|直接期待値、型拒否、順序、ファイル副作用|全仕様の正負ケース対応表|
|11 所有権/回収|move/drop、list変更、途中return/?/break、live=0、失敗注入|全所有型・失敗経路・FFI|
|12 実環境|OS別、O1/O2+LTO、通常CLI、副作用、bootstrap|実機/CPU/ツールチェーン構成表|

検査の具体的な一覧は `scripts/safety_compilers.py`、期待値は
`scripts/compiler_contracts.py`、不足は `scripts/safety_goals.py` に置く。
NULL注入はruntimeの5つの単一スレッドfixtureだけを対象とし、
生成LLVMやコンパイラ自身の確保を検査した扱いにはしない。
検出器負例は当該コンパイラのLLVMに意図的な欠陥を挿入して検出を確認する。
差分fuzzではRust側を比較先に使うが、jwcのコード生成・実行を必須にする。
直接期待値テストは他コンパイラの出力を正解として使用しない。

## 最終目標5項目の追加テスト

|目標|独立に実行する追加ケース|合わせて必要な12項目の証拠|
|---|---|---|
|意味保存|評価順序と戻り値|1,5,7,9,10,12|
|メモリ安全性|所有値を反復確保・解放しlive=0|2,3,4,6,8,11|
|型安全性|戻り値型の不一致をコード生成前に拒否|2,9,10|
|所有権・参照の整合性|連続move後の参照先の保持|1,2,3,8,11|
|実行時チェックの正しさ|drop後の参照を指定エラーで拒否|1,2,4,5,8,9|

これらは5目標の代表ケースから開始した検査で、目標全体の網羅完成ではない。
各目標の直接テストと関連項目の両方が成功して初めて `PROBES_PASS_ONLY` とする。
不足が残る間は `acceptance: BLOCKED`。未実装・未実行・失敗を合格にしない。
プロセス終了0は実行したプローブの成功であり、安全性認定を意味しない。

## 形式証明：別ツール

```sh
python scripts/formal_report.py --output target/formal-report.json
```

`--abstract-model` 指定時のみ既存の `check_formal.py` で共有抽象モデルのLean証明・公理監査を実行する。
5目標×コンパイラの形式証明状況は別JSONに記録する。
現状は実装全体に対応する証明がないため `NOT_PROVEN`。
抽象モデルの成功 `MODEL_PASS` で5目標を証明済みに変更しない。
全目標の証明が未完了なのでこのツールの終了コードは1（環境エラーは2）。

## レポート

schema 3: `items` はOS・compiler・項目番号の複合ID、`goals` はOS・compiler・目標の複合ID。
ネストしたケースのsubject不一致、必要ケースIDの欠落/重複、検出器が0件実行は `INFRA_ERROR`。
古いschema 2の混合集計と互換ではない。旧レポートを全コンパイラの合格証拠として移行しない。

`jwc --ll-checked file.jw` は検査省略解析を飛ばしたLLVMを出力する追加CLI。
通常の `--ll` と `build` の挙動は変更しない。

Rust CLIの `--no-elide --ll` が以前はフラグを無視していたため修正した。
両コンパイラでcheckedとelidedを区別して生成する。CLI回帰テストも追加したが未実行。
共有C runtimeの退役検査とUB検出器負例は `evidence_scope` に共有基盤と記録する。
これだけでコンパイラ生成コードを検証したとは扱わない。

### コンパイラ別形式証明の入口

`python scripts/formal_report.py --compiler jwc` または `--compiler rust`。
既定は両方で、`formal/compilers/<compiler>/obligations.json` の5義務を独立に処理する。
対象実装のソース一覧/hash、義務manifestのhash、証明入口、定理の公理監査を記録する。
証明モジュールの名前空間は `JabberwockFormal.Jwc.*` と `JabberwockFormal.Rust.*`。
別コンパイラのモジュールや、古い実装hashに結び付いた証明は使用しない。
現時点で各実装の証明モジュールは未制作であり `NOT_IMPLEMENTED / NOT_PROVEN`。
モジュール追加後もLean成功だけでは実装対応の正しさを認定せず、`KERNEL_CHECKED` と区別する。
共有抽象モデルは明示的な `--abstract-model` でのみ追加実行する。
