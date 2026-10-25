# aliceのmanifest・利用者設定・コマンド仕様

状態: 実装前の仕様。ここで定める新形式、設定、コマンドは現在のaliceには未実装。
[現在のパッケージ仕様](packages.md)、[標準ライブラリの読み込み](standard-library-loading.md)、
[ネイティブ依存・ビルド管理](alice-native.md) を前提に、設定の書式と解決規則を定める。
資源ABIの具体的な配置やGUI内部の設計を、この文書で確定したとは扱わない。

## ファイルの責任と版

| ファイル | 内容 | 読み込み元 |
|---|---|---|
| alice.toml | 名前、直接依存、要求するcompilerプロトコル・資源ABI、能力 | アプリと各依存パッケージ |
| .alice/machine.toml | このアプリをビルドするマシンのcompiler・SDK・DLL・上限 | 最終アプリのルートだけ |
| 利用者のconfig.toml | マシンに共通のcompiler・SDK・DLL・上限 | 利用者の設定先一か所だけ |

manifestにSDKの場所・任意のCコード・ビルドスクリプト・compiler引数を記載させない。
依存先のmachine.tomlや利用者設定は読まない。推移依存を通じて別のmachine設定を取り込むことも禁止する。
DBの接続先、DSN、パスワード、CAの選択などのアプリ実行時設定は、このビルド設定の対象ではない。

manifestの新形式はトップレベルの`format-version = 2`で選ぶ。
machine設定と利用者設定はトップレベルの`config-version = 1`を必須とする。
これらは文書で予約する形式番号であり、製品のリリース番号ではない。
未知の形式番号は拒否し、近い版として解釈しない。
manifestで形式番号を明示する場合は2だけを受け付け、v1は宣言のない互換形式としてのみ扱う。
config-versionも最初の注釈・空行以外の文に置く。

## 旧manifestとの互換性

format-versionのないmanifestは現行形式v1として扱う。
名前・path依存・文字列だけの依存指定・未知の節やキーを読み飛ばす挙動を維持する。
v1の文字列を新しいescape規則で再解釈せず、Windowsの既存パスを変えない。
ただし、新しい設定に似たnative節などをv1で書いた場合は、適用していないことを警告できるようにする。
未知キーを勝手に新しい能力宣言へ昇格させない。

v2を選ぶ宣言は最初の注釈・空行以外の文に置く。ほかのトップレベル位置にある宣言は配置エラーとする。
v2の未知の節・キー・重複・不正な型を拒否する。v1へ黙って戻さない。
旧aliceはformat-versionも読み飛ばすため、この宣言だけで旧版の拒否を保証しない。
bundled依存や新しいimport・内部操作には、旧compilerで解決できない機能要求も必要になる。
特に、旧道具がnative宣言だけを無視して一見成功する可能性を、版番号で防げたとは説明しない。

`alice new`は新しい道具でv2を既定にするが、既存のmanifestを自動変換・上書きしない。
新形式への移行時には、旧版が読まなかった独自の節をそのまま新仕様として引き継がない。

## 新形式で扱うTOMLの範囲

外部Rustクレートを使わない方針の下で、次の明示した部分集合を実装する。
TOML全体に対応したとは表示しない。legacy v1の読み取りは別の互換経路に保つ。

- UTF-8、LF/CRLF、先頭だけの任意のUTF-8 BOMを受け付ける。途中のBOMやNULは拒否する。
- bare keyと節名はASCIIの英字・数字・_・-を使う。任意のquoted keyは扱わない。
  節のドットは、このschemaに記した階層だけで使う。任意のdotted keyによる別表現は提供しない。
- 対応する値は文字列、非負の十進整数、文字列の配列、依存指定のinline table。
  整数の+記号・桁区切り・不要な先頭0は許さない。0自体は構文として読めても、各キーの範囲で検査する。
- 二重引用の文字列は`\"`、`\\`、`\n`、`\r`、`\t`、`\uXXXX`、`\UXXXXXXXX`を解釈する。
  Unicodeのscalar値だけを許し、不正なescape・surrogateを拒否する。
- 一重引用の文字列は一行のliteralとして扱う。Windowsのパスには`'C:\LLVM\bin\clang.exe'`と書ける。
- 文字列内の実改行、複数行文字列、配列の配列、array of tables、日時、浮動小数は扱わない。
- 配列とinline tableは一行とし、配列は空と末尾のカンマを許す。
  依存のinline tableは指定元の一要素だけを持ち、末尾のカンマは許さない。
- `#`は文字列の外側でだけ注釈になる。文字列中の#、カンマ、=を区切りとして扱わない。
- 同一キー・節の再宣言を拒否する。後勝ちで誤記を隠さない。ファイル間の上書きは後述の優先順位による。

ファイルごとに1 MiB、項目数4096、配列要素数4096、文字列の復号後16 KiBを上限とする。
これらのパーサー上限は設定ファイル自身では引き上げられない。
値は全体を読み切って型と上限を確認してから適用する。整数の桁数と計算のあふれも確認する。
行・列とkeyを保持し、設定エラーのためにpanicしたり、巨大な値を無制限に確保したりしない。

## alice.toml v2

```toml
format-version = 2

[package]
name = "app"

[dependencies]
mathx = { path = "../mathx" }
postgres = { bundled = "postgres" }

[requires]
build-protocol = 1
resource-abi = 1

[native]
capabilities = ["postgres"]
```

| 節・キー | 型・既定値 | 契約 |
|---|---|---|
| package.name | 必須の文字列 | 英字か_で始まる英字・数字・_。現行の名前規則を維持 |
| dependencies.名前 | inline table | pathまたはbundledのどちらか一つ。節を省略すれば依存なし |
| requires.build-protocol | 正のu32、既定1 | パッケージが要求するalice/compiler連携プロトコルの版 |
| requires.resource-abi | 正のu32、省略可 | 資源ABIの完全一致。省略しても標準・依存元が要求するABIを無効化しない |
| native.capabilities | 文字列配列、既定[] | fs、net、tls_client、postgres、window、draw2d、image、guiだけを許す |

build-protocolの版は対応する形式が明示された版だけを受け付ける。
単にcompilerが返した番号の大小で互換と推定せず、そのcompilerが要求版をサポートするか確認する。
例のresource-abiの1は初期ABI用に予約する番号で、メモリ配置の確定や対応compilerの存在を意味しない。
資源ABIは選択したcompiler・標準配布・使用する全パッケージで一致させ、複数版を混在させない。

pathはmanifestのあるディレクトリを基準に解決し、従来どおり絶対パスと親への相対パスも許す。
依存先のpackage.nameとの一致、src/lib.jwの存在、同名別場所の拒否、直接依存だけの可視性を保つ。
bundledは選択中のカタログの同名パッケージを指す。git、URL、版範囲、別名、lockfileは初期範囲に含めない。
v2では文字列だけの依存指定は許さず、pathであることを明示する。

capabilities内の重複・未知の名前はエラーにする。能力はネイティブ依存の要求であり、内部操作を定義する権限ではない。
native節はそのパッケージが読み込まれた場合に適用し、未使用の依存パッケージだけを理由にDLLを要求しない。
ただし、未使用の依存のmanifestも形式・依存元・要求名の妥当性は検査する。
読み込みグラフに入ったrequiresを集め、矛盾したABIや未対応プロトコルを実行前に拒否する。
アプリ側のnative節は省略できる。postgres依存を実際に読むだけでも、依存先自身のpostgres能力が収集される。

## machine設定と利用者設定の探索

プロジェクトは従来どおり、開始ディレクトリから親をたどり最も近いalice.tomlを選ぶ。
`--manifest-path PATH`を指定した場合はそのファイルだけを使い、不備があっても親へ探し直さない。
.alice/machine.tomlは選んだmanifestのディレクトリ内だけを読み、親や依存先の同名ファイルは重ねない。
`alice new`が作る.gitignoreには、既存のtargetに加えて/.alice/machine.tomlを入れる。
machine設定を共有用manifestへ移さず、マシン固有の場所を依存パッケージへ持たせない。

利用者設定の既定位置は次の一か所とする。

| OS | 位置 |
|---|---|
| Windows | OSのRoamingAppData既知フォルダ内のalice/config.toml |
| Linux | 絶対パスのXDG_CONFIG_HOME内のalice/config.toml。未設定なら利用者home内の.config/alice/config.toml |

LinuxでXDG_CONFIG_HOMEが相対なら誤設定として拒否する。
利用者の既定位置をOSから取得できない場合は、探索不可の警告を出し、その層を設定なしとする。
利用者設定が必要なら--user-configで明示でき、--no-user-configならその位置の取得も行わない。
作業ディレクトリへ利用者設定を探し直さない。ネット上の設定取得やincludeは提供しない。
利用者設定の既定ファイルとmachine設定は、不在なら設定なし。読取り拒否・不正形式・壊れたリンクは不在と区別してエラーにする。
利用者が`--user-config PATH`で指定したファイルは必須とし、既定の利用者設定を置き換える。追加の一層にはしない。
`--no-user-config`と`--no-local-config`で、それぞれの自動読取りを無効にできる。
`--user-config`と`--no-user-config`の同時指定はエラーとする。

## 設定キー

machine設定と利用者設定は同じschemaを使う。

```toml
config-version = 1

[toolchain]
compiler = "rust"
clang = 'C:\LLVM\bin\clang.exe'
target = "x86_64-pc-windows-msvc"

[sdk.windows]
root = 'C:\Program Files (x86)\Windows Kits\10'
version = "10.0.22621.0"

[providers.postgres]
library = 'C:\deps\postgres\bin\libpq.dll'
include-dir = 'C:\deps\postgres\include'
runtime-files = ['C:\deps\postgres\bin\libssl-3-x64.dll', 'C:\deps\postgres\bin\libcrypto-3-x64.dll']
deploy = "copy"

[limits]
modules = 4096
```

例のDLL一覧は実際のlibpq配布物の依存を保証しない。必要な推移依存を実物から調べて検証する。

| キー | 値・既定 | 契約 |
|---|---|---|
| toolchain.compiler | rustまたはjwc、既定rust | compiler実装を選ぶ。実行器の自動変更はしない |
| toolchain.jwc | 実行ファイルの指定、省略可 | jwc選択時の実体。既定はaliceの実体と同じbin内のjwc |
| toolchain.clang | 実行ファイルの指定、省略可 | C/LLVM用。省略時の解決は優先順位に従う |
| toolchain.target | target名、省略時は選択compilerのhost | 初期はhostと同じtargetだけ。未知の名前を推測しない |
| sdk.windows.root | SDKのディレクトリ、省略可 | Windows backendで必要な場合だけ解決する |
| sdk.windows.version | SDK版の文字列、省略可 | root内の実在版を一つ選ぶ。自動選択の結果も記録する |
| providers.postgres.library | DLL/共有ライブラリのファイル、省略可 | 使用時は明示指定を必須とし、PATHや作業ディレクトリから探さない |
| providers.postgres.include-dir | ヘッダーのディレクトリ、省略可 | 選択する接続処理のビルドで必要なら必須 |
| providers.postgres.runtime-files | ファイルの配列、既定[] | 配布に追加する推移依存。ディレクトリ全体のコピーやglobは許さない |
| providers.postgres.deploy | copyまたはexternal、既定copy | copyは検証済みの依存を出力へ配置。externalは管理された指定先を利用 |
| limits.名前 | 後述の正の整数 | 読み込みの資源上限 |

rustはaliceに組み込んだRust版のcompilerを既定とし、同じ情報取得APIからhost・標準配布の位置・ABIを得る。
jwc選択時は指定した実行ファイルが同じ連携プロトコルを提供する必要がある。現在のjwcが対応済みとはしない。
rustを選んだ際にjwcを探したり、jwcが使えないときにRust版へ戻したりしない。
--sysrootは標準ソースの指定であり、使用するjwcの実体をそのルートの別ファイルへ差し替えない。

jwcやclangの実体を指定する文字列は一つの実行ファイルであり、shellのコマンド行ではない。
jwc/clangの指定が区切りを含まない実行名なら、選択時にOSのPATHで一度解決して実体を固定する。
それ以外は設定ファイルのあるディレクトリを基準に解決する。CLIの相対パスはコマンド開始時のディレクトリが基準。
library・include-dir・runtime-files・SDK rootの相対パスも記載した設定ファイルが基準であり、manifestや依存元が基準ではない。
全パスでNULを拒否し、~や$NAMEや%NAME%の展開をしない。設定の文字列はOSのPathへ変換できる範囲に限る。
非UTF-8のOSパスまで設定文字列で表現できるとは約束しない。CLIで受け取るパスはOS表現を保持する。
SDK versionは4つの非負のu32をドットで区切った形に限定し、ディレクトリの区切りや親への移動を含めない。

未知のキー・能力ごとの任意節は拒否する。ファイルを跨いだinclude、任意compiler/linker引数、任意のprovider登録は許さない。
sysrootは設定キーにせず、読み込み仕様どおり明示CLIか選択compilerの配布情報から決める。
これにより依存先や隠れた利用者設定が、標準ソースの探索先を差し替えることを避ける。

## 優先順位と合成

設定値ごとに次の順で最初に存在するものを採る。

1. 明示したCLIオプション。
2. 最終アプリの.alice/machine.toml。
3. 選択した利用者config.toml。
4. clangに限り、既存のJW_CLANG。
5. 選択compiler・登録したOS backendの既定値。

JW_CLANGの既定互換を残すが、明示設定のcompilerを環境変数で上書きしない。
JW_CLANGも一つの実行ファイルとして扱い、shellへ渡さない。値が空なら誤設定として拒否する。
scalarは値単位で上書きし、配列は上位の配列で全体を置き換える。自動で連結・重複除去しない。
たとえばlibraryをmachine、include-dirを利用者設定から採る場合も、選択結果の版・ABIが一致するか検証する。
同じ層の重複指定は拒否する。上位で隠れる値も構文・型の誤りは無視せず、読んだファイル全体を検証する。

使っていないproviderについてはschema・文字列・パス形式を検査するが、DLLやSDKの存在・版のprobeやロードはしない。
実際の能力に必要になった時点で存在・header・target・ABI・依存を確認する。
例外として明示した--sysrootは、標準の利用がなくても選択と整合を検証する。
明示した設定の不備で既定値へ黙って戻らず、どのファイルのどのキーが原因かを示す。

v1 manifestでも利用者が新しいmachine設定やCLIを選んだ場合は、新しい設定の規則を適用する。
manifestも外部設定も新機能を使わない従来の呼び出しでは、既存のJW_CFLAGSの解釈を維持する。
新形式を使う呼び出しでJW_CFLAGSが非空なら拒否し、--sanitizeなどの登録済み指定へ移すよう診断する。
空白分割する旧JW_CFLAGSへ新しい任意引数APIを足さない。値自体を診断へ出さない。
従来の呼び出しでも、再ビルド判定には実際に使ったJW_CLANG/JW_CFLAGSと入力の内容を反映する。

## 読み込み上限

machine設定と利用者設定のlimitsは、標準・通常モジュール・manifestの依存グラフ全体に適用する。
依存パッケージには上限を引き上げる設定を持たせない。

| キー | 単位 | 既定 | 設定可能な最大値 |
|---|---|---:|---:|
| modules | 読み込むmodule IDの数 | 4096 | 65536 |
| manifests | rootと依存のmanifest数 | 4096 | 16384 |
| manifest-total-bytes | manifestの合計バイト数 | 16777216 | 67108864 |
| dependency-edges | manifest依存とimportの解決済み辺数 | 32768 | 1048576 |
| dependency-depth | manifest依存とimportを辿る最大深さ | 128 | 512 |
| source-file-bytes | 1ソースのバイト数 | 16777216 | 67108864 |
| source-total-bytes | snapshotの合計バイト数 | 268435456 | 1073741824 |

単位のsuffix、負、0、範囲外、整数変換のあふれを拒否する。
source-total-bytesはsource-file-bytes以上を必要とする。辺・深さはmanifest探索でも読み込み前に検査する。
重複した同じIDのソースを何度も加算せず、別のIDとして読む同じ内容はそれぞれ数える。
辺は利用元・参照先の組ごとに数え、注釈や文字列を辺にしない。循環は深さ上限を待たず拒否する。
manifestの数と合計は、依存表を作る段階から検査する。同じパッケージのmanifestは一度だけ数える。
ソース・manifestの上限は全量を確保してから確認せず、上限付きの読み取り中にも確認する。
これらは読み込みの上限であり、型展開や実行時メモリまで一定量に制限する仕組みではない。
安全性検証で別に残る資源枯渇・巨大型の課題を、設定上限で解決済みとは扱わない。

## CLI

新しい共通オプションはbuild/run/testとconfigサブコマンドで扱い、コマンド名の後に書く。
`--key VALUE`と`--key=VALUE`を受け付ける。未知のオプション・値不足・同じscalarの重複は使用方法のエラー。

| オプション | 内容 |
|---|---|
| --manifest-path PATH | 使うmanifestを明示 |
| --user-config PATH / --no-user-config | 利用者設定の置き換え/無効化 |
| --no-local-config | rootのmachine設定を読まない |
| --compiler rustまたはjwc / --jwc PATH | compiler実装とjwcの実体 |
| --clang PATH / --target NAME | C compilerと対象 |
| --sysroot PATH | 標準配布のルートを明示 |
| --windows-sdk-root PATH / --windows-sdk-version VERSION | SDKの指定 |
| --postgres-library PATH / --postgres-include-dir PATH | postgresのlibrary/header |
| --postgres-runtime-file PATH | 推移依存。繰り返した値でCLI層の配列を作り、下位の配列を置き換える |
| --postgres-deploy copyまたはexternal | 配布方式 |
| --limit KEY=DECIMAL | limitsの一項目。同じKEYの重複はエラー |
| --sanitize LIST | native用。none、address、undefined、address,undefined、thread、memoryのいずれか |

--jwcはjwc選択時だけ許す。設定に記録した未選択jwcのパスはprobeしない。
--sanitizeの初期既定はnone。target/backendが対応しないsanitizerは拒否し、外した状態で成功扱いにしない。
ASanとTSanなどの併用は提供しない。実際の依存まで計装したかは試験結果で別に記録する。
--sanitizeの指定はnativeのbuild/run/testだけで受け付け、インタプリタへ黙って適用しない。
configサブコマンドでは選択候補として検証できるが、試験を実行したとは扱わない。

| コマンド | 初期の動作 |
|---|---|
| alice build [共通指定] [--release] | nativeを作る。ライブラリだけのプロジェクトは現在どおり実行ファイルbuildを拒否 |
| alice run [共通指定] [--native] [--release] [-- 引数...] | 既定はRustのインタプリタ。--releaseは従来どおりnativeも選ぶ |
| alice test [共通指定] [--native] [--release] [filter] | 既定は既存のインタプリタ試験。native指定なら実行ファイルで試験 |
| alice config check [共通指定] | 形式・合成・依存表・読み込みグラフを検証し、設定/必要能力の不一致を診断 |
| alice config show [共通指定] [--json] | 同じ静的解決結果から有効値と選択元を表示。非表示項目は値を出さない |

runの最初の--以降はプログラムの引数としてそのまま渡し、以後はaliceの指定として解析しない。
testのfilterは従来どおり一つの部分文字列。新しい--native/--release以外は既存の試験発見と分離を維持する。
--compiler jwcのrun/testにはnative指定が必要。暗黙に実行器を変えず、足りなければ診断する。
configコマンドはbuild/run/testの切替を持たず、jwcを選んでもnativeの指定は不要。

config check/showでは設定・ソースを読むが、ユーザープログラム・compilerの外部実体・clangを起動せず、DLLもロードしない。
外部jwcのプロトコル、clangのtarget、DLLの依存など実行や追加probeが必要な項目は「未検証」として列挙する。
外部compilerのhostなど、静的に確定できない値をRust版の値で補完しない。
checkは必要なソース・カタログの静的解決を完了できない場合は失敗とし、showは未確定の項目を別表示できる。
checkの成功は静的検証の成功であり、buildの成功・対応環境の検証済みを意味しない。
--sysroot指定時は静的に得られるcatalogを検証し、外部compilerとの一致が未検証ならそのまま明示する。
実際のbuild/run/testでは必要な情報取得とbackend検証を完了してから実行する。

```text
alice build --release --compiler rust --sysroot C:\jw --postgres-library C:\deps\pg\bin\libpq.dll
alice run --native -- --user-option
alice test --native query
alice config check --no-user-config --no-local-config
alice config show --json
```

利用者設定とlocal設定を両方無効にしても、OS toolchainの探索まで完全に再現可能になるとは約束しない。
再現性が必要な場合はcompiler・SDK・native入力を明示し、指紋に実際の入力を記録する。
newの既存オプションは維持し、この共通指定は受け付けない。取得・公開・任意のscript実行コマンドは追加しない。

## 配布・起動・ビルド指紋

postgresのcopyは、選択したlibraryと検証したruntime-filesを内容hashごとの配布先へ置く。
同名・異内容のDLL、target/CRT不一致、欠けた推移依存、明示されていない非system依存は拒否する。
runtime-filesで同じ実体・同じ配置名を重複指定した場合は、内容が同じでも誤設定として拒否する。
コピーを実行可能な完成済み配置へ公開し、開発機の絶対パスからロードし続けない。
移動した配布物で起動する試験を完了してから、移動可能と説明する。

externalは選択した検証済みの絶対パスを使用し、配布先にも同じ管理された実体が必要であると明示する。
実行時に必要な版・target・ABI・内容の一致を確認し、同名別版やPATHへfallbackしない。
copyが失敗したときにexternalへ切り替えない。探索の制限、DLLの寿命、ライセンスはalice-nativeの契約を適用する。

指紋には実際に読んだ設定の形式・内容hash、正規化した有効値、選択元の分類、
能力、target、profile、sanitizer、選択したcompiler/runtime/標準ABI、SDKとnative入力の内容を含める。
CLIで設定を変えても古い出力を使わず、ビルド中に設定が変わった場合もsnapshotの契約で検出する。
環境変数は全体を保存せず、compilerへ影響するものと実際に使った入力を特定する。
WindowsのSDK関連環境などから選んだheaderやlibraryも、使った実体を追跡して指紋へ反映する。
DBパスワードなどのアプリ実行時情報を、指紋や設定解決情報へ収集しない。

## 診断・終了状態・実行器間の共有

診断には設定のファイルと行・列・キー、期待する型、選択元を示す。
config showのJSONは`"schema-version": 1`を持つ版付きオブジェクトとする。
effectiveに有効値、originsにキーごとの選択元、unverifiedに未検証事項、unresolvedに未確定事項を入れる。
未確定の有効値はnullとし、既定の推測値を確定値として埋めない。表示を許す項目は本schemaの既知キーに限る。
OSパスはdisplay、encoding、hexを持つオブジェクトとし、WindowsはUTF-16LE、LinuxはOSのバイト列を小文字hexで保持する。
表示用の置換文字を入力の識別に再利用しない。JSON出力がそのまま設定ファイルとして読めるとは扱わない。
未知のキーのエラーで入力した値をそのまま出さず、設定全体・環境全体・任意のcompiler出力もdumpしない。
外部ツールの診断は必要な範囲だけを扱い、機密情報を必ず除去できると保証しない。

alice自身の使用方法エラーは2、設定・読み込み・ビルドの失敗と試験失敗は1、成功は0を基本とする。
runで正常に起動したプログラムの終了codeは従来どおり伝える。
実行条件不足を試験成功にせず、testで選択したnative能力の不足は失敗とする。
CIが意図的にskipした構成は、別の記録として扱い合格数へ足さない。

設定解決はaliceが一度行い、module ID・ソースsnapshot・有効設定・能力・ABIをまとめた版付きの連携情報を渡す。
Rust版の組み込み経路とjwcの外部経路で同じ情報を使い、jwcが利用者設定や依存先を独自に探し直さない。
workerにも同じ解決結果を渡す。連携情報の具体的なbinary/JSON配置はcompiler連携設計で確定する。
受け側で形式・内容hash・直接依存・ABIを検証し、その情報だけを内部操作の特権を与える根拠にしない。
通常実行で外部副作用を重ねて起こさない規則も維持する。

## 完了条件

| 試験 | 確認する内容 |
|---|---|
| 互換 | v1の未知キー・文字列path・Windowsのbackslash、新形式を使わないCLIと環境変数の既存挙動 |
| 形式 | v2/設定v1、未知版、未知キー、重複、escape・注釈・UTF-8・BOM、各種上限とあふれ |
| 依存 | path/bundledの排他、同名別場所、直接/推移の可視性、未使用能力、旧道具での誤った成功の検出 |
| 合成 | CLI/local/user/JW_CLANG/既定の順、配列置換、相対パスの基準、上書きされた誤設定の拒否 |
| 探索 | manifest明示/親探索、利用者設定の不在/読取り失敗、依存先設定の無視、自動読取り無効化 |
| compiler | Rust/jwc、protocol/ABI、未対応target、clang実体、jwc不足、暗黙のfallbackがないこと |
| CLI | --以降の引数、test filter、重複・不足・未知指定、release/nativeとsanitizerの組合せ |
| 設定検査 | check/showが外部実体・ユーザーコード・DLLを実行せず、未検証と成功を混同しないこと |
| 配布 | copy/external、欠損・同名別内容・target/CRT違い、探索制限、移動先での起動 |
| 指紋 | 設定・SDK・DLL・compiler・flagsの変更、時刻を保った変更、途中変更、旧出力との不一致 |
| 診断 | 行列・選択元・終了code、値を出さない未知キー診断、実行条件不足を成功扱いにしないこと |

上記は実装後の試験条件であり、設計書を書いたことで合格したものではない。
新パーサーと読み込み経路には既存の固定攻撃・生成入力・安全性1〜7のCIを追加適用する。
