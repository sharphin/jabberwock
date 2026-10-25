# 初期版の設計範囲と実装への引渡し

状態: 初期版の設計成果物。機能本体は未実装。
残っていた公開型/API・個別C ABI・カタログ/登録表を一括で具体化した。

| 設計対象 | 正本 |
|---|---|
| 公開149操作・142型、引数順・所有・戻り値 | [公開API](public-apis.md)、[登録表](stdlib-v1/registry.json) |
| 操作ID・C symbol・Args/Outの配置 | [個別操作](native-operations.md)、[Cヘッダー](stdlib-v1/stdlib-abi-v1.h) |
| codec、部分結果、transaction、既定値・上限 | [公開契約とC境界](stdlib-v1-contracts.md) |
| 配布・依存・登録・生成・hash | [カタログ/登録表](catalog-registry.md) |
| GUIのtree・配置・入力・TSF/UIA | [GUI内部仕様](gui-internals.md) |
| 型・context・世代・drop | [資源ABI](resource-abi.md) |
| 設定・CLIとcompilerの受渡し | [alice設定](alice-configuration.md)、[compiler連携](compiler-integration.md) |

以上で、今回列挙していた初期版の残り3項目を設計成果物にした。
個別設計の候補表記との相違は公開契約側を優先し、実装で変更が必要なら理由と互換性を記録して改訂する。
実装中に新しい問題が見つからない、またはこの設計の全てが実環境で成立した、と保証するものではない。

実装は模擬資源とABI、File、共通loader/alice/jwc連携、Window/描画、TCP/TLS/PostgreSQL、GUIの順に進められる。
各段階で必要な全実行器・負例・失敗注入を通し、部分成功をライブラリ全体の完成としない。

| 実装後の検証 | 合格条件の区別 |
|---|---|
| 共通C/LLVM | layout、Owner/View、allocator、未初期化、世代/clock/window混同、途中失敗 |
| Rust/IR/native/jwc | 同じ型・意味・診断・回収とbootstrap。外部副作用の通常実行は一回 |
| Linux | File/TCP/libpqの実処理、模擬GUI、ASan/UBSan/MSan/TSanと負例。Linux TLS/画像/GUIはUnsupported |
| Windows x64 | clang/MSVC ABI、実File/Winsock/Schannel/libpq/Win32/WIC/COM、WARPの描画 |
| Windows対話/実機 | 表示、実GPU、DPI/focus、日本語IME/NarratorとUIAの実操作 |
| 配布 | 旧形式互換、内容指紋、同時build、DLL不足/差替え、移動先起動、条件不足の拒否 |

新機能へ既存の安全性1〜7の合格を流用しない。新しい操作を固定攻撃・生成入力とCIへ追加する。
MSanは依存ライブラリの計装範囲も明記し、skip/unsupported/未実施を合格数に含めない。
資源枯渇・巨大型に関する既存の別課題を、この設計で解決済みとは扱わない。

HTTP/WebSocket/UDP/非同期I/O、pool/COPY/LISTEN/NOTIFY、行stream/監視/mmap、
3D/shader/音声/複数行GUI/ScrollView/ゲームパッド/Linux描画、registry配信・lockfile・任意FFIは初期版の外。
これらを今回の未完了作業として混ぜず、追加する際に別の版/範囲として設計する。
