# 標準ライブラリ v1の公開契約とC境界

状態: 実装前の設計。初期範囲の型・引数順・返却型・操作IDは [registry.json](stdlib-v1/registry.json)、
読みやすい一覧は [公開API](public-apis.md)、C宣言は [個別操作](native-operations.md) と [ヘッダー](stdlib-v1/stdlib-abi-v1.h) に定める。
既存の個別設計に残る草案の名前・省略表記より、このv1の具体的な宣言を優先する。
現在のcompiler/runtimeがこれらを実装した、またはCの関数本体を提供したという意味ではない。

## 型・モジュール・エラー

公開moduleはstd.fs、std.time、std.net、std.tlsと、選択式のwindow、draw2d、image、gui、postgres。
WindowError、DrawError、ImageError、GuiErrorをそれぞれ定義し、windowがguiへ依存する循環を作らない。
TLSはstd.net.NetErrorを利用する。geometryとCPU画像を理由にimageからD3Dを要求しない。
image.CpuImageはwidth/height/stride/pixelsを持ち、draw2dのsizeへは明示的に変換する。
guiのTextStyleはdraw2d.TextStyle、GUIのboundsはdraw2d.Rectを使う。

record/enumのfield名・順序・variantはregistryを正本とする。通常のstructの既存のムーブ規則を維持する。
不透明な値は登録したCopy/Sendを使い、resourceは全て!Copy/!Send。
Viewを可変変数へ置いても所有者へ昇格させない。native呼出し前にも引数の権限を検査する。
namespace/generation/kind/threadの検査と、通常のOS失敗として返すdomain errorを混同しない。

Error recordのoperationは登録された安全な操作名、os_codeは存在する場合だけの補足情報。
OSの自由なmessage、SQL、password、clipboard、入力textを既定の診断へ混ぜない。
SQLSTATEはPgError.sqlstateへ保持し、nativeErrorだけから架空のSQLSTATEを作らない。
strやbyte列は所有する値として返し、外部APIの一時bufferへrefを返さない。

## ファイルとPathの確定事項

OpenOptions.defaultsはread=true、write=false、creation=Existing、append=false、windows_share=None。
WriteModeは明示必須とし、既定で切り詰めるfactoryを提供しない。
Windowsのshare=Noneはread/write/deleteを共有する。Linuxで明示WindowsShareを指定した場合はUnsupportedとし、
POSIXのlockと同じだと説明しない。読み書き権限の矛盾・appendとseekを呼出し前に拒否する。
作成・新規のみ・切り詰めはOSの一操作で実現し、存在確認と作成を分けない。
NoReplaceのrenameを原子的に実現できない対象ではUnsupported。別filesystemへのcopy/deleteへ切り替えない。

whole readのlimitはbyte数で1〜67108864、read_dirのlimitはentry数で1〜65536。
read_dirのPath等の合計にも64 MiBを上限とし、件数だけで無制限に長い名前を確保しない。
File.readは1〜1048576 byte。一度のreadが短くてもEOFと決めず、EofはDataと別のvariantにする。
writeの空入力は0、write_allの空入力は成功。非空で進捗0ならNoProgress。
write/write_all/write_bytesの失敗はWriteFailureに確認済みwritten、追加書込みの不確実性、対象変更の可能性を保持する。
0 byteでもcreate/truncate済みならtarget_may_have_changedはtrue。失敗を元に戻せたとはしない。

Path.joinは絶対childを指定した場合はchildを採用し、字句的な結合とcanonicalを区別する。
absoluteは現在の作業ディレクトリに基づき、canonicalは実在対象をOSで解決する。
どちらもsymlink/reparse pointや権限を文字列処理だけで検証できたとは扱わない。
Metadata.modifiedはUnixTime(seconds:i64,nanoseconds:u32)で表し、nanosecondsは0〜999999999。
取得できない時刻はNoneとし、nanosecondをi64へ丸めて範囲を狭めない。
read_dirのkindは取得時点で不明ならNone、metadataのFollow/NoFollowは明示する。
flush/sync/closeの保証は個別設計どおり。closeで所有を消費し、失敗時にも同じ番号へ自動再試行しない。

## 時計・TCP・TLS

時計の公開入口はstd.time.now/elapsed/deadline_after/deadline_atとし、従来の文書中のClock.nowはこの入口を指す。
nowはResult<Instant,TimeError>を返す。Durationの単位はnanosecond、加算・単位変換のあふれは拒否する。
Deadline.infiniteと有限期限を区別する。clock違いと過去へ戻る時刻を黙って補正しない。
新しい値codecを使う操作もABI 1を要求し、OS資源を返さない時計操作だからABI検査不要とはしない。

net.TargetはAddressまたはHost。resolveは最大64候補、connectは一つの全体期限で順番に試す。
readは1〜65536 byte、read_exactは0〜67108864 byte。失敗時のpartialをReadFailureで返す。
write失敗はOSが受け付けたwrittenと追加書込みの不確実性を返し、相手が受信・処理した証拠とはしない。
BindOptionsの既定はbacklog=128、reuse_address=false、ipv6_only=true。backlogの指定範囲は1〜65535。
IPv4ではipv6_onlyは適用しない。Windowsは既定で排他的bindを使い、初期のreuse_address=trueはUnsupported。
Linuxのreuse_address=trueはSO_REUSEADDRであり、reuseportや複数serverの同時bindを提供するものではない。
送信不可のlistener/streamはworker内で作る。acceptしたstreamを別workerへ送れると説明しない。

TLSの公開名はstd.tls.connect/start_tls。ServerNameはDnsまたはIpで明示する。
OSの信頼storeと接続先名検証を必須とし、TLS 1.2を既定の最低版とする。Tls13要求を未対応OSで弱めない。
SNIはDNS名の場合だけ使い、IPは証明書のIP情報で検証する。
start_tlsはTCPを消費し、失敗なら閉じる。元の借用をplaintext経路として残さない。
TlsStream.closeはDeadlineを明示し、close_notifyの試行とsocket解放を行う。期限失敗でも所有値を返さない。
dropは非待機の解放であり、TLSの正常終了通知を送信できた保証ではない。
Linux TLSのbackendは初期版ではUnsupportedと確定し、未選定の暗黙fallbackは設けない。

## PostgreSQLの初期adapter

初期clientはlibpq 18.xの対応版に絞る。server 16/17/18は結合試験の候補とし、成功した構成だけを対応済みにする。
以前のlibpq 16/17という候補は初期adapterの対象から外し、追加時に同じ互換・TLS・既定値試験を必要とする。
ConnectOptionsはhost/port/database/user/password/security/deadlineを全て明示する。
初期のpasswordは非空のUTF-8で、NULを拒否する。認証はrequire_auth=scram-sha-256に固定する。
VerifyFullはCAファイルを明示し、sslmode=verify-full、ssl_min_protocol_version=TLSv1.2、channel_binding=require。
LoopbackPlainは数値loopbackだけに限定し、sslmode=disable、channel_binding=disable。DNS名からloopback例外を推測しない。
client certificate、GSS、service、複数host、passfileを選ぶ公開オプションは初期版に設けない。

接続を始める前にPG*環境変数が存在すれば空でもConfigurationとして拒否し、プロセス環境を書き換えない。
Windowsは環境名の大文字小文字をOSの規則で扱う。値自体を診断へ出さない。
host等をPQconnectStartParamsへ個別指定し、dbname展開を無効にする。sslcertmode=disable、gssencmode=disable、optionsは空に固定する。
非空passwordによってpassfile lookupを避け、serviceを環境やファイルから選ばせない。
埋め込みhostが同時に環境を変更しないことは前提条件。接続後もTLSとUTF8を検査する。
CA等のPathがlibpqの要求するOS文字表現へ損失なく変換できなければConfigurationとして拒否する。
libpq/OpenSSLが既定のroot.crlを参照し得る点は残る。初期版はCRLによる失効確認を必須として成功保証するAPIではない。
この既定値・PG環境・認証・TLSの組合せはlibpq 18のソースと実結合試験で版ごとに確認する。

QueryOptions.defaults(deadline)は100000行、1024列、67108864 byte。
指定の最大は1000000行、4096列、268435456 byteとし、byte予算には列名・cell・行等のcodec上の構造量も含める。
libpq内部の一行分の確保まで、この予算で制限できるとはしない。
ParamはNull/Bool/I64/F64/Text/Bytes。TextとSQLはNULを拒否し、Bytesは0 byteも保持する。
F64のNaN/InfinityはPostgreSQLの表現へ明示変換し、UIの有限座標の検査とは分ける。
column_indexは完全一致で検索し、無しはNotFound、重複はAmbiguous、Command結果はType。
Cell.as_*は明示変換で、NULLや種類・範囲の違いをTypeとして返す。numericを暗黙にf64へ変換しない。
as_boolはt/f、as_i64は符号付きASCII十進整数、as_f64はASCII十進/指数とPostgreSQLのNaN/Infinity表記を扱う。
localeの小数点・桁区切り・前後の空白で解釈を変えない。as_text/as_bytesも指定したCellの種類だけを受け付ける。

beginはConnectionを消費し、失敗時は安全な場合だけBeginFailure.connectionに返す。
commit/rollbackはDeadlineを受けてTransactionを消費し、常にTxEndを返す。
outcome、再利用可能なConnection、errorを独立に扱い、errorがあっても既知のCommittedをUnknownへ変更しない。
結果が不明ならUnknown、connection=None。preflight失敗後にtransactionを閉じたことを、rollback応答の確認と偽らない。
実行状態NotSent/MayHaveRun/Completedも保持し、通信・期限・serialization failureからSQLを自動再送しない。

## Window・描画・画像・GUIの確定事項

Window.Options.defaultsは1280×720 DIP、titleは空、resizable=true。
Window.info等もWindowErrorのResultとし、Windowの失効を成功した情報で隠さない。
draw2dのRendererは既定HardwarePreferred、明示WarpとHardwareRequiredを区別する。
FrameStartはReady(Frame)かSkipped(reason)。finish/abortは所有を消費し、Presentはfinishだけが行う。
PresentOptions.defaultsはvsync=true。初期版はtearingの公開指定を設けない。
command数65536、clip/transform深さ128、command領域16 MiBを上限とする。
Textureは正の寸法かつdevice上限以下。strideは省略時width×4、入力はstride×heightを必要とし、paddingも所有入力内にあることを確認する。
TextureOptionsの既定はLinear/strideなし/64 MiB、max_bytesは256 MiB以下。
Frame.imageのsourceはtexture pixel座標、targetはDIP。source省略は全体。tintは明示し、白なら変色しない。
TextOptionsはstyle/max_width/max_bytesを明示し、textは1 MiB以下。TextStyle.defaultsはSystemUi、OS size、Normal、NoWrap。
WidgetSpecのstyle=Noneはkindの既定を使い、LabelはWordWrap、Button/TextInputはNoWrap。
Some(TextStyle)を明示したときはその値を尊重し、TextInputへのWordWrapは拒否する。

imageはWindows WICでPNG/JPEGの単一画像をRGBA8・straight alpha・sRGBへ復号する。
Microsoft vendorのPNG/JPEG codecを選び、任意の追加codecを優先して自動選択しない。初期JPEGはRGB/grayscaleを対象にする。
EXIF orientationを反映し、ICC情報があれば色変換する。不正なprofileや未対応の複数frame/APNGを黙って無視しない。
Limits.defaultsはencoded 16 MiB、16777216 pixel、decoded 64 MiB。指定上限もそれぞれ64 MiB/67108864 pixel/256 MiB以下。
OS codec内部の確保をプロセス全体の厳密な上限としない。Linuxの実画像decoderは初期版ではUnsupported。
loadはfs経由の読取りを伴う。decodeだけでも同じimage packageを読む能力単位は維持する。

GUIは [内部仕様](gui-internals.md) に従う。rootもResult<NodeId,GuiError>へ統一する。
SemanticsやLayoutのdefault factoryはregistryの値を返し、TextInputに必要なnameを勝手に補完しない。
GuiOptionsにrendererを隠して所有させない。GUI未使用のWindow/描画にはTSF/UIAのDLL依存を加えない。

## 型付きC呼出しとcodec v1

C関数は各操作ごとに専用のArgs/Out型を持ち、`int32_t symbol(void *context, Args *inout_args, Out *out)`で呼ぶ。
context以外に未定義のvoid引数列は設けない。Argsのfield順・offset・大きさは生成ヘッダーのassertを正本とする。
scalarは固定幅のC整数/double、bool入力はu32の0/1。resourceは24/8のtoken、それ以外の複合値はJwSliceV1。
JwSliceV1はconst pointer/u64 lengthの16/8、JwBufferV1はpointer/u64 length/u64 capacityの24/8。
複合値をsliceにする場合も、fieldごとの型とcodecは登録表で固定し、任意の別型として解釈しない。

codecはlittle endian、paddingなしで次の順にencodeする。

| 型 | encoding |
|---|---|
| unit/bool | unitは0 byte、boolはu8で0/1 |
| 整数/f64 | 宣言幅。signedは2の補数、f64はIEEE 754のbit列 |
| str | u64 byte長と厳密なUTF-8。NUL等は操作の制約で追加検査 |
| list<T> / array<T,N> | listはu64要素数の後に各要素。固定配列は要素だけ |
| record | registry.fieldsの順で各fieldをencode |
| enum | registryのu32 tagと、そのvariantのfield列 |
| Option / Result | Optionは0=None/1=Some、Resultは0=Ok/1=Err。言語内部のenum番号と同じとは仮定しない |
| opaque | value_id:u32、access:u32、registryのprivate field列。Copy値はaccess=0、move値はOwner=1/View=2 |
| resourceの戻り値 | u32の所有slot番号。token本体はOutのresource_Nに置く |

moveのopaque引数はOwnerを必要とし、Viewを同じbyte列として再所有させない。GameInputBatchとEventBatchもvalue_idで区別する。
nativeへ渡す前にcompiler/runtime側の型・権限を検査し、戻った値でもvalue_id・context/clock/window・seal/revisionを検査する。
resourceのslotは実際に選んだvariantをfield順に辿って0から発行する。Kind/Ownerを宣言型と照合し、未使用slotは全0。
record内のresourceの数は和、enum/Option/Resultでは同時に必要な最大数とする。
初期の公開境界に可変長resource配列は設けない。必要なら将来の版で新しい所有プロトコルを定める。

decoderは全入力を消費し、余分なbyte・未知tag・不正bool・長さの加算乗算・depthを検査する。
codecの固定上限は256 MiB、深さ64、単一list 1048576要素。操作ごとの小さい予算を先に適用する。
pointerの正当性は内部callerの前提であり、任意の破損pointerを安全にdecodeできるという保証ではない。
byte列・recordをそのままCのstruct/unionへcastしない。LLVMのaggregate ABIにも依存させない。

## 所有・失敗・結果を失わない規則

Outの先頭はJwOutcomeV1で48/8。

| field | offset | 意味 |
|---|---:|---|
| valid:u32 | 0 | 0=型付きvalueなし、1=完全なvalueあり |
| effect:u32 | 4 | 0=未開始、1=変更し得る、2=完了確認済み。受領後の解放も考慮 |
| progress:u64 | 8 | 入出力で確認済みのbyte数。相手の処理完了数ではない |
| outcome:u32 | 16 | 0=非transaction、1=Committed、2=RolledBack、3=Unknown |
| reserved:u32 | 20 | 常に0 |
| value:JwBufferV1 | 24 | 完全な公開戻り値のcodec。allocatorとcontextを保持 |

その後に24 byteのresource slotを必要数置く。全out/Argsとpaddingを呼出し前に0初期化する。
statusは資源ABIの0〜12を使う。valid=1ならstatusだけで戻り値を捨てず、宣言した型をdecodeする。
特にTxEndはResultではなく、errorを含んでも既知のoutcomeを保持する。
headerのprogress/outcomeと型付きpayloadの記述を照合し、不一致を内部エラーとして拒否する。

外部効果の前に、少なくとも型付きの失敗結果を返す領域と必要なslotを予約する。
成功結果の確保・encodeに失敗しても、確認済みの書込み数やCOMMIT結果を失敗の一般文面へ潰さない。
variableな成功結果に失敗した場合は予約したerrorを返し、PGでは実行状態と接続の利用可否も保つ。
caller側のdecodeが失敗しても、headerにある確認済み結果をUnknownへ書き換えない。
全プロセスのメモリ枯渇時に、必ずアプリの新しいResultを確保できる保証ではない。その場合も虚偽のrollback等を出さない。

valid=0の通常失敗は所有受領前のpreflight失敗だけ。Move resource fieldは元のArgsに残す。
caller guardは既存の失敗契約に従い、beginなら安全なConnectionを返せ、TLS変換/finish/closeなら所有値を処分する。
その処分の効果をheader/診断へ反映し、transactionを処分しただけでRolledBackとはしない。
受領した後はcalleeがMove fieldを全0へ移して責任を持ち、失敗でも二つ目の所有者を残さない。
borrowedなresource fieldは消費・zero化しない。mutable操作の権限をOwner印だけで証明しない。

callerはOutの全所有slotとbufferのguardを持ち、言語の値への採用完了後だけ責任を移す。
途中のdecode失敗では採用していない資源・bufferを一度だけ解放する。
bufferはjw_buffer_release_v1へ同じcontextと組を渡し、解放後は全0。別CRTやRust allocatorでfreeしない。
password等のscratchは所有可能な範囲で消去するが、言語上の全str複製を消去できるとは保証しない。
ABI不一致・破損・wrong thread等の実行時停止は、既存_Exit経路の解放保証を変更しない。
ビルド側がruntimeの接続設定を追加収集しないことと、source literalや既存のdebug用source埋込みから秘密を自動削除することは別である。

複数resourceの操作は表の錠で一括検査してleaseを取り、OS呼出し前に錠を離す。
内部から同じ公開操作を再入してBusyを回避するのでなく、取得済みleaseを使うprivate処理を呼ぶ。
namespace・世代・thread・権限検査を最適化で消さず、LLVMへnoalias等の不成立な属性を追加しない。

## 初期版の検証境界

registryの参照・ID・所有slot・型・依存・default値と生成物の一致をscripts/stdlib_design.pyで検査する。
Cヘッダーの配置assertはLinux x64のGCCで確認し、Windows x64/MSVC ABIのclangも実装時に同じassertを通す。
これはC関数本体、OS I/O、IME、GPU、PostgreSQLの試験ではない。
実装時はRust/IR/native/jwc、解析あり/なし、debug/release、安全性1〜7と失敗注入・外部結合をそれぞれ満たす。
未実施の構成、Unsupported、条件不足を合格へ足さない。
