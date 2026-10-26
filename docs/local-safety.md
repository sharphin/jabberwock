# ローカル安全性テスト（GitHub Actions不要）

この仕組みは [verification-plan.md](verification-plan.md) の **実用安全性12項目** に対応する既存テストの実行入口です。
**12項目の合格を自動認定するものではありません。** 合格条件には、網羅性、OS、検出器の負例、実行時の副作用など、別途判定すべき内容が残ります。

## 前提

- Python 3.10以上、Rust/Cargo、Cコンパイラの clang
- 包括検証には Linux x86_64 推奨。clangの sanitizer runtime（ASan/UBSan、TSan、MSan）、lld
- 形式モデルの検査には `lake` と Lean 4.24.0（`formal/lean-toolchain`）
- Windowsは通常のRust/nativeテストの対象ですが、Linux固有のsanitizerやRLIMIT検査は **NOT_RUN**。両OSでの実行記録を別々に残してください
- `clang --version`、`cargo --version`、`lake env lean --version` で準備状況を確認してください
- sanitizerがランタイム起動に失敗する環境ではその試験は失敗です。スクリプトはOSの `sysctl` を勝手に書き換えません

## 使い方

リポジトリルートで実行します。

```sh
# 日常の高速な開発確認（対象を絞ったテスト）
python3 scripts/local_safety.py quick

# 実際にコマンドを起動せず、実行計画を確認
python3 scripts/local_safety.py full --dry-run

# Linuxで既存の包括的な安全試験を再現（未実行も終了コード非0）
python3 scripts/local_safety.py full --strict

# 例：未解決の8・9だけを再テスト
python3 scripts/local_safety.py full --items 8,9 --strict

# 例：生成プログラム数を20倍にする
python3 scripts/local_safety.py full --scale 20 --strict

# 出力先を固定
python3 scripts/local_safety.py full --strict --report-dir target/local-safety/manual
```

Windowsでは `python` に読み替えられます。サニタイザ非対応の試験を含む `full --strict` は、Windowsで成功終了しないのが意図した挙動です。

## 実行内容

スクリプトは既存の `safety.yml`、`formal-spec.yml`、`test.yml`、`bench-checks.yml` と専用再試験スクリプトをもとに、以下を統合します。

- Rust通常テストと `JW_NATIVE=1` 差分テスト、退役テスト、ベンチマークの検査回数
- sanitizer自身が実際に欠陥を検出できることを確認する負例
- ASan/UBSanでの堅牢性テスト、変異入力のnative実行、型の通るファジング（O1とO2）
- TSanスレッド試験、MSan全体試験
- 自己ホスト `jwc` のビルド、jwc生成実行ファイルのASan/MSan差分比較
- 項目8・9の専用再テスト（資源枯渇と巨大配列）
- 項目10〜12の外部副作用を検証する部分テスト、Rust releaseテスト
- Leanモデルのビルドと公理監査

**重要:** `jwc` の生成物への `JW_CFLAGS` の伝播は既存実装の制約を受けます。Rust側でsanitizerフラグを指定しただけで、jwc生成物まで完全に計装されたとみなさないでください。

## 結果の読み方

ログと `report.json` は既定で `target/local-safety/<UTC時刻>/` に保存します。

- `PROBE_PASS`：そのコマンドが終了コード0になりました
- `FAIL`：コマンド失敗・タイムアウトなど
- `NOT_RUN`：必要なツールや対応OSがなく実行できませんでした
- `PROBES_PASS_ONLY`：項目に紐付く今回の試験は成功しましたが、**項目全体の合格ではありません**
- `acceptance: NOT_ASSESSED`：12項目の包括的な合格判定は実施していません

`--strict` は試験失敗だけでなく `NOT_RUN` も非0終了にします。未解決の8・9や10の副作用問題が再現する場合は `FAIL` のまま記録し、無理に緑にしません。

`quick` は短い開発用プロファイルで、全項目を実行しません。包括的な証拠には `full --strict` とLinux/Windowsそれぞれの実行記録が必要です。

### 注意と限界

- GitHub Actionsを停止・削除する変更は含みません。既存CIと並行運用した後、必要なら無効化してください
- 本変更はテスト実行器のみで、コンパイラ本体・言語仕様・既存テストの合格基準は変更しません
- コマンドが環境によって条件付きでテストをスキップすることがあります。終了コード0やテスト件数だけで実行到達を保証できません。各ログと `docs/verification-plan.md` の受け入れ条件を照合してください
- `full` はファジングやsanitizerを多用するため高負荷・長時間です。独立した作業環境での実行を推奨します
