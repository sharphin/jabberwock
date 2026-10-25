# PostgreSQLライブラリの設計

状態: 実装前の設計。API名・型名・配置先は提案であり、利用可能な機能ではない。
[標準ライブラリ全体の方針](standard-libraries.md) のPostgreSQL部分を具体化する。
この文書の追加では、コンパイラ・alice・既存プログラムの動作を変更しない。
関連する境界は [共通資源管理](resource-management.md) と [aliceのネイティブ依存](alice-native.md) に定める。
型・引数・返却型は [公開API](public-apis.md)、初期adapter・認証・既定値・C境界は [v1公開契約](stdlib-v1-contracts.md) に具体化する。

## 目的と初期範囲

Windows向けアプリとCLIから、PostgreSQLへ安全に接続してSQLを実行する。
GUIではworker内で接続を作り、取得した値をchannelでGUIスレッドへ渡せる構成にする。
ドライバはPostgreSQLの公式Cクライアントライブラリlibpqを使い、認証・TLS・通信プロトコルを自作しない。
外部Rustクレートを使わない現在の方針を保つ。

最初の範囲は次のとおり。

- 構造化した接続設定、認証、接続先名と証明書を検証するTLS接続。
- パラメータ付きの単一SQL、コマンド結果と行データ、NULLとbytea。
- 明示的なbegin・commit・rollbackと、トランザクションの所有。
- 接続・操作の期限、結果の行数・サイズ制限、構造化したエラー。
- Windows x64での配布、aliceによる依存の検出とビルド、使い捨てDBによる結合試験。

接続プール、公開された非同期API、COPY、LISTEN/NOTIFY、大きなオブジェクト、名前付きprepared statement、
ORM、自動マイグレーション、ネストしたトランザクションは初期範囲に含めない。
libpq自体の対応範囲と、このライブラリが実際に検証した対応範囲は分けて記録する。
初期clientはlibpq 18.xの対応版に絞り、server 16/17/18を結合試験の候補とする。成功した構成だけを対応済みとする。
Windowsを利用者向けの先行環境とし、Linuxでも共通の接続処理・資源管理をCIで検証する。
これはGUIのLinux対応を決めるものではない。

## 構成

| 層 | 責任 |
|---|---|
| `postgres` パッケージ（公開名の案） | 接続設定、パラメータ、結果、エラー、トランザクションのAPI |
| コンパイラの資源型 | Connection・Transactionの所有、借用、ムーブ、送信禁止 |
| 共通のC接続処理（`runtime/jw_pg.c` の案） | libpqの呼び出し、通信の進行、PGconn・PGresultの解放、エラーの変換 |
| インタプリタ・IR実行 | 同じ接続処理を呼び、同じ値とエラーへ変換する |
| LLVM生成・自己ホスト版 | 同じ資源ABIと接続処理を利用するコードを生成する |
| alice | パッケージの能力依存、libpqの探索・配布情報、ビルド指紋 |

libpqを呼ぶ実装をインタプリタとC runtimeで別々に作らず、共通のC接続処理を第一案とする。
Rust版の配布バイナリはビルド時にこの接続処理を含められる必要がある。
そのためのCコンパイル手順・ヘッダー・対象ABIの管理も実装対象であり、現在できるとは扱わない。

## 公開APIの草案

以下はインターフェースの草案。現在の言語に不透明な資源型が追加済みという意味ではない。

| 操作 | 戻り値・所有の扱い |
|---|---|
| `connect(options)` | `Result<Connection, PgError>` |
| `Connection.run(sql, params, options)` | `Result<QueryResult, PgError>`。接続は借用し、同時に実行できる操作は1つ |
| `Connection.begin(options)` | 接続をムーブし、`Result<Transaction, BeginFailure>` |
| `Transaction.run(sql, params, options)` | `Result<QueryResult, PgError>` |
| `Transaction.commit(deadline)` / `rollback(deadline)` | Transactionをムーブし、`TxEnd` |
| `Connection.close()` | Connectionを消費する。以後その所有値・別名では利用できない |

`run`はSELECTと更新を共通に扱う。SQLを既に実行した後で「行を返すSQLだったから失敗」とするAPIを避ける。
QueryResultは行データを持つ結果と、行を持たないコマンド結果を区別する。
クエリビルダーや文字列連結による値の埋め込みは初期APIに含めない。

## 接続の所有と状態

Connectionは複製できない不透明な所有値とし、ユーザーが整数や構造体リテラルから組み立てられないようにする。
PGconnのポインタは公開しない。共通の資源管理で、種類・世代・開閉状態を確認する。

通常の状態遷移は `Connecting → Idle → Busy → Idle → Closed`。
通信・プロトコルの異常や、期限・サイズ上限による中断では `Busy → Broken → Closed` とする。
壊れた接続を自動再接続して、元の操作を再実行することはしない。

- 初期段階ではConnectionとTransactionをスレッド間で送れない型にする。
- 構造体・enum・リスト・クロージャに包んでも送信禁止をすり抜けられないようにする。
- 作成スレッドをruntimeにも記録し、内部APIでも誤用を拒否する。
- `ref mut`は排他的借用ではない。別名があってもBusy中の再入・別操作を拒否する。
- libpqからユーザーの関数を呼ぶコールバックを提供しない。通知は内部で処理する。
- 明示close後の破棄は再びPQfinishしない。部分的な接続失敗でも、返されたPGconnがあればPQfinishする。

結果をすべてjabberwockの値へコピーした後、Connectionを閉じても行データは使える。
ConnectionをworkerからGUIへ送る代わりに、資源を含まない行データやPgErrorを送る。

## 接続設定・認証・TLS

接続設定はhost、port、database、user、認証情報、CA証明書、接続期限、操作期限、結果制限を個別の項目として渡す。
初期段階では任意のURI・conninfo文字列・service名・libpqの任意オプションをそのまま公開しない。
`PQconnectStartParams`には個別のキー・値を渡し、dbnameの接続文字列としての展開を無効にする。
単一の接続先から始め、複数hostへの透過的な切り替えは初期範囲に含めない。

通常の接続では `sslmode=verify-full` とCA証明書の場所を明示する。
TLS非対応のlibpqや、TLSへ移行できない接続では失敗させる。接続後もTLSの利用を確認する。
IPアドレスを使う場合も、そのIPに対する証明書検証を満たす必要がある。
平文を許す例外は、CI・開発用に明示した数値ループバックアドレスに限定する案とする。
DNS名の `localhost` を文字列だけでループバックとして扱わない。

v1はPG*環境変数が存在する場合に拒否し、非空passwordのSCRAM認証と明示TLSを使う。
passfile・service・client certificate/GSSを初期の公開認証選択へ含めない。root.crl等のlibpq既定値の範囲もv1契約に記す。
TLSなど安全性に関わる設定は明示値で固定し、環境変数で黙って弱められないよう試験する。
プロセス全体の環境変数を書き換えて制御しない。
passwordなどlibpqへC文字列で渡す値には埋め込みNULを認めず、長さとUTF-8の妥当性を境界で確認する。
接続成功後はclient_encodingをUTF8に固定し、設定できなければ接続を返さない。

## SQLとパラメータ

`PQsendQueryParams`を使い、SQLと値を別々に渡す。SQLはUTF-8で、埋め込みNULを拒否する。
1回の呼び出しは1つのSQLコマンドに限定する。セミコロンで複数の更新をまとめるAPIは提供しない。

v1のパラメータ型は以下とする。

| パラメータ | PostgreSQL側での扱い |
|---|---|
| Null（任意の型OID付き） | NULL。型が推論できなければサーバーの型エラーを返す |
| Bool | bool |
| I64 | int8。狭い型の列への代入ではサーバーの範囲検査を使う |
| F64 | float8 |
| Text | UTF-8のtext |
| Bytes | bytea。バイナリ形式で長さを指定し、0バイトも保持する |

SQLの識別子・表名・並び順などを値パラメータとして置き換えられるとは扱わない。
たとえば `WHERE id = $1` に値を渡し、NULLの型推論が曖昧なら `$1::bigint` のようにSQLで型を指定する。
金額などの正確なnumericをf64へ勝手に変換しない。初期段階ではTextと明示的なSQLの型指定で扱う。

パラメータ数・各バッファ長がlibpqのC引数やプロトコルの上限に収まることを、縮小変換の前に検査する。
パラメータの領域は少なくとも操作が完了するまで保持し、送信中に借用元を解放できないようにする。

## 結果と型変換

初期APIでは、結果のネイティブ領域を保持する公開カーソルを作らず、上限内の結果を所有する値へコピーして返す。
Columnは名前と型OIDを持つ。名前が重複しても保持し、列番号で確実に参照できるようにする。
名前による取得は、見つからない場合と重複して曖昧な場合を区別する。

CellはNull・Text・Bytesを区別する案とする。初期結果形式はlibpqのテキスト形式を基本とし、
byteaは型OIDを確認して復号する。numeric、日時、JSONなどの元の表現は保持する。

- NULL判定はPQgetisnullで行う。空文字と空byteaはNULLではない。
- PQgetvalueの領域はPQgetlengthの長さで読み、strlenで切らない。
- PGresultが生きている間にコピーし、その後に必ずPQclearする。
- byteaの復号でlibpqが確保した領域は、正しい解放関数PQfreememで解放する。
- 整数への変換は桁・符号・範囲を検査してエラーを返す。失敗を0に置き換えたり、切り詰めたりしない。
- 受信したUTF-8が不正なら文字列として公開しない。部分的に作った結果も解放する。
- コマンドタグと影響行数を保持する。行数がない場合と0行の場合を区別し、数値の上限を超えたら明示的に失敗する。

行数・列数・合計バイト数には設定可能な上限を設ける。v1の初期値は10万行・1024列・合計64 MiBとし、
加算・乗算があふれる前に判定する。上限到達時は接続を閉じ、操作結果が不明になりうることも返す。

内部では送信直後にsingle-row modeを選び、行を順次回収して上限を検査する案とする。
ただしlibpqは単一の大きな行を返す前に確保しうる。これをプロセス全体の厳密なメモリ上限とは説明しない。
厳密な上限が必要な実行環境ではOSの制限と組み合わせる。

## 期限と内部の通信処理

公開APIはまず同期式とするが、内部はlibpqの非ブロッキングAPIで進行させる。
GUIを止めないため、接続とSQLの実行はworker内で行う。

- 接続はPQconnectStartParams/PQconnectPoll、SQLはPQsendQueryParamsから開始する。
- PQflush、PQconsumeInput、PQisBusy、PQgetResultの状態とソケットの読み書き待ちを組み合わせる。
- 待ちには単調増加する時計の締切を使う。応答のたびに全タイムアウトを延長しない。
- libpqが返すソケットは状態により変わりうるので取り直し、WindowsのSOCKETの幅を保つ。
- 単一SQLでも、正常な結果・エラー・single-rowの終了結果を回収し、PQgetResultがNULLになるまで完了としない。
- COPYなど未対応の結果状態は、接続を再利用できると決めつけず閉じる。

非ブロッキングAPIを使っても、libpq内部のDNS・認証・ファイルアクセスすべてに厳密な時間上限が付くとは限らない。
初期仕様では接続期限を「ライブラリが制御できる待ちの締切」とし、この限界を明記する。
厳密な接続期限が必要なら、別途制御したDNS解決や認証方式の制限まで検証する。

初期段階では期限超過時にPQfinishして接続を破棄する。同期のcancel APIを、必ず期限内に戻るものとして組み込まない。
必要に応じてサーバー側のstatement_timeout/lock_timeoutも使うが、値は検査した整数から内部で設定する。
切断やキャンセル要求で、サーバーが確実に処理を取り消したとは扱わない。

## トランザクション

`ref mut Connection`は排他的ではないため、Transactionが接続を借りるだけの設計にはしない。
beginでConnectionをTransactionへムーブし、トランザクションが接続を所有する。
移動前の所有値・借用から操作できないことを、型とruntimeの双方で確認する。
BeginFailureはPgErrorと、再利用できる場合だけOption<Connection>を所有する。

commit/rollbackはTransactionを消費し、次の情報を持つTxEndを返す案とする。

| 情報 | 意味 |
|---|---|
| outcome | Committed / RolledBack / Unknown |
| connection | 再利用可能だと確認できた場合だけOption<Connection> |
| error | サーバーによる拒否、通信失敗などがあればPgError |

エラーの有無だけで再実行を決めず、outcomeを必ず確認する。
COMMITの完了応答を受信した後に接続が壊れた場合は、Committedを保ちながらconnectionをNoneにできる。
応答を失って成否を証明できない場合はUnknownとし、接続を破棄する。

- 各操作の後にPQtransactionStatusを検査する。エラー後のINERRORと、正常なINTRANSを区別する。
- エラーになったトランザクションに対するCOMMITがROLLBACKとして終了した場合、Committedとして返さない。
- コマンドタグ、サーバーの応答、SQLSTATE、接続状態を使って判定する。証明できない結果をRolledBackと断言しない。
- 読み取り専用・分離レベルは固定の選択肢から内部SQLを作る。任意文字列を埋め込まない。
- ユーザーのSQLで管理外のBEGIN/COMMITなどが実行された場合は状態の不一致を検出し、接続を閉じる。
  単純なSQL文字列の検査で、トランザクション制御を完全に禁止できたとは扱わない。
- 通信失敗・期限超過・serialization failure・deadlockを含め、初期APIはSQLを自動再実行しない。

未完了のTransactionを通常のスコープ終了で捨てた場合は、接続を閉じて再利用させない。
切断によるサーバー側の後始末は、同期のrollback完了通知ではない。
rollbackの結果が必要なら明示的にrollbackする。
既存のexit・実行時エラーは後片づけをしないため、暗黙のcommitや確実なrollbackを約束しない。

## エラーと機密情報

PgErrorは、接続・認証・TLS・期限・サーバーSQL・型変換・上限・資源誤用・通信/プロトコルなどの種類を持つ。
SQLSTATEは存在するときだけ保持し、通信エラーへ架空のSQLSTATEを付けない。
接続消失時の操作結果の不確実性も保持する。SQLSTATEと切断だけで「未実行」と判断しない。

既定の表示は、安全な操作名・エラーの種類・SQLSTATEを中心にする。
libpqのエラー文、サーバーのMESSAGE/DETAIL/CONTEXTにはSQLの値や個人情報が含まれうるため、自動表示しない。
詳細診断は明示的に取得する機能として分け、文字列の置換だけで必ず秘密を除けるとは説明しない。
SQL・パラメータ・接続設定・パスワードを自動でログに残さない。
libpqの既定のNOTICE出力も抑制し、接続の進行を始める前から内部の受信処理を設定する。

## 配布・alice・コンパイラへの影響

PostgreSQLを使うパッケージだけが、登録済みのネイティブ能力 `postgres` を要求する案とする。
alice.tomlとマシン別のlibpq指定は [aliceの設定仕様](alice-configuration.md) に定める。
現在のaliceが任意のnative設定を認識するとは扱わない。
任意のビルドスクリプトや任意のCソースを実行する機能へ、そのまま拡張しない。

Windowsはまず明示したlibpq.dllを動的に読み込む案とし、DLLの探索を作業ディレクトリやPATHに任せない。
絶対パスと制限したDLL探索でロードし、依存DLLを含めた配置、アーキテクチャ、必要な関数と版を確認する。
必要なDLLがなければ診断を返し、PostgreSQLを使わないアプリにはDLLを要求しない。
関数表は初期化を同期して共有し、利用中の資源を残してライブラリをアンロードしない。
Windows以外も、明示したライブラリから同じC接続処理を使えるよう設計する。

aliceは、接続処理・ヘッダー・libpqの版と配置・依存DLL・対象環境・ABIをビルド指紋へ含める。
ライブラリの変更・削除や、能力の追加・削除で古い生成物を使い続けないようにする。
配布物には依存関係と必要なライセンス文書を含める。認証情報はビルド設定や指紋へ保存しない。

Rust版の型検査、インタプリタ、IR、LLVM、C runtimeと、jwc側の型・IR・LLVMに同じ資源ABIを実装する。
Rust版から共通C接続処理を呼ぶビルド手順、jwcのパッケージ対応、任意のネイティブ能力の検証も必要になる。
任意のユーザーコードを呼ぶ汎用FFIを公開することは、初期実装の前提にしない。

DB更新を既存のデバッグ比較機能で何度も実行してはならない。
実際のアプリの通常実行では、インタプリタ・IR・ネイティブの自動照合によるDB操作の再実行を抑止する仕組みを入れる。
比較試験では各実行器に独立したDBまたはschemaと同じ初期データを用意する。
実DBへの一度の操作を、rollbackだけで副作用が完全に消えると扱わない。sequenceや外部処理などは戻らない場合がある。

## 実装順序と完了条件

1. 共通の資源型・所有・close・スレッド制約と、aliceの能力依存・ビルド指紋を実装する。
2. C接続処理、接続設定、TLS、明示closeを実装し、接続の失敗経路も含めて確認する。
3. 単一SQL・パラメータ・結果コピー・NULL/bytea・上限・エラーを実装する。
4. Transactionの所有移動とTxEndを実装し、COMMITの結果不明を再現する。
5. Rust/IR/ネイティブ/jwcで独立DBを使って比較し、workerから値だけをGUIへ渡す例を動かす。

完成とする条件は、次の全項目の成功を記録すること。

| 試験 | 確認内容 |
|---|---|
| 基本の結合試験 | 接続、SELECT、更新、影響行数、NULLと空文字、空byteaと0バイトを含むbytea、日本語、結果の独立した寿命 |
| パラメータ | 引用符・改行・SQLのような値が命令にならないこと、NULLの型推論失敗、整数の端とnumericの保持 |
| 資源管理 | 部分失敗、二重close、ムーブ後使用、close後の別名、異種資源、世代再利用、thread送信禁止、Busy中の再入 |
| トランザクション | commit、明示rollback、途中のSQLエラー、INERROR、未完了での破棄、管理外の制御SQL |
| 障害注入 | 接続拒否、認証失敗、送受信中の切断、応答の分割、期限、上限、COMMIT送信後の応答消失 |
| TLS | 正しいCAと接続先、未知のCA、期限切れ、接続先名不一致、TLS無効なサーバー、環境変数による格下げの拒否 |
| 機密情報 | 接続情報・値を含むサーバーエラー・NOTICEが既定ログへ漏れないこと |
| 配布・再ビルド | DLLと依存DLLの不足、誤ったアーキテクチャ、探索先、版変更、ABI変更、能力の追加・削除 |
| 実行器の一致 | 独立DBでRust/IR/ネイティブ/jwcの結果・診断・所有を比較。GUIに依存しないworker例を含む |
| 安全性1〜7 | 共通接続処理をASan/UBSan・MSan・TSanで検査し、負例・固定攻撃・生成入力を追加 |

未知のCOMMIT結果は、使い捨てDBと通信中継・障害注入で再現し、確認用の別接続でDBの実際の状態も調べる。
サーバーにcommit済みであるケースでも、応答を失った呼び出し元へUnknownを返すことを確認する。
第三者ライブラリの検査範囲も記録する。MSanでlibpq内部まで検査するときは、依存ライブラリの計装が必要であり、
未計装のlibpqを使っただけで全経路の未初期化読みを検査済みとはしない。

この文書は設計の完了であり、上の試験を実施済みという報告ではない。

## 参照する公式仕様

実装時には、実際に対応する版の仕様と照合する。以下はPostgreSQL 16の公式文書。

- [接続開始・接続パラメータ](https://www.postgresql.org/docs/16/libpq-connect.html)
- [SQL・パラメータ・結果](https://www.postgresql.org/docs/16/libpq-exec.html)
- [非ブロッキングの操作](https://www.postgresql.org/docs/16/libpq-async.html)
- [single-row mode](https://www.postgresql.org/docs/16/libpq-single-row-mode.html)
- [TLSと証明書の検証](https://www.postgresql.org/docs/16/libpq-ssl.html)
- [環境変数による既定値](https://www.postgresql.org/docs/16/libpq-envars.html)
