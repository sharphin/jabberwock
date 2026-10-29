# 検査を消す判断の証明（`formal/JabberwockFormal/ElisionSoundness.lean`）

`src/ir/analyze.rs`（と同じ規則の `jwc/analyze.jw`）が「この `*r` は世代の検査を消してよい」と判断したら、
その時点で `r` は本当に生きている（借りたときの世代と、区画の今の世代が等しい）ことを Lean で証明した。
`sorry` も追加の公理も使っていない（`lake build` が `warningAsError` で通る）。

## 証明したこと

| 定理 | 内容 |
|---|---|
| `elided_access_is_valid` | 事実が正しければ、`safeDeref`（`place` の判断）が真の `*r` は必ず通る |
| `step_sound` | 命令の前に事実が正しければ、解析の規則（`transfer`）を通したあとの事実も、命令のあとで正しい（14 種の命令すべて） |
| `sound_of_sub` / `meet_sub_left` / `meet_sub_right` | 合流（`meet`）で事実を減らしても正しいまま |
| `stable_of_meet` | 繰り返しが止まった（どの辺でも `meet(entry, out) == entry`）なら、各ブロックの入口の事実は前のブロックの出口に含まれる |
| `path_sound` / `elided_access_in_function` | 関数の入口で事実が正しく、解析が止まっていれば、どのブロックのどこで消した `*r` も必ず通る |
| `live_arg_is_valid` | 呼び出しの直前に `is_live` が真の引数は生きた ref で、リストの要素でもない（呼ばれた関数が Pinned と見なしてよい） |
| `old_argument_check_is_unsound` | 以前の引数の判定（読む前の事実を取っておく）の反例 |

## 模型の考え方

区画に「種類」を付ける。種類は、その区画を解放・使い回しできるのが誰かを表す。

| 種類 | 区画 | 変えられる命令 |
|---|---|---|
| `root x` | own の変数 `x` が持つ先 | `x` の drop・ムーブ・入れ直し |
| `slot x` | 変数 `x` そのものの場所 | `x` の drop・ムーブ（入れ直しでは変わらない） |
| `list` | リストの中身 | 要素を動かしうる操作・リストを持ちうる値の上書きや解放 |
| `nested` | 値の中の own の先・受け渡し中の値 | 何でも（事実はここを指さない） |
| `caller` | 引数の ref の先（呼び出し元のもの） | 関数の中では誰も変えない |
| `free` | 空き・呼ばれた関数の中のもの | 何でも |

命令ごとに「どの種類の区画の世代と種類を変えてよいか」を決める（`Exec`）。解析の規則（`transfer`）は analyze.rs の関数を一つずつ写した。

| Lean | analyze.rs |
|---|---|
| `killAll` / `killValue` / `killElems` / `isLive` / `meet` | 同名の関数 |
| `safeDeref` | `place` の `safe` |
| `setFact` | `Inst::Assign` の最後（自分を借りた事実は入れない） |
| `drop x holds` | `Inst::Drop`（`holds` は `may_hold_list`）。`Inst::Discard` は値の読み（`moveOut` など）のあとに、`holds` なら `elems`、そうでなければ `pure` |
| `moveOut x` | `Operand::Move` / `Operand::Read` |
| `assignOwn x holds` | own の型の変数への代入（`copy_out` 以外）。値は先の命令（`moveOut` など）で読み終わっている |
| `assignNone x holds` | 事実の付かない代入（`holds` は射影なしなら `may_hold_list`） |
| `copyRef` / `moveRef` | ref の変数の `copy` / `move`、`ref_fact` の「`follows` で射影なし・own でない」場合 |
| `borrowOwn r y` | `ref_fact` で根が Own の場合（`RefTo`） |
| `reborrow r s live` | `ref_fact` で根が ref の場合（`live` は `*s` を通る形で `is_live` を求める） |
| `borrowSlot` / `borrowElem` | `ref_fact` の `RefSlot` / `RefElem` |
| `writeInside holds` | 射影のある場所への代入（`holds` は `value_may_hold`） |
| `elems` | `kill_elems` を呼ぶ操作（ふつうの呼び出し・関数の値・`CallDyn`・リストの操作・入出力など） |
| `pure` | `kill_elems` を呼ばない操作（数の計算・`scalar_call`・純粋な組み込み） |
| `access r` | `place` で先頭の `*` を判断する所 |

## 前提（証明の外。言語と実装の決まりとして守る）

1. **唯一の持ち主**: own の先を解放・使い回しできるのは、その own を持つ変数への操作だけ。ムーブした own の先は受け渡し中（`nested`）になる。
2. **呼び出しの枠**: 呼ばれた関数は、呼び出し元の変数の先（`root`）・場所（`slot`）・引数の先（`caller`）を解放しない。
   呼ばれた関数の中の区画は、呼び出し元からは `free` と見る。
3. **要素を動かす操作の一覧**: リストの中身を解放・移動しうる操作は、すべて `elems`（`kill_elems`）か `holds` の付いた上書き・解放として扱われている。
   特に、純粋な組み込み（print・eprint・concat・append・assert・`f64.*`・`int.*`・`str.*`）と、len・contains・get・keys はリストの中身を動かさない。
4. **型の決まり**: own の型の変数は `copy` されない（`Pre`）。ref の `x = move x` は作られない。
5. **関数の入口**: Pinned の引数は呼び出し元から生きた ref で渡される（関数をまたぐ繰り返しの正しさは範囲外。`live_arg_is_valid` が呼び出し元の側を与える）。
6. **範囲外**: 値・型の中身、スレッド、2 段目以降の `*`（いつも実行時に検査する）、`LIST_SHARED` の印（何度付けても同じなので省いた）。

## 標準ライブラリに組み込みを足すとき

新しい組み込みは、模型では `elems` か `pure` のどちらか（効き方）だけで決まる。

- リストの中身を動かしうる・呼び出し元の値に届きうる・分からない → `elems`（何もしなければ analyze.rs はこちらに倒す）。
- 数・文字列だけを受け取り、何も解放しない → `pure`。analyze.rs の `pure` の一覧（`Rvalue::Builtin`）と、jwc の同じ一覧に名前を足す。
  このとき前提 3 を新しい組み込みが守っていることを確かめる。

新しい種類の事実や、`ref_fact` の新しい場合を足すときは、`Fact`・`transfer`・`Exec` に足して `step_sound` を直す。

## 証明の途中で見つけて直したこと

呼び出しの引数の判定（`operands`）は、読む前の事実を取っておき、すべて読み終えたあとで `is_live` を見ていた。
`is_live` は `RefSlot` を事実の表を見ずに真とするので、`f(r, y)` で r が y を借りた ref（`RefSlot(y)`）、y が後の引数でムーブされると、
取っておいた事実のせいで安全と判定されうる（`old_argument_check_is_unsound`）。今の下げ方は引数を一度一時変数に入れるので
実際のプログラムではこの形にならないが、両コンパイラで、読んだ値を名前のない変数に置いて `kill_all` が効くようにした。
