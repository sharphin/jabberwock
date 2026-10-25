# 配布カタログ・登録表・生成物の仕様 v1

状態: 実装前の設計。公開契約は [registry.json](stdlib-v1/registry.json)、配布形式は
[catalog.schema.json](stdlib-v1/catalog.schema.json)、構造の例は [catalog.example.json](stdlib-v1/catalog.example.json)。
例はdesign-exampleで、標準ライブラリの実ソースや実装済みruntimeを含む配布物ではない。

## 登録表

registryのschema_version/api_version/resource_abi/codec_versionは初期1とし、別の版として管理する。
state=design-onlyはこの資料の状態。対応済みかどうかはcompiler/runtimeの実装・試験と別に判定する。
modulesは名前、standard/bundled、能力、直接の型/API依存を持つ。順序に頼った名前解決はしない。
typesは完全修飾名をkeyとし、resource/opaque/record/enumのいずれか。

| 種別 | 必須の定義 |
|---|---|
| resource | resource_kind、copy=false、send=false |
| opaque | value_id、Copy/Send、privateなcodec fieldの順序 |
| record | 公開fieldの名前と型の順序。必要な型だけdefault factoryの値を持つ |
| enum | variant名、固定u32 tag、各fieldの名前と型の順序 |

プリミティブとlist/array/option/result以外の型は必ずtypesに解決できること。
個別の公開型と能力の対応、resource kind 1〜12、opaque value_idの一意性を検査する。
opaqueにresourceを隠したり、型参照でpackageの直接依存を迂回したりしない。
初期の登録表では再帰する値型を持たず、将来追加するときも無限layout計算にしない。

operationsの各項目はid/module/name/symbol/params/returns/effects/barrier/capabilitiesを持つ。
paramsには名前・型・value/ref/mut/move、returnsには完全な公開返却型を記す。
default_resultがある場合は外部効果のないfactoryで、$param参照も同じ型の引数だけを許す。
全初期操作は解析のbarrierとする。pureでもユーザーが属性を足して世代検査を消す入口にはしない。

操作IDの上位16bitは初期bank、下位16bitは固定番号。bankはfs=1、time=2、net=3、tls=4、
postgres=5、window=6、draw2d=7、image=8、gui=9。bank 0はruntime内部の管理用に予約する。
番号とC symbolを再利用しない。削除した番号はreserved_operation_idsへ残す。
関数の表示名だけでnative操作を決めず、解決済み定義元と登録IDを使う。

registry.lock.jsonは正規化した契約のSHA-256を固定する。
正規化はUTF-8 JSONでobjectのkeyをASCII順、空白なし、非ASCIIはそのまま、array順は保持する。
default値は有限数だけを許す。これは生成チェック用で、compiler間の浮動小数の再serializeを要求するprotocolではない。
compilerへ埋め込むregistry-sha256は実際のregistry.jsonの元byte列のhashで、解釈後の再serializeでは計算しない。
v1の契約変更を--writeで黙ってlockへ上書きしない。公開後の破壊的変更は版を分け、過去のIDを流用しない。

## カタログ

カタログは型やnative操作を新設するファイルではなく、選んだ配布のソースを既知の定義へ結び付ける。
schemaの未知field・重複key・未知版を拒否する。全JSONはUTF-8、整数をf64で丸めない。
catalog_kind=installedだけが実行・buildへ使える。design-exampleは構造検査には使えても、installとして拒否する。
targetsの列挙も実際に検証したものだけを配布時に書き、例の候補を対応済みと見なさない。

| 最上位field | 意味 |
|---|---|
| catalog_version/catalog_kind/distribution_id | 形式、配布種別、配布の識別名 |
| stdlib_build_id | 下記の内容ID |
| registry_sha256/runtime_sha256/resource_abi | compiler・runtime・C接続と照合する内容と版 |
| runtime_manifest | 全配布runtimeソースのmanifestのpath/bytes/sha256 |
| targets | 登録済みの対応target |
| modules | 公開/内部の標準moduleとsource・依存・公開範囲 |
| packages | 同梱packageのroot・manifest・entry・直接依存・標準import・能力 |

sourceはpath/bytes/sha256を持ち、pathはカタログのある配布ルート内の相対パス。
初期の配布内の名前はASCIIとし、ルート自体のOSパスは日本語・空白を含めて保持する。
絶対パス、空の成分、.、..、区切りの置換、同一source pathの二重登録を拒否する。
sourceの固定上限は16 MiB、module数16384、package数4096、文字列16384文字。
実際に読む際はalice設定の小さい全体/個別上限も適用し、カタログ全体にも64 MiBの上限を置く。

標準moduleはid/visibility/source/imports/allowed_importers/capabilities/resource_abiを持つ。
privateの参照には利用元がallowed_importersに存在する必要がある。publicのallowed_importersは空。
同梱packageはname/root/manifest/entry/dependencies/standard_modules/capabilities/resource_abi。
dependenciesは同梱package、standard_modulesは公開標準moduleだけを指す。
root外のentry/manifest、未定義参照、重複ID、循環、登録表と異なる必須依存や能力を拒否する。
同梱packageをcatalogに置いても、利用者の直接依存宣言を省略できるようにはしない。

stdlib_build_idはそのfieldを除いたカタログを正規化し、prefixと合わせてhashする。
`SHA256("JW-STDLIB-CATALOG-V1" + NUL + canonical-json)`とし、objectはASCII key順、空白なし、UTF-8。
カタログは浮動小数を持たず、整数・bool/null・文字列・配列・objectに限る。文字列のescapeはJSONの最小形に固定する。
bytes/hashはソースの元byte列に対応し、改行やBOMを正規化しない。
自己参照するbuild_idをhashへ入れず、manifest/entry/private helperの差し替えも内容IDへ反映する。

runtime_sha256は生成元C/runtime集合のmanifestを同じcanonical方式でhashする。
manifestは論理パス順の{path,bytes,sha256}配列で、header、共通C、配布に含む全能力の接続ソースを全て含む。
選択能力だけの集合へ内容IDを変えず、compiler-infoと同じ全体のIDにする。
runtime_manifest自体の元byte列の長さ/hashも検証し、そこから各sourceを上限付きでsnapshotへ読む。
実際にcompile/linkするのは必要な部分だけとし、その選択結果と内容はbuild fingerprintへ別に入れる。
SDK/DLL・実際のnative入力と出力のfingerprintは、別途alice-nativeの規則で管理する。
例のruntime hashやsourceはfixture用であり、存在するABI v1 runtimeのhashだと表示しない。

## 信頼・生成・compiler連携

配布ルートを一つ選び、catalogの内容ID・登録hash・ABI・targetを照合してからsourceをsnapshotへ読む。
symlink/reparse pointも解決した実体が配布内か検査し、検査後に別のパスで開き直さない。
配布更新中に内容が変われば再試行または失敗にする。digestは署名や配布者への信頼の証明ではない。
同一利用者が明示して選ぶtoolchainそのものを、この設計がOS sandboxへ閉じ込めるわけではない。

Rust/C/LLVM/jwcの型・操作・効果・ABI宣言は、この登録表から生成する。
実行時にプロジェクトのregistryを読み込んで任意symbol・DLL・Cソースを登録する機能は設けない。
公開fieldを持つユーザーのstruct、request中のStandard分類、capabilities宣言だけでは内部操作を呼べない。
compilerの既知IDと信頼する配布の定義元・内容を両方照合する。

compiler-info v1にはoperationsとして実装済みID配列、capability-targetsとして能力ごとの対応targetを加える。
登録表の全IDを知っていることと、それを実装・検証したことを別にする。
aliceは読み込む標準module/同梱packageの宣言に必要な操作が実装されているか照合し、不足なら実行前に拒否する。
new ABIだけが部分実装された段階で、全標準moduleを対応済みと広告しない。
Clock等も新値codecの操作なのでABI 1を要求する。ABIなしのpure legacy programは従来経路を保つ。

## 設計成果物の再生成と検査

```text
python3 scripts/stdlib_design.py --write
python3 scripts/stdlib_design.py
gcc -std=c11 -Wall -Wextra -Werror -fsyntax-only -x c docs/stdlib-v1/stdlib-abi-v1.h
```

Pythonの標準ライブラリだけで実行する。--writeも既存lockと異なるv1契約は拒否する。
生成するのは公開API表、操作表、C宣言、カタログschema/例であり、C関数本体や.jwの標準ライブラリではない。
正常例の参照整合に加え、重複ID、未知型、resourceのSend化、未知key、依存cycle、版違い、逸脱path、例の誤installを拒否する。
headerのstatic assertはArgs/Outのsize/align/offsetを確認する。
.github/workflows/stdlib-design.ymlでもLinux/Windowsの両方で定義検査とC/C++宣言のclang検査を実行する。
このworkflowは設計の宣言を検査するもので、未実装のOS操作や既存安全性1〜7を代わりに合格させるものではない。
productionのC実装・生成runtime/jwcファイルにも、同じ元データとhashの一致チェックを実装する。

WindowsのC ABI、実機GUI/IME/UIA、TLSとDBの結合、既存の安全性1〜7は実装後に別々に検証する。
設計生成の成功を機能完成、実環境への対応、独立した安全性監査として報告しない。
