# コンパイラ別の形式検証

[実行コマンド・必要環境・結果の読み方](../docs/formal-commands.md) を参照。

```powershell
py -3 scripts/formal_report.py --compiler jwc --output target/formal-jwc-report.json
py -3 scripts/formal_report.py --compiler rust --output target/formal-rust-report.json
```

`JabberwockFormal/Jwc/ReferenceAccess.lean` と `Rust/ReferenceAccess.lean` に
各8定理の局所証明を実装。実装との対応は手動モデルであり未証明。
両コンパイラの5目標全体は未証明。Leanの実行結果は利用者が確認する。

## 共有抽象モデル

固定幅整数9型の値域、丸め、変換、算術・ビット演算、除算・シフトの意味論を
`JabberwockFormal/MachineIntegers.lean` に追加。
[定義・実装との未証明の対応・確認状況](../docs/machine-integer-semantics.md) を参照。
Leanでの確認は未実施であり、コンパイラ全体の証明や4機構の完成には数えない。

# 形式仕様の検査

[形式仕様の範囲と退役規則](../docs/formal-semantics.md) の機械検査部分。
Lean 4.24.0の標準ライブラリだけを使用し、mathlibや外部パッケージは追加しない。
`lean-toolchain` と `lake-manifest.json` を固定する。

実用テスト1〜12とは別に、[形式証明専用の検証様式](../docs/formal-verification.md) でF1〜F5を管理する。
[テンプレート](records/TEMPLATE.md) と既存の [世代・退役](records/F2-001-generation.md) / [所有位置のムーブ](records/F4-001-ownership.md) の記録を用意した。
`records/` は検査した時点の証拠と進捗であり、Leanの証明やCIの成功を追加するものではない。

Lean/elanを導入済みなら、リポジトリのルートから次を実行する。

```sh
python3 scripts/check_formal.py
```

elanがない場合も、公式Lean 4.24.0配布のbinをPATHへ入れれば実行できる。
CIは公式Linux配布のSHA-256を確認してから同じコマンドを実行する。
Leanがない・版が違う・証明が通らない場合は失敗とし、skipで成功にはしない。

| モジュール | 現在の範囲 |
|---|---|
| `JabberwockFormal/Generation.lean` | 有限の世代上限、退役、参照の有効性、許された有限遷移列、読書きの検査 |
| `JabberwockFormal/Ownership.lean` | 抽象的な所有位置間のムーブ、移動先の事前検査、一意な所有者の保存 |

`lake build` に加え、全ての `theorem` の `#print axioms` を確認する。
標準Leanの `propext`・`Quot.sound`・`Classical.choice` 以外の公理依存は拒否する。
証明の穴、独自公理、native実行による証明の置換は受け入れない。
これはLeanのkernelと固定したツールチェーンを信頼する検査であり、そのツール自体の証明ではない。

この段階の定理は抽象モデルについてのもの。
構文・型検査・複合値の回収・スレッド・静的解析・C/LLVM実装との対応は未証明であり、
ビルド成功を「言語全体の安全性の証明」と表示しない。
