# 通信ライブラリの設計

状態: 実装前の設計。型・API・設定は提案であり、既存機能ではない。
[全体方針](standard-libraries.md)、[共通資源管理](resource-management.md) に従う。
Windowsを先行し、Linuxで共通処理の検証も行う。任意の外部Rustクレートや独自TLSを追加しない。
v1の正式名・型・引数順は [公開API](public-apis.md)、既定値・上限と所有の失敗契約は [公開契約](stdlib-v1-contracts.md) に定める。

## 初期範囲

TCPクライアント・サーバー、IPv4/IPv6、DNS、期限付きの送受信、WindowsのTLSクライアントを対象とする。
公開APIは同期式とし、GUIではworker内で利用する。UDP、HTTP、WebSocket、TLSサーバー、公開された非同期API、
接続プール、接続済み資源のスレッド移動は後の範囲とする。

公開モジュールは [配置・読み込み仕様](standard-library-loading.md) に従い、TCP/DNSをstd.net、
TLSクライアントをstd.tlsへ分ける。std.tlsはstd.netのTcpStreamと共通の期限型を使う。
std.netだけの利用でtls_client能力やTLSのネイティブ依存を要求しない。

PostgreSQLは [libpqの設計](postgresql.md) に従い、独自のTCP/TLS上でDBプロトコルを実装しない。
libpqのTLSと本ライブラリのTLSでは、信頼する証明書の取得方法などが異なりうることを明記する。

## APIの草案

| API・型 | 契約 |
|---|---|
| IpAddress / SocketAddress | IPv4/IPv6、port、必要なscope情報を保持する値 |
| `resolve(host, port, deadline)` | 所有する候補アドレスを返す。IPv6のscope情報を失わない |
| `connect(target, deadline)` | `Result<TcpStream, NetError>` |
| `bind(address, options)` | `Result<TcpListener, NetError>`。port 0はOSによる割り当て |
| `TcpListener.accept(deadline)` | 所有するTcpStreamと相手のアドレス |
| `TcpStream.read(max_bytes, deadline)` | 空でないDataまたはEof |
| `TcpStream.read_exact(n, deadline)` | 指定量。失敗時には取得済みのバイト列と原因を返す |
| `TcpStream.write(data, deadline)` | OSが受け付けた長さ。短い書き込みを許す |
| `TcpStream.write_all(data, deadline)` | 全量、または確認済みの進捗を含む失敗 |
| `TcpStream.shutdown_write()` | TCPの送信側だけを閉じる。受信は継続できる |
| `TcpStream.close()` | 所有値を消費し、ソケットを閉じる |
| `tls_connect(target, tls_options, deadline)` | 検証済みのTlsStream |
| `start_tls(stream, server_name, options, deadline)` | TcpStreamをムーブしてTlsStreamへ変換する |
| `TlsStream.read` / `write` / `write_all` / `close` | 認証された平文の入出力。生のソケットは公開しない |

max_bytesは正の上限内に制限する。read_exact(0)は空で成功し、空のwriteは0で成功する。
v1のread上限は64 KiB、read_exactは64 MiBとする。
ホスト名とportは別項目にする。portの縮小変換前に範囲検査し、IPv6を単純なコロン分割で壊さない。

## 接続とDNS

WindowsはWinsockの初期化を同期し、SOCKETの幅、INVALID_SOCKET、closesocketなどを内部で扱う。
WSAStartup/WSACleanupを個々の資源のcloseと混同せず、他の接続が生きている間に基盤を終了しない。
Linuxでも同じ公開契約を使い、OS固有のfd・エラー・待ちAPIは内部に閉じる。

DNSの結果を所有する値へコピーしてからOS側の結果を解放する。結果数・名前の長さにも上限を設ける。
国際化ドメイン名は、OSの検証済みの変換でDNS名とTLSの接続先名をそろえる。
末尾のドットやIPリテラルの扱いを仕様にし、証明書検証の名前を勝手に別のホストへ置き換えない。

初期の複数候補への接続は、1つの全体締切の内側で順に試す方式とする。
試行する候補数に上限を設け、最初のアプリケーションデータを送った後は自動で別接続へ再実行しない。
並列のIPv4/IPv6接続競争は別途設計する。

DNSの期限は「呼び出し側が結果を待つ締切」と、OSの処理自体を停止できるかを区別する。
Windowsのキャンセル可能な解決APIを検討し、Linuxなどで停止できないresolverを使う場合は、
上限付きworker・待ち行列・遅い結果の破棄を管理する。放置された要求でthreadやメモリを無制限に増やさない。
キャンセル後もresolverが触る入力・結果領域は、その処理の終了まで保持する。

## 同期APIと内部の待ち

公開APIが同期式でも、接続・送受信は非ブロッキングソケットと待ち処理で進行させる。
Deadlineは共通のClock/Durationから作る、単調増加する時計の絶対時刻とする。短い受信や再試行ごとに全期限を延長しない。

- connectの完了は、書き込み可能になっただけで成功とせず、SO_ERRORなどOSの契約で確かめる。
- WouldBlockと再開可能な割り込みを扱う。EOFと、一時的にデータがない状態を混同しない。
- socketの幅を保ってpoll/selectなどへ渡し、FD_SETSIZEのような制限を無視しない。
- OSの待ち時間の単位変換、丸め、最大値、期限直前の再確認を検査する。
- read/write/acceptの期限超過は明確なエラーにする。期限が過ぎてから長い新規処理を始めない。
- 初期APIはソケットのread/write timeoutだけで全体期限を実装したとは扱わない。

受信の期限切れだけでTCPのバイト列が壊れたとは限らない。初期実装ではreadの期限超過後も、
状態が健全であれば接続を保つ。write_all失敗は進捗と接続の利用可否を返し、元のバッファを先頭から再送しない。
上位プロトコルは途中まで受信したフレームなどを自分の状態として管理する必要がある。

## 所有・別名・状態

TcpStream・TcpListener・TlsStreamは不透明な所有値であり、生のソケット番号は公開しない。
初期段階は送信不可とし、worker内で作成・利用する。サーバーはまず1つのworker内でacceptと処理を行う。
接続を別workerへ渡す並列サーバーは、所有値のスレッド移動を検証してから追加する。

状態はConnecting、Open、WriteClosed、PeerEof、Busy、Broken、Closedを区別する。
read/writeなどは可変借用を要求するが、ref mutは排他的でないため、runtimeでも再入と別名によるcloseを検査する。
OSが番号を再利用しても古い借用を使えないことを [共通資源管理](resource-management.md) で保証する。

start_tlsは元のTcpStreamを消費し、所有の世代を切り替える。
変換前の別名から平文を読み書きしてTLSを迂回できないようにする。
TLSへの変換が失敗した場合、途中で通信したソケットを平文接続として返さず閉じる。

## TCPのバイト列と失敗

TCPはメッセージの区切りを持たない。write1回とread1回が対応するとは約束しない。
read_exactは不足分を読み続け、Eofや期限では取得済みの部分を失わずに返す。
失敗したreadの未初期化部分を公開せず、非空のwrite_allで進捗0が続く場合はNoProgressにする。

OSがsendを受け付けたことは、相手アプリが処理を完了したことではない。
切断後の自動再接続・要求再送は行わない。更新操作の結果の不確実性は上位プロトコルで扱う。
TCPのEOFは相手の送信終了であり、こちらの送信終了とは別とする。

NetErrorはResolve、Refused、Timeout、PeerReset、UnexpectedEof、Tls、InvalidArgument、Closed、Busy、
LimitExceeded、NoProgress、OsErrorなどを区別する。
OSのメッセージだけで分岐させず、操作名・確認済み進捗・接続の利用可否を持たせる。
ペイロード・TLSの秘密情報を自動でログへ出さない。

## TLSクライアント

WindowsはSchannelとOSの証明書検証を第一案とする。
TLS 1.2以上を必須とし、TLS 1.3はOSが対応し実際に検証できた構成で利用する。
プロトコル、暗号、証明書のchain・期限・接続先名の検証を自作しない。
既定でWindowsの信頼ストアを使い、証明書検証を無効にする一般向けのスイッチを初期APIへ設けない。
Linux向けTLSは初期版ではUnsupportedとする。backendの追加は後の範囲として別に設計・検証する。
公開入口はstd.tls.connect/start_tls、明示closeにはDeadlineを渡す。dropでclose_notify成功は保証しない。

- handshakeから通常の送受信まで、SSPIが示す入出力状態と、残りの暗号化バイト列を保持する。
- 不完全なTLSレコードをエラーや平文EOFと決めつけない。複数レコードを1回のreadで受けることも扱う。
- handshakeの再進行・認証情報の寿命・SSPIが返す追加データも管理する。
- バッファはOSが返すサイズ条件と設定した上限で管理し、証明書などの大きなhandshakeで無制限に確保しない。
- TLSのclose_notifyによる終了と、認証された終了を確認できないTCP切断を区別する。
- 初期TLS APIに半切断は公開しない。close時の通知・待ちには期限を付け、dropは通信を待たずに後始末する。
- TlsStreamを閉じたら、認証context・credential・残りバッファ・ソケットを各々一度だけ解放する。

TLSはデータを認証・暗号化する仕組みであり、アプリのメッセージ全体の完了通知ではない。
TLSの送信成功を、DBのcommitやHTTP要求の処理成功と同一視しない。

## 実装と完了条件

能力名の案は `net`、WindowsのTLSは追加の `tls_client` とする。aliceと各実行器の扱いは
[ネイティブ依存](alice-native.md) に従う。任意の外部DLL・関数へアクセスする汎用FFIは公開しない。
共通Cの通信処理をRust/IR/ネイティブ/jwcから利用し、アプリの副作用をデバッグ比較で重複実行しない。

| 段階 | 完了条件 |
|---|---|
| TCP | loopbackでconnect/bind/accept、IPv4/IPv6、port 0、EOF・半切断、短い送受信、接続拒否を検証 |
| DNSと期限 | 偽resolverと実resolver、遅い応答、候補上限、全体締切、キャンセル後の解放と要求数上限を検証 |
| 資源 | move/close/別名、Busy、スレッド制約、番号の再利用、TLS変換前の別名を検証 |
| TLS | 正常な証明書、未知のCA・期限切れ・名前不一致、分割・結合したレコード、切断・close_notify、上限を検証 |
| 障害注入 | handshake・接続・送受信の各位置で切断・期限・失敗を起こし、進捗と解放を確認 |
| 実行器 | 各実行器に独立したサーバーと同じ入力を用意し、結果・エラー・所有を比較 |
| 安全性1〜7 | OS非依存処理・模擬TLSをLinuxのsanitizerで、Windows実APIをWindows CIで検証 |

結合試験はloopbackと使い捨てのサーバーで行い、外部の公開サービスの稼働に合否を依存させない。
TLSの試験用CAは隔離した環境だけで信頼させ、OSの常用信頼設定を恒久的に変えない。
実TLS、模擬実装、OSごとの未確認構成を区別する。この文書は機能の実装完了報告ではない。
