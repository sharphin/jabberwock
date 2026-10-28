# 実ソースの契約証明と受入経路

## 今回の対象

テスト入力を個別に追加する作業ではない。実ソースから取り出した関数の式に対し、
**全入力を量化した命題**を生成し、Leanで検査し、別工程で再検証して受け入れる。

最初の対象はjwcに埋め込む `runtime/jw_rt.c` の `gen_has_room`。
現在の証明生成・再検証コードは実装済みだが、**Lean実行・Windows/Linux検証は未実施**。
コンパイラ全体の実装対応証明は未完成であり、これを完了としてテストを再開しない。

## 命題と信頼する前提

通常ビルドの `GEN_LIMIT = 2^62 - 1` に固定し、すべての64ビット符号なし整数
`gen`、`step` に対して、抽出したC関数の返値が真であることと、次が同値であることを証明する。

```
gen != UINT64_MAX
and step <= 2^62 - 1
and (gen & (2^62 - 1)) <= (2^62 - 1) - step
```

`UnsignedGuardSource.lean` が、対応する式の64ビット値意味論を定義する。
減算は符号なし剰余演算、シフトは0〜63、論理積は短絡評価。
仕様側は既存の `RuntimeGenerationBitVec.guardSpec` を使い、レポート側から変更できない。
定理本文は新しく抽出した式から生成する。証明はLeanの簡約・書き換えを用い、
許可する公理は既存と同じ `propext`、`Quot.sound`、`Classical.choice` のみ。
具体値を列挙するテスト、ソルバーの成功文字列、ハッシュ一致を証明の代わりにしない。

これは以下を信頼する**条件付きの関数契約証明**である。

- Clangの前処理・構文解析・ターゲットのC型幅。
- Pythonの閉じた型付きAST抽出器と、その式断片のC値意味論への対応。
- Lean 4.24.0のカーネルと標準依存関係。

Clang自体や抽出器自体の正当性を形式証明したものではない。
ヘッダー、ツールチェーン、ビルド成果物の完全な来歴証明でもない。
`jwc/runtime.jw` が `sync_runtime.render` と一致することも確認するが、
jwcがその文字列を正しく生成・コンパイル・リンクすることの証明は別に必要。

## 検証・受入の順序

1. ランタイム・埋め込み文字列・検査器・Lean入力のスナップショットを記録する。
2. Clangに通常構成を前処理させ、実際の `gen_has_room` 定義の型付きASTを取得する。
3. 引数2個・return文1個と、許可した整数演算だけを受け付ける。呼出し、メモリ操作、
   別の文、未対応型・キャスト・副作用・範囲外シフト等は拒否する。
4. ASTから全称命題を生成し、Leanの証明検査と公理監査を行う。
   一時プロジェクトに記録済みのLeanソースをコピーしてビルドし、作業ツリーの
   `.lake` キャッシュは流用しない。標準Leanのライブラリは信頼する前提に含む。
5. ソース・検査器とClang/Lean/lake起動実体が途中で変わっていないことを確認して
   証拠JSONを出力する。
6. `source_contract_acceptance.py` がローカルの設定で前処理・抽出・証明を再実行する。
   入力レポートに書かれた実行ファイル、コマンド、定理本文は実行しない。
   再生成した証拠全体が一致した場合だけ、契約に限定した受入を記録する。

証拠JSONは同じ作業ツリー・同じツールで再検証するためのもの。
別のソース版やツール版で生成した証拠は受け入れず、その環境で生成し直す。

| 結果 | 意味 |
|---|---|
| `CONTRACT_ACCEPTED_ASSUMING_FRONTEND` | 上記前提の下で、この関数契約だけを検査できた |
| `INVALID_EVIDENCE` | 検査失敗・未対応構文・環境不足・証拠不一致など。成功扱いしない |
| `whole_implementation: NOT_PROVEN` | 契約の成否にかかわらず、全体証明は未完成 |

未対応構文や検査器の失敗を「言語が危険だと証明した」とも解釈しない。
`reason` と工程のログで区別する。

## コマンド（実行再開時に使用）

前提はPython、Clangの使用可能なCヘッダー環境、Lean 4.24.0 / lake。
Windowsでは既存のx64開発者環境を初期化してから実行する。
`JW_CLANG` はローカルのClangパス指定に利用できる。

```sh
# 実ソースから契約証明を生成・検査する。安全性テストは実行しない。
python scripts/runtime_source_contract.py --output target/runtime-source-contract.json

# 同じソースとローカルツールで証明を再実行し、契約単位で受け入れる。
python scripts/source_contract_acceptance.py --report target/runtime-source-contract.json --output target/source-contract-acceptance.json

# 既存の形式検証一括コマンドにも両工程を接続済み。
python scripts/formal_tool.py --output target/formal-tool
```

単独の2コマンドは契約が受理できれば終了値0、検査できなければ2。
一括コマンドの終了値は従来どおり、全体証明完了0、未証明1、工程エラー2。
契約が受理されても全体の終了値0にはしない。

既存の34項目受入監査にも、任意の `--source-contract` 引数を追加した。
この引数を付けた場合だけ、監査中にLeanを再実行する。

```sh
python scripts/acceptance_report.py --os windows --practical target/safety-windows-report.json --formal target/formal-report.json --inventory target/whole-proof-inventory.json --transition target/formal-tool/refinement.json --source-contract target/formal-tool/runtime-source-contract.json --output target/acceptance-34.json
```

Linuxでは `--os linux` と対応する実証レポートを指定する。
`accepted_source_contracts` に契約の受入を記録するが、34項目の合格数には加算しない。
Rust版の証明結果にも流用しない。

## 全体証明に残る作業

この変更をもって「不足していた実装対応証明が全部揃った」としてはならない。
次の対応証明を各コンパイラに接続し、前提の整合性を確認して合成する必要がある。

- 字句解析・構文解析・型検査・所有権検査と全言語仕様。
- ASTからIRへの変換、解析・最適化・チェック省略。
- LLVM出力の全命令・制御フロー・未定義動作条件。
- ランタイムの確保・解放・退役・ポインタ寿命・並行実行。
- ビルド来歴・リンク・FFI/OS境界と、最終的なG1〜G5の合成。

全体用の `refinement_transition.py` のレジストリは、この局所契約で埋めない。
型や命題の一致を検証する全体用チェッカーが未実装である事実を保持する。
