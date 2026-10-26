# 形式仕様の検査

[形式仕様の範囲と退役規則](../docs/formal-semantics.md) の機械検査部分。
Lean 4.24.0の標準ライブラリだけを使用し、mathlibや外部パッケージは追加しない。
`lean-toolchain` と `lake-manifest.json` を固定する。

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
