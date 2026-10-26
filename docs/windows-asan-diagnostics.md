# Windows ASan起動失敗の診断

通常の実用テストに組み込み済み。リポジトリ直下で実行する。

```powershell
.\scripts\safety_windows.cmd
```

`target/safety-windows-report.json` の `results` → `windows-asan-preflight` →
`case_report.results[].diagnostics` に診断を丸ごと格納する。このJSON一つをアップロードすればよい。
別ファイル `target/safety-windows-report-work/windows-asan-diagnostic.json` にも保存する。

環境設定済みのx64 Developer端末で、診断だけ行う場合:

```powershell
py -3 scripts/check_windows_asan.py --output target/windows-asan-diagnostic.json
```

## 記録内容

- 元の実行の終了コード（10進・16進）、stdout/stderr。
- `loader_trace.modules`: 診断用プロセスをWindowsデバッグAPIで再実行し、LOAD_DLL_DEBUG_EVENTのファイルハンドルから取得した実際のDLLパス。PATH候補と区別する。
- `loader_trace.exceptions` / `debug_strings`: ローダー例外、取得できたデバッグ文字列、診断プロセスの終了コード、トレース完了の有無。
- `pe_analysis.modules`: 対象exeと観測したDLLのSHA256、machine、要求DLL名と関数名/ordinal。
- `pe_analysis.missing_exports`: 実際に観測した同名DLLが要求関数を公開していない組み合わせ。要求元・要求関数・読み込み先を記録。
- `unresolved_imports` / `errors`: API-setの別名、取得失敗、転送先未観測、同名DLL複数等。推測で読み込み先を埋めない。

PEのimport/exportは標準ライブラリで静的に読み取る。公開関数の転送は観測済みDLLまで追跡する。
API-setのホスト解決、delay-load、動的GetProcAddressの全経路は未対応。
Windowsデバッガ起動権限がない場合や途中でローダーが停止した場合も、取得できた情報と失敗理由を残す。
診断用再実行は通常実行とは異なるため、元の実行結果と診断プロセスの結果を別に保持する。
この仕組みでも不足関数を特定できなかった場合は未解決と明記する。

64-bit Pythonが必要。管理者化、レジストリ変更、DLLのコピー/削除は行わない。
診断プロセスには時間制限を設け、タイムアウト時に終了させる。
起動・検出器の失敗はINFRA_ERRORになり、ASan依存テストを合格にしない。

実装時にはWindows実行・テストを行っていない。回帰テストには合成PEの名前/ordinal/転送と一ファイル報告への格納を追加した。
