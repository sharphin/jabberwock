# CLAUDE.md

自作言語 ward（仮名）の Rust 実装。`own` と世代付き `ref` だけで「C++より安全、Rustより簡単、Goより速い」を狙う。

## 作業ルール（オーナーの指示）

- 変更は **main に直接 push してよい**（PR は不要）。push 前に `cargo test` と `cargo clippy --all-targets` を通すこと。
- オーナーとのやりとりは日本語、平易な言葉で。
- ソースコードについて質問されたら答えられるよう、下の「コードの地図」と不変条件を最新に保つ。
- 外部クレートは使わない。Rust 1.65 以上で動くこと（`is_some_and` など新しい API は避ける）。
- `cargo fmt` はリポジトリ全体ではかけていない（元のコードが rustfmt 形式ではないため）。周りの書き方に合わせる。

## コードの地図

| ファイル | 役割 |
|---|---|
| `src/lexer.rs` | 字句解析。改行はただの空白（文の区切りは `;`） |
| `src/parser.rs` | 構文解析。`if`/`while` の条件では構造体リテラルを禁止（`{` があいまいになるため）。`x += e` は `x = x + e`、`x++` は `x = x + 1` に書き換える（左辺は複製） |
| `src/ast.rs` | 構文木。`Deref` と `Field` は静的解析の結論 `Cell<Check>` を持つ。整数の型 `IntTy`（範囲・一周・自動変換の規則）もここ |
| `src/types.rs` | 型検査。実行前に型と `mut` / `ref mut` の違反を見つける（`lib.rs` で解析・実行の前に呼ぶ） |
| `src/analyze.rs` | 静的解析。証明できた `*p` / `p.f` の ref 世代チェックを消す |
| `src/interp.rs` | 木をたどるインタプリタ。ヒープはスロット＋世代、変数は Live / Moved |
| `src/diag.rs` | 「借りた → 解放された → 使った」を時系列で出すエラー表示 |
| `src/lib.rs` | `run_source` / `run_source_with`（解析なしで比較できる） |
| `tests/examples.rs` | 言語の基本動作 |
| `tests/analyze.rs` | 検査を消す解析。消せる例と、消してはいけない例 |
| `tests/structs.rs` | 構造体 |
| `tests/control.rs` | loop / while / for / break / continue / if 式 |
| `tests/match.rs` | match（Java の switch の矢印形式） |
| `tests/types.rs` | 型検査 |
| `tests/enums.rs` | enum と Option・match での取り出し・`match ref` |
| `tests/generics.rs` | ジェネリクス（関数・構造体・enum の型の引数） |
| `tests/results.rs` | Result と `?`（途中で抜けたときの解放も） |
| `tests/methods.rs` | メソッド（`this`・`fn mut`・構造体の値への呼び出し・静的メソッド） |
| `tests/interfaces.rs` | interface（Go 風）と `<T: Shape>` / `<T: ordered>`・interface を型として使う |
| `tests/strings.rs` | 文字列の埋め込み `"x = {x}"` |
| `tests/arrays.rs` | 配列（`i64[3]`・添字・定数の添字の範囲チェック） |
| `tests/lists.rs` | リスト `i64[]`（push / pop / len）と `for x in a` |
| `tests/ints.rs` | 整数型・数の書き方・自動変換・`as` |
| `tests/text.rs` | 文字列のメソッドと入力（`read_line()` / `args()`） |
| `tests/borrow.rs` | own でない値を借りる・`ref` の引数を自動で借りる |
| `tests/mutability.rs` | `mut` / `ref mut` / `;` / ブロックの値 / 複合代入 / `++` |

## 大事な不変条件

- `own` は唯一の所有者。値として読むとムーブ。構造体も値として読むとムーブ（中に own を持ちうるため）。
- 値を捨てるときは `release` で中の own を（構造体のフィールドも含めて）すべて解放する。リークはテストで `live_objects == 0` を確認。
- 変数の値もヒープのスロットに置く（`VarState::Live(slot)`）。ムーブ・スコープ終了で `vacate` して世代を進める（変数を借りた ref が無効になる）。
  `RefVal` は `slot` / `generation` に加えてスロットの中の道筋 `path`（フィールドと添字）を持つ。own の先を指すなら空。
  道筋はリストの中を通らない（`through_list` / types.rs の `inside_list` で弾く）。enum の中も通らない（`place` が通さない）。
  `check_ptr` は (スロット, 道筋) を返し、`at` / `at_mut` で中の値をたどる。
- `ref T` の引数に `ref` なしで場所を渡すと、types.rs の `auto_borrow` が `Expr.borrow` を立てる。interp（`eval` / `eval_borrow`）と
  analyze（`expr`）はそれを `ref e` と同じに扱う（`make_ref` / `ref_of`）。関数とメソッドの引数だけ（enum の中身・構造体のフィールドは対象外）。
- 静的解析の根拠: own 変数 `a` のスロットを解放できるのは `a` という名前を使う操作（再代入・ムーブ・`drop`・スコープ終了）だけ。呼び出された関数は呼び出し元の変数に触れない。
- 解析は安全側。分からないものは `Check::Required`（実行時検査）。デバッグビルドでは消した検査も `debug_assert!` で確かめる。
- 解析とインタプリタの「評価順」と「ムーブする／しない（`eval` と `eval_borrow`）」は必ず一致させる。
- `break` / `continue` で抜けるときは、抜けるスコープの変数の事実を消してから合流させる（`facts_leaving`）。
- `if` / `match` は式（`ExprKind::If` / `ExprKind::Match`）。文として使うときは `Stmt::Expr` に入り、中の return / break を `Flow` で外へ伝える（`eval_flow`）。
  値として使うときの中の return / 外への break はパーサが禁止しているので（`value_context`）、`eval` では `Flow` を気にしなくてよい。
- 書き換えの可否は `Access`（Write / ImmutVar / ReadOnlyRef）で表す。`place()` が持ち主から順にたどって決め、
  `ref mut` を通ると Write のまま、`ref` を通ると ReadOnlyRef になる。`RefVal.mutable` が「ref mut か」。
  同じ規則を `types.rs` が型から静的に決める（`Access::through` に型を渡す）。実行時の検査は念のため残している。
  二つの規則は必ずそろえる。
- 型検査は「前から順に決めるだけ」。`ref mut T` は `ref T` として渡せる（`assignable`）。必ず抜けるブロックは `Ty::Never`。
- 戻り値の型がある関数では、本体の最後の `;` のない式が戻り値（`call_fn` が `exec_block_value(body, true)`）。
  戻り値のない関数では最後の式の値は捨てる。
- 整数の値は `Value::Int(i128, IntTy)`。計算は i128 でして、型の幅で一周させる（`IntTy::wrap`）。
- 型検査は整数の式に「この型で使う」という印 `Expr.int` をつける（数の書き方を相手に合わせる・自動の変換）。
  interp は `eval` / `eval_flow` の最後にその型へ変換する（`convert`）。印をつけるのは `types.rs` の `convert` / `set_tree` だけ。
- 整数のあふれは一周する。定数の式（`is_const_tree`）の結果が型に入らないときだけ `types.rs` が警告を出す（そのまま書いた数ならエラー）。
  警告は `Diagnostic::warning` で、実行は止めない（`Outcome.warnings`、CLI は実行前に表示）。
- enum の値は `Value::Enum { variant, fields, copy }`。`copy` は型検査が `ExprKind::Variant` の `Cell<bool>` に書く
  （own や構造体を持ちうる型ならムーブ。`Value::moves`）。型の引数を含んで決められないときは None で、interp が中身を見て決める。
  Option と Result は `lib.rs` の `PRELUDE` を `parse` がプログラムの前に足す。`Some(x)` / `None` / `Ok(x)` / `Err(e)` はパーサが `Option.Some` などにする。
- enum の match は interp の `match_enum`。値の match は中身を受け取る名前へ移し、借りた match（scrut が ref）は
  中身の own を ref にして渡す。全部の種類を書いたかは `types.rs` の `match_enum` が調べる。
  case で受け取る名前は、analyze でも新しい変数として宣言する（外の同名の変数の事実を使わない）。
- パーサはファイル中の `enum 名前` を先に集め、`名前.種類` を enum の値として読む（`enum_names`）。
- ブロックの値は `Block.tail` が true のときだけ（最後の文が `;` のない式）。`}` で終わる文（`is_block_like`）には `;` 不要。

- ジェネリクス: 型検査だけの話で、interp は型を見ない（値はそのまま動く）。`Ty::Param` は定義の中の `T`、
  `Ty::Struct/Enum(名前, 型の引数)`。使う側では `instantiate` が引数（ふつうの式 → 数の式 → enum の値の順）と
  入れる先の型から `unify` で型の引数を決め、`subst` で置き換える。`T` は copyable ではない（安全側）。

- `e?`（`ExprKind::Try`）で抜けるときは、interp が `unwinding` に返す値を入れて `Err` を返し、`call_fn` が受け止めて return にする。
  途中のスコープは frame に残ったままなので `call_fn` がまとめて解放する。評価途中の値（引数・構造体のフィールド・print の一時値）は
  `eval_all` / `discard` で解放する。新しく「値を集めながら評価する」場所を足すときは、同じように解放すること（リークする）。
  analyze では `?` で抜ける道は関数の中で合流しないので、続く道だけ考えればよい。

- メソッドは、パーサが `構造体名.名前` という普通の関数にする（最初の引数 `this` は `ref [mut] 構造体<T..>`、
  型の引数は構造体のもの＋メソッドのもの）。本体のフィールド名・メソッド名は `bind_this` が `this.名前` に書き換える
  （引数・ローカル変数が同じ名前なら書き換えない）。呼び出しは `ExprKind::MethodCall`。
  interp の `eval_method`: 一時的な receiver は先に評価、場所の receiver は引数のあとで借りる（analyze も同じ順）。
  場所にある構造体の値の receiver はその場所を借りて this にする。一時的な値とリストの中の receiver だけヒープへ貸し（`Lent`）、呼び出し後に解放して元の場所へ戻す。
  analyze は receiver の型を知らないので、同じ名前のメソッドすべての呼び出しとして記録する（安全側）。
  静的メソッド（`static fn`）は this のない `構造体名.名前` の関数。パーサが `struct 名前` を先に集め（`struct_names`）、
  `Point.at(...)` を `ExprKind::Call("Point.at")` にする。型検査の `FnSig.method` で呼び方の取り違えを弾く。

- interface と条件: 型検査だけの話（interp は値の構造体名でメソッドを探すので何もしない）。`FnDef.bounds` が型の引数ごとの条件。
  `types.rs` の `satisfies` が「構造体のメソッドが interface の形とそろうか」を調べ、`check_bounds` で呼び出しの場所で確かめる。
  関数の中では `Checker.bounds` を見て、`T` のメソッド呼び出しと比較（`is_ordered_param`）を許す。
  interface を型として使うときは `Ty::Iface`（own / ref の先にだけ書ける。`resolve_pointee`）。`own S` → `own I`、
  `ref S` → `ref I` は `Checker::fits`（`assignable` に interface を足したもの）で許す。interp は何もしない（値の構造体名でメソッドを探す）。
  メソッド呼び出しは `T` も interface も `iface_method_call`。if / match の枝は、入れる先が interface なら `join` がそれにまとめる。

- 文字列の埋め込み: lexer が `{...}` の中を別に字句解析して `Tok::Interp`（位置は元のファイルに直す）、
  パーサが同じ enum / 構造体の名前で式に読む（`ExprKind::Interp`）。値は print と同じく `eval_borrow` で借りて読む（analyze も borrow）。

- 文字列のメソッドは types.rs の `str_method`（型）と interp の `str_method`（動き）。長さと位置は文字単位。
  入力は `Interp::set_io` で渡す（CLI は標準入力、テストは `run_source_with_input`）。`read_line` / `args` は組み込み関数（`BUILTINS`）。

- 配列: `Value::Array`（要素にムーブする値があればムーブ）。`Place.path` は構造体のフィールドと配列の添字の両方をたどる。
  `a[i]`（`ExprKind::Index`）は **添字を先に評価してから** 配列の場所を求める（interp の `place` / `eval_borrow`、analyze も同じ順）。
  定数の添字の範囲チェックは `types.rs`、実行時は `bounds`。`len()` は `eval_method` / `method_call` の特別扱い。
  リストは `Value::Array(_, true)`（いつもムーブ）・`Ty::List`。`[...]` がリストかは型検査が `ArrayLit` の `Cell<bool>` に書く。
  `push` / `pop` は `eval_method` が場所の中のリストをじかに書き換える（丸ごと複製しない）。
  `for x in a`（`Stmt::ForEach`）は `eval_borrow` で借り、始めた時点の要素を回す。own の要素は `RefVal` にして渡す。

## 設計の決めごと

文法や仕様で決めたことは `docs/design-decisions.md` に記録する。
今の言語の決まりは `docs/spec.md`（言語仕様）にまとめる。仕様を変えたら同じコミットで spec.md も直す。
