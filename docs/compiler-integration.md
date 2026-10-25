# alice・Rust版・jwcのcompiler連携プロトコル v1

状態: 実装前の詳細設計。現在のjwcには本プロトコルもaliceのパッケージ対応も未実装。
[alice設定](alice-configuration.md)、[読み込み仕様](standard-library-loading.md)、
[資源ABI](resource-abi.md) を、同じソースsnapshotと登録情報で各実行器へ渡す契約を定める。
製品の版、manifestの形式、資源ABI、以下の連携プロトコルの番号を混同しない。

## 責任の分担

| 処理 | 主な責任 |
|---|---|
| 設定・依存・標準配布の選択 | alice。利用者設定や元のソースを一度解決する |
| 読み込みグラフとsnapshot | aliceの共通loader。module ID、公開範囲、直接依存、能力を確定する |
| 字句・構文・型・所有・IR・LLVM | 選択compiler。渡された情報を検証し、登録された操作だけを扱う |
| C/runtime・SDK・native依存・リンク | aliceの共通backend。compiler別に別実装を持たせない |
| AST/IRでの実行 | Rust版。対応する資源contextと同じsnapshotを使用する |
| nativeの実行・test・出力公開 | alice。失敗・条件不足・未検証を区別する |

jwcは初期はLLVM生成を担当する。jwcを選んだことでAST/IRを実行できるとは扱わない。
alice経由では既存のjwc buildを呼ばず、LLVMを得た後のCビルド・sanitizer・リンクをaliceが一括で行う。
単体のjwc buildは別の既存入口として維持し、同じbackend設定を利用できた段階で新機能を対応済みとする。

## compiler情報の取得

新しい外部入口は`jwc --compiler-info`とする。
成功時はstdoutへUTF-8 JSONオブジェクトを一つ、最後に改行を出し、0で終了する。
stderrは診断用であり、stdoutに進捗や通常のテキストを混ぜない。
Rust版は組み込みAPIで同じ構造を返す。単体jabberwockにも同じ外部入口を設ける。

| フィールド | 型と意味 |
|---|---|
| info-version | 整数1 |
| compiler | rustまたはjwc |
| compiler-version | 製品の版の文字列。これだけで互換と判断しない |
| protocols | 対応する連携プロトコル番号の配列。初期は実装済みなら[1] |
| operations | 実装済みの登録操作IDのu32配列。重複・未知IDを拒否する |
| capability-targets | fs等の能力名から実装・検証済みtarget配列への対応 |
| modes | 実装済みのast、ir、llvmの配列。jwcの初期対応はllvmだけ |
| host | compiler実体が動作するtargetの登録名 |
| targets | このcompilerが生成を実装・検証したtargetの登録名の配列 |
| resource-abis | 実装済みの資源ABI番号の配列 |
| registry-sha256 | 型・kind・内部操作・layout・効果の登録表の内容hash |
| runtime-sha256 | この配布に対応する共通runtimeソース集合の内容hash |
| stdlib-build-id | 対応する標準配布の内容ID。標準配布なしならnull |
| default-sysroot | 既定配布ルートのOSパス表現、またはnull |

hashは64桁の小文字hexとする。OSパスの表現はalice設定のdisplay/encoding/hexに合わせる。
default-sysrootは要求に応じた自動探索の結果ではなく、そのcompilerの配布情報を返す。
--sysrootを利用者が明示した場合は、そちらのカタログとcompiler側の対応情報を照合する。
情報取得のために標準コード、ユーザープログラム、SDK、libpqを実行・ロードしない。

aliceは実体のファイル内容hashも自分で計算し、compilerの自己申告だけでバイナリを同一と判定しない。
未対応protocol、mode、target、登録表、runtime、ABIの不一致では停止する。
未知のプロトコルを番号の近さで互換と見なさず、Rust版へのfallbackもしない。
info取得の上限は64 KiB、待ち時間は15秒とし、超過や不正JSONを利用不可として診断する。
同じ配布状態のinfoはキー順・表記・末尾改行を固定して出力し、requestにあるhashと照合できるようにする。
選択から実行までに配布情報が変わった場合は不一致として止め、別内容のinfoを黙って採り直さない。

## snapshot bundle

aliceは出力の錠を取得し、OSの一時領域に他のjobと衝突しない作業ディレクトリを作る。
POSIXの権限とWindowsのACLで、その利用者の作業領域として扱う。
権限を確保できなければ、安全な一時領域を作れたと仮定して続行しない。
作業ディレクトリの選択・清掃はaliceが担当し、compilerに任意の出力先を選ばせない。

```text
job/
  request.json
  sources/s00000000.jw
  sources/s00000001.jw
  manifests/p00000000.toml
  manifests/p00000001.toml
  output/program.ll
  response.json
```

元ソースのパスを作業用ファイル名に流用しない。名前はaliceが発行するASCII IDから作る。
ファイルはUTF-8の元のバイト列を保ち、BOM・CRLF・改行末尾をhash前に正規化しない。
各ソースを読んだ一つの内容をグラフ・hash・診断・実際のコンパイルへ使う。
元ファイルの後からの再読み込みや、worker側での別ディレクトリ探索をしない。

requestはsnapshotファイルの名前・長さ・SHA-256を列挙し、compilerは上限付きで一度読み、hashを検証した内容を保持する。
symlink/reparse point・絶対パス・親への移動でjob外を参照するbundle内パスを拒否する。
検査後に別のファイルを開き直さず、保持した内容を解析する。
hash不一致や読取り失敗では、元ソースから補完しない。
診断用の元OSパスは読み取り用のパスとして使わない。

request.jsonは64 MiB、response.jsonは8 MiB、LLVM出力は256 MiBを固定上限とする。
ソースとmanifestにはalice設定の個別・合計・数・深さの上限も適用する。
大きい設定上限が許されていても、連携形式の上限まで必ず生成できるとは約束しない。
compilerは上限超過を明確に返し、途中のLLVMを成功した成果物にしない。

## requestのschema

JSONはUTF-8とし、重複キー、未知のキー、未知の版、型違い、整数の範囲外を拒否する。
数値は必要な幅の整数として解析し、f64経由でIDや長さを丸めない。
各配列の要素数、文字列の長さ、JSON自体の深さも、読み込み前・読み込み中に検査する。
初期はJSON深さ32、単一文字列64 KiB、診断数4096とし、ソース本文をJSON文字列へ埋め込まない。

requestの最上位キーは次とする。

| キー | 内容 |
|---|---|
| protocol | 整数1 |
| job-id | jobごとの128bit以上の一意なIDを小文字hexで表す |
| compiler-info-sha256 | 選択時に取得したcompiler情報JSONの内容hash |
| entry | 入口のmodule ID |
| options | target、profile、emit、elide、sanitizer、entry-function |
| requirements | resource-abi、registry-sha256、runtime-sha256、stdlib-build-id |
| limits | 有効な読み込み上限。compiler自身の固定上限を超えて引き上げない |
| packages | パッケージID、名前、依存元、manifestのsnapshot、直接依存、要求する版・能力 |
| modules | 以下のmodule記録の配列 |
| imports | 利用元、import位置、指定、別名、解決先module IDの配列 |
| capabilities | 読み込みグラフから得た能力の確定集合 |

optionsのprofileはdebug/release、emitは初期はllvm、elideはbool。
sanitizerはalice設定の登録済み指定だけを受け付ける。
entry-functionの既定はmain。関数別testでは入口module内のtest_関数を指定する。
既存どおり引数・戻り値のない関数に限り、mainとtest_関数を混在させたファイルは拒否する。
元ソースをsnapshot上で書き換えず、compilerが選択した関数を呼ぶ内部の入口を生成する。
この入口の生成で別moduleのprivate関数や資源の内部操作を呼ぶ権限は追加しない。
操作とClockの効果はcompiler側の登録表から検証し、「副作用なし」という自己申告のキーを提供しない。
requirementsのresource-abiは新ABIの操作がなければnullを許すが、必要な資源/値codecのABI検査を無効にできない。
ClockなどOS資源を所有しない新操作もABI 1を要求し、読み込むmoduleの操作が未実装なら実行前に拒否する。
能力・ABI・登録表の不足を、最適化で使わないはずだという推測で無視しない。

module記録はID、分類、所属package、公開ID、sourceの相対ファイル名・長さ・hash、診断用元パスを持つ。
分類はEntry、Local、DependencyEntry、PackageInternal、StandardPublic、StandardPrivateとする。
単体ファイルにもroot packageの記録を作り、nameとmanifestをnull、直接依存を空とする。
標準moduleの所属packageはnullとし、定義元は検証したカタログのIDで識別する。同梱packageは通常どおりmanifestを持つ。
Entryと通常moduleが所属先を欠いたり、標準moduleが任意のユーザーpackageへ所属したりする記録は拒否する。
元のimportのファイル番号とbyte offsetも保持し、別名を書き換えた内部名を診断位置にしない。
IDは論理分類・パッケージ・module名から安定した順で発行し、OSの走査順に依存させない。
同じIDの重複、存在しない参照、同名別場所、重複した利用名、循環を拒否する。

入れ子の記録のキーも次に固定する。省略とnullを混同せず、nullableと記した項目以外は必須とする。

| 記録 | キーと型 |
|---|---|
| snapshotファイル | path:string、bytes:u64、sha256:string |
| package | id:string、name:nullable string、origin:object、manifest:nullable snapshot、direct:配列、requires:object、native-capabilities:配列、loaded:bool |
| package.origin | kind:string、root:nullable OSパス、bundled-name:nullable string。kindはroot/path/bundled/standalone |
| directの要素 | name:string、package:string、source:string、value:string。sourceはpath/bundled、valueはmanifestの指定値 |
| requires | build-protocol:u32、resource-abi:nullable u32 |
| module | id:string、class:string、package:nullable string、public-name:string、source:snapshot、origin:OSパス |
| import | from:string、to:string、start:u64、end:u64、spelling:string、alias:string |

snapshotのpathはjob内の相対ファイル名。moduleのoriginとpackageのrootは診断・同一性用の元OSパスである。
root/path/standaloneのoriginはrootを持ちbundled-nameはnull、bundledは検証したカタログ内のrootとbundled-nameを持つ。
packageのloadedもimportグラフから再検証し、未使用だという宣言で能力を省略させない。
spellingは元のimport指定、aliasは実際の利用名。start/endは利用元ソース内の半開区間で、必ず構文の位置と照合する。
ID、名前、版、能力、hash、パスにはそれぞれの既存規則を適用し、stringというだけで任意の値を許さない。

packagesの直接依存表は正規化したmanifest宣言と対応付ける。
compilerはmanifestのschemaと宣言、表の対応、各importの可視性を検証するが、元のディレクトリを探索して依存を選び直さない。
path/bundledの選択結果を、compilerごとの独立した依存resolverへ戻さない。
設定キー・宣言の検査表は共通の定義から生成し、Rust/jwcで同じfixtureを検証する。
未使用のmanifestと、実際に読み込んだpackageを区別し、後者から能力とABI要求を再確認する。

StandardPublic/Privateというrequestの分類だけで、内部操作を宣言する権限を与えない。
compilerはCLIで選んだsysrootまたは自身の配布情報から信頼するカタログを得て、標準/同梱の定義元と内容hashを照合する。
同梱と名乗るpackageや標準風のユーザーファイルも、同じ確認なしでは特別扱いしない。
依存の公開範囲と内部操作の登録は別に検査する。requestやhashは配布者の署名ではない。

## コンパイルの実行とresponse

aliceはjobを子プロセスの作業ディレクトリにし、`jwc --compile-request request.json`をshellを介さず起動する。
--sysrootが必要ならOS表現を保持した別の引数で渡す。request中に勝手な探索ルートを指定するキーは設けない。
元パスが文字列に変換できなくても、snapshotの名前と引数をASCIIに保てるようにする。
jwcはこのmodeで利用者設定・machine設定・JW_CLANG/JW_CFLAGSを解決せず、clangやrun_commandも起動しない。
通常の単体コンパイルmodeと混在したオプション、任意出力パス、未登録のemitを拒否する。

responseは次のキーを持つ。

| キー | 内容 |
|---|---|
| protocol | 整数1 |
| job-id | requestと同じID |
| request-sha256 | request.jsonの元バイト列のhash |
| compiler-info-sha256 | 選択したcompiler情報のhash |
| status | ok、errorのいずれか |
| resource-abi | 実際に必要なABI、またはnull |
| artifacts | kind、固定の相対パス、長さ、hashの配列。初期kindはllvmだけ |
| diagnostics | severity、安定したcode、message、module ID、byte位置、関連位置の配列 |

okならprogram.llの生成を完了し、最後にresponse.jsonを公開して0で終了する。
errorならartifactsは空、診断とともに1で終了する。CLI自体が不正なら2。
応答を書けない場合や異常終了でも非0とし、responseなしの0終了をaliceが成功として受け入れない。
job-id/hash/status/終了code/成果物が一つでも矛盾すれば失敗にする。
未知の診断module、範囲外の位置、巨大なmessageも検査し、読み取れない元ソース位置を偽造して表示しない。

aliceはartifactを上限付きで読み、固定のjob内ファイル・長さ・内容hashを検証する。
要求外のファイルやLLVMを別のパスから補完せず、検証した内容を共通backendへ渡す。
検証後にcompilerが残ったまま出力を書き換えないよう、子プロセスの終了を待ってから読む。
hashは悪意あるcompilerの正しさを証明するものではなく、利用者が選んだcompilerは信頼する実行プログラムとして扱う。

compilerのstdout/stderrはそれぞれ64 KiBまでを診断用に保持し、超過はdrainまたは制御した中断で処理する。
pipeが満杯になったために子プロセスと相互に待ち合わないようにする。
responseを正規の結果とし、stdoutのログや「success」という文字列で成功を判定しない。
compile-requestの初期期限は15分。期限超過・中断では子を終了させてjoinし、途中の出力と成功stampを公開しない。
これはcompilerのメモリ使用量をOSで完全に制限する仕組みではない。
OSで中断・終了待ちが遅れる可能性を含め、15分で必ず全回収が終わるという実時間の保証にはしない。

## 共通backendと通常実行

aliceは検証済みLLVMに、選択能力と同じ版のC/runtime、SDK、登録済みのリンク指定を組み合わせる。
compilerから任意のCファイル・コマンド行・DLL探索先を受け取らない。
header、登録表、compilerのLLVM、C側のlayout/ABIを検証し、ABI v1の起動時確認も資源初期化より前に行う。
LLVM verifierと必要なsanitizerを適用し、未対応環境なら明確に拒否する。
未計装の外部ライブラリを含む構成でMSan全体を検証したとは数えない。

RustのAST/IRは同じ解決済みグラフからProgramを作り、workerにも探索用closureではなく同じsnapshotの解決表を渡す。
資源・Clockなどの外部効果がある通常実行では自動の再実行比較を抑止する。
比較ハーネスは実行器ごとに独立した作業領域・DB・socket・描画先・資源contextを用意する。
status、部分進捗、DBのoutcome、診断、回収結果も比較し、表示の一致だけで意味が同じと判定しない。

## キャッシュとbootstrap

指紋には正規化したグラフ・設定・入力内容と、選択compilerの実体hash・登録表・runtime・標準版を含める。
job-id、毎回変わる一時ディレクトリ、requestのtransport用hashそのものは再ビルドのキーにしない。
同じ入力のキャッシュを保ちつつ、別jobのresponseや、同じ名前で差し替えたcompilerを受け入れない。
出力の錠、失敗時の旧成果物、DLLのhash配置、stampの公開順はalice-nativeの契約を使う。

新jwcをコンパイルするために、同じ新標準ライブラリが既に必要になる循環を避ける。
先にRust版へ資源型・登録表・プロトコルとcompiler用の上限付きUTF-8入力処理を入れる。
compiler専用の入力処理はビルドする道具の内部接続とし、アプリ用File型や任意FFIの公開を前提にしない。
jwc側のJSON・manifest検査・SHA-256・登録情報の読み取りを対応させ、既存の整数・文字列・bytes操作を土台にする。
現行read_fileで無制限に全量を読んでから長さを調べる処理を、上限付き読み込みと同等とは扱わない。

1. 新Rust版で新jwcを生成するstage 1。
2. stage 1のjwcで同じjwcソースを生成するstage 2。
3. 両stageの型/IR/LLVMとABI、資源攻撃、パッケージを含むfixtureを比較する。

bootstrapの成功だけでWindows実APIや外部依存の動作まで確認済みとはしない。
現行runtime.jwと新規の生成ファイルは元データとの一致も確認し、手動で片側だけ更新しない。

## 完了条件

| 検証 | 必須の確認 |
|---|---|
| 情報 | 正常/旧版/未知版、infoの混在・巨大出力・不正JSON、実体差し替え、能力・ABIの不足 |
| snapshot | 元ファイルの変更、hash不一致、CRLF/BOM、非UTF-8の元パス、link逸脱、上限付き読取り |
| グラフ | 直接/推移依存、private、標準の偽装、別名、同名別場所、循環、重複ID、不正な位置 |
| 結果 | 成功/error/使用法失敗、異常終了、response欠落、別job、範囲外のartifact、部分出力 |
| 制御 | stdout/stderr過多、pipeの詰まり、期限・中断、子の回収、旧出力とstampの維持 |
| 一致 | Rust/jwcの型・IR・LLVM・診断、関数別testの入口、同じbackend、workerの同じsnapshot、通常実行の副作用一回 |
| ABI | C/Rust/LLVM/jwcのlayoutと状態、出力初期化、起動時の不一致拒否、資源contextの分離 |
| bootstrap | stage 1/2、生成物の元データとの一致、パッケージ・資源を含む実プログラム |
| 安全性 | 固定攻撃・生成入力・LLVM verifier・sanitizerの負例と安全性1〜7を拡張 |

これらは実装後の合格条件であり、本設計の作成時点の試験成功ではない。
