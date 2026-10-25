# 資源型とruntime ABI v1の詳細設計

状態: 実装前の設計。[共通資源管理](resource-management.md) の型・値・検査を具体化する。
ここで予約するABI番号1と型の表現は、現行compilerが対応したという意味ではない。
[alice設定](alice-configuration.md) と [compiler連携](compiler-integration.md) に同じABIを渡す。
個別の操作ID・Args/Outの完全な宣言は [個別C ABI](native-operations.md)、codecと結果の保持は [v1公開契約](stdlib-v1-contracts.md) に定める。

## 対象と既存の型

最初は64ビットのWindowsとLinuxを対象とし、各能力の対応環境は個別に検証する。
32ビット・big endian・外部の任意FFI・動的に追加する資源kindはABI v1の対象外。
既存のown/refの区画、16バイトの世代ヘッダー、task/chanの表現は変更しない。
資源の表と既存の区画の世代は別のものとして管理し、既存ヘッダーのatomicアクセスも維持する。

型検査にResource(kind)を追加し、通常のStructと区別する。
定義元のmodule IDと登録済みkindが型の同一性を決め、importの別名やユーザーの同名structで置き換えない。
公開名はfs.File、net.TcpStreamなどを使い、不透明な型をユーザーが宣言する文法は追加しない。
資源の生成・内部メソッドは、検証した標準/同梱moduleとcompiler内の登録表に結び付ける。
整数・cast・リテラル・構造体のフィールドからtokenを生成する公開APIは提供しない。

| 特性 | Resource(kind)の規則 |
|---|---|
| 所有 | 値自体が所有者。値として読むとムーブ |
| コピー | 所有値の複製を禁止。内部の読み取り用の写しだけを区別して作る |
| 借用 | 既存のref/ref mutを使用。ref mutの排他性は仮定しない |
| 解放 | 所有者のdropだけが解放。読むだけの写しのdropは解放しない |
| 送信 | 初期はすべて不可。内部payloadにRustのSendを自動付与しない |
| 表示 | 型と公開状態だけ。token、OS番号、pointer、接続情報は表示しない |
| 比較・キー | 初期は資源同士の等値・順序比較と辞書のキーを提供しない |

Fileをさらにownで包んでも、外側の区画と内側の資源をそれぞれ一度だけ解放する。
struct・enum・Option/Result・配列・list・辞書の値・クロージャに含む場合も同じ規則を再帰的に適用する。
interfaceは既存のstruct経由の規則を維持し、資源単体に任意のユーザーメソッドやvtableを追加しない。
資源を含むstructをinterfaceの所有値へ包んだ場合は、実際の型のdrop処理まで到達させる。
ジェネリクスの単相化、taskの戻り値、chanの要素でも送信不可を検査する。

Pathや時計の値には、別の登録型OpaqueValue(id)を使えるようにする。
不透明であることとOS資源を所有することを分け、登録した値型のCopy/Drop/Sendとcodecを型ごとに定める。
Pathはencodingと不変のOS byte列を保持し、言語上は複製でき、送信時は所有するデータへコピーする。
Instant/Duration/Deadlineは固定幅の値を複製できるが、利用時にはclockの同一性を確認する。
OpaqueValueの登録も検証した定義元に限り、ユーザーが同名structや任意のfieldから内部値を生成できないようにする。
GUIのNodeIdや入力batchのCopy/Send/世代と利用制約は [GUI内部仕様](gui-internals.md) に定める。
一般のCPU画像・色・座標のrecordは、中身に応じた既存のムーブ・送信規則とAPI境界の検査を使う。

## 所有値・写し・既存の実行器

Rust側はValueに資源専用のvariantを追加し、実行Worldのcontextとtokenを保持する。
現行Valueのderive(Clone)にそのまま所有tokenを載せて、言語上の複製と見なしてはいけない。
内部の一時コピーは、所有を移す操作か、読み取り用のViewを作る操作かを必ず区別する。

has_own相当の「所有するものを含む」判定にも資源を含める。
これにより、資源を取り込んだ関数の値がコピー可能になったり、view経由で所有権が復活したりしないようにする。
releaseは現在の区画ownの列挙だけで終わらず、入れ子の資源所有値の解放も行う。
ただし、Ownの先を処理する責任と、その先のフィールドを処理する責任を重ねて二重解放しない。

view_of、forの要素の写し、借りたmatch、取り込んだ値の読み取りは、資源のOwnerをViewへ変換する。
Viewを可変変数へ代入しただけではOwnerに戻らない。close、commit、TLS変換など所有を消費する操作へ渡せない。
ref mutを静的に確認するだけでは足りない経路は、実行時にも読み取り専用の写しかを確認する。
Viewは資源の寿命を延長する所有者ではなく、失効したときは利用を拒否する。

RustのPortableには資源を転送するvariantを追加しない。
型検査をすり抜けた場合も、export/importとC側のworker入口で検査してからpayloadに触れる。
型引数やinterfaceで内容を証明できない場合は安全側で拒否する。既存の送信可能な値の規則は維持する。

## Tokenの固定配置

ABI v1のC tokenは次で固定する。Cはstdint.hの整数型、Rustはrepr(C)、LLVMは対応する非packed型を使う。

```c
typedef struct {
    uint64_t slot;
    uint64_t generation;
    uint32_t kind;
    uint32_t access;
} JwResTokenV1;
```

| 項目 | offset | 大きさ | 意味 |
|---|---:|---:|---|
| slot | 0 | 8 | 上位32bitがcontext namespace、下位32bitがentry番号 |
| generation | 8 | 8 | 0以外の世代。wrapしない |
| kind | 16 | 4 | 以下の登録番号 |
| access | 20 | 4 | 0=Invalid、1=Owner、2=View。それ以外は拒否 |

全体は24バイト、8バイト整列。全フィールド0は所有しない無効値であり、API成功時の資源ではない。
context namespaceとentry番号の0を予約し、entryの添字は範囲検査後に求める。
slotをpointerへ変換しない。資源表の検査前にnative payloadへ到達するアドレス計算をしない。

| kind番号 | 型 |
|---:|---|
| 1 | File |
| 2 | TcpListener |
| 3 | TcpStream |
| 4 | TlsStream |
| 5 | PostgreSQL Connection |
| 6 | PostgreSQL Transaction |
| 7 | Window |
| 8 | Renderer |
| 9 | Frame |
| 10 | Texture |
| 11 | TextLayout |
| 12 | GuiContext |

0と未登録の番号は拒否し、番号を別の型へ再割り当てしない。
Path、色、アドレス、CPU画像、Clock/Duration/Deadlineはこの資源kindにはしない。
共通の値の入出力は後述の規則で区別し、単に24バイトへ詰めれば共通に扱えるとはしない。

## contextと世代の再利用

資源contextは一つの実行Worldに属し、そのworker群から同じ表を使う。
ASTとIRを独立実行する場合、別のテスト、別の埋め込み実行には別contextを割り当てる。
namespaceはプロセス内で一度だけ発行し、contextを破棄しても再利用しない。
割当て用の単調counterはmutexで同期し、u32の上限で新規contextを拒否する。
同一プロセス内で別のruntime実体のcontext/tokenを交換する構成は初期範囲に含めない。

各呼び出しはcontextを明示的に受け取り、token上位のnamespaceが一致してから表を引く。
RustのValueも同じcontextを保持し、別Worldへtokenだけを渡さない。
同じentry番号・generationを持つ別実行器の資源が、偶然に同じ資源として通ることを防ぐ。

entryにはgeneration、kind、作成threadの内部ID、状態、payload、内部lease、依存先、close結果を持つ。
threadのIDも再利用したOSの番号だけに依存せず、context内で発行した一意の登録IDを使う。
表のmetadataはmutexで守り、native呼び出しの間に表全体の錠を保持しない。
操作するthread・kind・状態・accessを確認し、payloadを内部leaseで保持してから錠を離す。
leaseはユーザーへ返さず、任意のユーザー関数を呼ばない。
通常操作ではnamespaceとslot範囲、accessと登録kindを検査した後、entryのgenerationを比較する。
世代が違えばClosedとし、新世代のkindやpayloadを代わりに利用しない。
世代が一致してからkindの一致・thread・状態を確認する。未知のnamespace/範囲外slotはInvalidToken。
tokenのOwner印は可変借用の証明ではない。wrapperは元のref/ref mutと場所の権限を確認してから、登録された操作を呼ぶ。

slot再利用と権限切替ではgenerationを増やし、MAXではentryを引退させる。
Connection→Transaction→Connection、TcpStream→TlsStreamの切替は、通常の変数ムーブとは別に世代を更新する。
割当て・世代更新に失敗する可能性を外部操作の前に検査し、失敗後に古い別名を復活させない。
COMMITの成功など、既に確定した外部結果を世代更新の失敗でUnknownへ変更しない。

contextを閉じる前にworkerをjoinし、各thread上で資源を回収する。
GUI資源を別のthreadのcontext destructorへ任せない。
通常終了のlive owner・entry・内部lease・隔離資源を別々に数え、既存の区画数だけでリークなしと判定しない。
exitや実行時エラーで_Exitする既存の経路に、必ずdropが走るという保証は追加しない。

## C呼び出しの規則

外部境界はC calling convention、固定幅の整数、context pointer、初期化済みの入出力領域を使う。
tokenなどのaggregateを値引数や戻り値にせず、pointer経由に統一する。Win64とSysVのaggregate ABIの差を避ける。
Cのbool・long・size_t・enumを共有表現として使わず、幅と意味を明示する。

代表的な共通入口は次の形とする。型・関数名も新設計であり、現在のsymbolではない。

```c
int32_t jw_res_context_new_v1(uint32_t abi, void **out_context);
int32_t jw_res_require_abi_v1(uint32_t abi, const uint8_t *registry_sha256_32);
int32_t jw_res_context_finish_v1(void *context);
int32_t jw_res_close_v1(void *context, JwResTokenV1 *inout_owner);
int32_t jw_res_drop_v1(void *context, JwResTokenV1 *inout_owner);
```

呼び出し側は全out領域を0にし、calleeも出力の全フィールドを初期化する。
context_newの失敗ではout_contextはnull。contextの途中作成失敗も回収する。
require_abiは32バイトの登録表digestを確認し、資源の初期化前に版と登録情報の不一致を拒否する。
finishはentry/leaseの残りを確認してcontextを消費する操作で、同じpointerで再試行しない。
正常な呼び出しの前提は、すべての公開所有値を処分し、workerと実行中の操作を回収済みであること。
違反時もnative payloadへ無理に触れず、診断と隔離を行い、全回収した成功として返さない。
隔離資源が残る場合はその事実を返し、正常に全解放した結果へ変えない。関連する隔離metadataはruntime側に保持する。
未完了leaseがある不変条件違反では、内部処理が参照するcontext自体も保持し、caller側の所有だけを失効させる。
metadataを残すことと全解放の成功を区別し、entryや操作が参照する表を先にfreeしない。
検証済みの生きたcontext pointerだけを内部callerが渡すことは前提条件で、任意の不正なpointerを安全に検査できるとはしない。

close/dropは所有を受領した時点でinout_ownerを全0へ移し、以後のエラーでも呼び出し元が再解放しない。
Viewをclose/dropの所有入口に渡した場合はWrongAccess。Viewの通常の破棄は入口を呼ばず、写しだけを捨てる。
無効な全0値のdropは何もしない。親のclose等で既に失効したOwnerのdropもnativeを再解放しない。
通常の操作・明示closeからの利用失効はClosedとして報告できるが、種類・namespace・accessの破損は別に扱う。
generationが一致しないtokenから過去のpayloadを復元したり、現在のentryを代わりに閉じたりしない。

共通statusは0=Ok、1=OperationError、2=Closed、3=Busy、4=LimitExceeded、5=Unsupported、
6=InvalidArgument、7=AbiMismatch、8=InvalidToken、9=WrongKind、10=WrongThread、11=WrongAccess、12=InternalErrorとする。
ABI/kind/thread/所有の不変条件違反は通常のOS失敗と混同せず、言語の実行時診断へ変換する。
OS失敗・進捗・DBのoutcomeなどは、各操作の型付きの出力に保持する。status番号だけで再試行や未実行を判断しない。
個別操作の引数と出力は同じ規則の版付き宣言を登録表で持ち、未定義な汎用voidポインタ引数列にはしない。
通常の個別操作は専用Args/Outへのpointerを使い、複合値のsliceも宣言型ごとのcodecを必須とする。
Outのvalid/progress/outcomeと所有slotを使い、statusが非0でも完全なTxEnd等の結果を失わない。

呼び出し中だけ借りるbyte列はconst pointerとu64長を使い、長さ・null・確保範囲・OSの幅への変換を確認する。
calleeは借りたpointerを保持しない。出力のbyte列はcaller所有の領域か、runtime所有のbufferと専用解放関数で返す。
出力をjabberwockの所有する値へ移してからbufferを解放し、DLLや別CRTの領域をRustのfreeへ渡さない。
エラーで一部データが有効な操作は、有効範囲と進捗を出力へ明示し、未初期化領域を読む規則にしない。

## 共通の値とbuffer

文字・byte列の入力はborrowedなpointerとu64バイト長を別引数で渡す。
長さ0ではpointerがnullでも読まない。長さが正なら有効な領域を必要とする。
文字列用の入口では厳密なUTF-8、byte列用では全byte値を許し、この二つを同じ文字列変換にしない。
このpointerはcompilerが検証した値から作る内部表現であり、任意の不正pointerをCが安全に読み出せる保証ではない。

返すbufferはruntimeのcontextとallocatorに属する。pointer・u64長・u64確保長を出力し、
専用のbuffer_release_v1へ同じcontextと返された組を一度だけ渡す。
成功・部分失敗とも、有効な長さは確保長以下を必要とし、空の解放済み表現は全0にする。
allocation失敗は全0、部分結果は初期化した有効範囲だけを返す。
callerは値へのコピー・変換の途中で失敗してもbufferを解放するguardを持つ。Rustの自動freeへ直接渡さない。
caller所有領域へ書く入口は、容量と実際に書いた長さを別に持ち、要求量まで初期化済みだと推定しない。
明示したFile.readの上限などは確保前に確認する。

Pathの境界はencoding番号、borrowed bytes、バイト長を別引数で持つ。
1=Windows UTF-16LE、2=LinuxのOS byte列とし、対象OSと違うencodingを拒否する。
Windowsは偶数長を必要とし、NULは16bit code unit単位で検査する。byte単位の0でASCII名まで拒否しない。
byte列を未整列のwchar pointerへcastせず、整列と終端を持つOS用領域へ検査してコピーする。
OSから列挙した不正surrogateや非UTF-8の名前も損失なく保持し、to_textが成功するとは限らない。
Linuxはbyte値0を拒否する。診断用のdisplayをPathとして開き直さない。

Clockはcontext namespaceに対応する0以外のu64 clock_idを使い、同じWorldのworkerで共有する。
Instantはclock_idとu64の経過nanosecondの組、Durationはu64 nanosecondとする。
時計の基準からの値はOS counterからあふれを検査して変換し、丸めで期限を延長しない。
OSの待機から戻るたびに絶対期限を再検査する。OSのスケジューリングを含む厳密な応答時刻の保証とは区別する。
Deadlineのメモリ表現はclock_id:u64、nanoseconds:u64、mode:u32、reserved:u32の順で24/8。
mode 0は無期限で他の全フィールド0、mode 1は指定clockの有限期限、reservedは常に0とする。
異なるclock、未知のmode、加算・単位変換のあふれをInvalidArgumentとして拒否し、反復で期限を延長しない。

SocketAddressの共通表現はfamily:u32、port:u16、reserved:u16、address:u8[16]、scope:u32、reserved2:u32の順で32/4。
familyは4か6、portはhost順、addressはネットワーク順のbyte列、reservedは両方0。
IPv4は先頭4byteを使い残りとscopeは0、IPv6は16byteとscopeを保持する。
OSのsockaddrへ直接castせず、family番号・幅・byte順をOS側の構造体へ変換する。
これらの配置も生成宣言とlayout試験へ含める。
描画やDBの型付き出力は各操作の宣言で定め、初期化されていないunionを汎用bufferとして公開しない。

## LLVM・IR・解析への反映

LLVMの資源値は`%jw.res.v1 = type { i64, i64, i32, i32 }`とし、型のlayout計算へ24/8を追加する。
資源を含むstruct・enum・配列・関数環境もlayoutを再計算する。Optionの未実装なniche最適化を仮定しない。
aggregate内部で値を移すことは可能だが、C境界では必ず初期化したメモリを経由する。
失敗後の非活性enum領域や余白を、memcmp・表示・転送のために読むことを禁止する。

IRには登録済みの操作IDと型付き引数・結果を持つ資源操作を追加する。
Move、Copy/View、Drop、Discard、return、?、takeの責任を保持し、所有tokenを暗黙に複製しない。
操作IDのmetadataは引数のMove/ref/ref mut、所有の返却、失効、副作用・外部入力を定める。
ASTとIRとLLVMでこのmetadataを同じ版の登録表から使い、関数名の文字列だけで操作を推測しない。

初期は資源操作を解析上の保守的なbarrierとし、呼び出し・close・変換で失われる可能性のあるrefの事実を消す。
既存の区画の世代検査を消せても、資源のnamespace/kind/generation/state/thread検査は省略しない。
--no-elideでも同じ資源検査を行い、--releaseで安全性の検査を外さない。
MSanでC宣言に必要なnoundefを維持し、未初期化のtoken・out引数も負例で検出する。
LLVMのnoaliasやnonnullなどの属性を、ref mutの非排他性や失敗時nullと矛盾する形で付けない。

## 共通コードと導入順

版付きの宣言・kind・操作ID・layout・効果の登録表を一つ持ち、Rust/jwc/Cの対応情報を生成する。
生成されたファイルは手で別々に更新しない。登録表の内容hashをcompiler連携にも含める。
既存のruntime.jwは現在どおり生成物とし、新Cソースを追加した場合も元ソースとの一致を検証する。
Rust版へのCビルド・接続と、生成nativeへの同じC接続を用意し、二つのOS実装を独立に作らない。

1. 模擬資源一種類でcontext、token、Owner/View、drop、失敗、thread制約を実装する。
2. Rustの型・Value・入れ子の解放とIR実行を対応させる。
3. LLVMとCを対応させ、各境界のlayout・所有・出力初期化を確認する。
4. jwcのTy/型検査/lower/LLVMを同じ登録表へ対応させる。
5. Fileを接続し、独立した作業領域で各実行器とaliceのビルド指紋を比較する。
6. GUI・通信・PostgreSQLへ進み、それぞれの依存・変換・失敗状態を試験する。

これらは段階を分けた実装順であり、途中の成功をABI全体の完成としない。

## 完了条件

| 検証 | 必須の確認 |
|---|---|
| 配置 | Cのsizeof/alignof/offsetof、Rustの対応値、LLVM datalayout、jwc出力が24/8と各offsetに一致 |
| 文脈 | 別World・別実行器・終了済みcontextのnamespace、同番号entry、thread ID再利用、上限での拒否 |
| 所有 | Move/View、for/match/closure、代入・return/?・入れ子のdrop、写し経由のclose拒否 |
| 状態 | slot/OS番号再利用、MAX引退、Busy、親close、device世代、TLS/transaction変換 |
| 境界 | 全status、部分結果、確保失敗、out未初期化、幅・null、allocatorの対応、ABI不一致 |
| 送信 | struct/enum/list/map/own/interface/task/chanの抜け道を拒否し、runtimeでも検出 |
| 解放 | owner/entry/lease/隔離を別計上し、context終了、各threadの終了、二重解放と失敗を検証 |
| 実行器 | Rust/IR/native/jwc、解析あり/なし、debug/release、通常実行の副作用一回 |
| 安全性 | 固定攻撃・生成入力、LLVM verifier、Linux ASan/UBSan/MSan/TSanとWindows実API |

未実施構成や模擬backendだけの成功を実OSの成功に数えない。
既存の安全性1〜7の合格を、新資源ABIへの自動的な合格として流用しない。
