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
| `tests/ints.rs` | 整数型・数の書き方・自動変換・`as` |
| `tests/mutability.rs` | `mut` / `ref mut` / `;` / ブロックの値 / 複合代入 / `++` |

## 大事な不変条件

- `own` は唯一の所有者。値として読むとムーブ。構造体も値として読むとムーブ（中に own を持ちうるため）。
- 値を捨てるときは `release` で中の own を（構造体のフィールドも含めて）すべて解放する。リークはテストで `live_objects == 0` を確認。
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
  （own や構造体を持ちうる型ならムーブ。`Value::moves`）。Option は enum 名 "Option" の特別扱い。
- enum の match は interp の `match_enum`。値の match は中身を受け取る名前へ移し、借りた match（scrut が ref）は
  中身の own を ref にして渡す。全部の種類を書いたかは `types.rs` の `match_enum` が調べる。
  case で受け取る名前は、analyze でも新しい変数として宣言する（外の同名の変数の事実を使わない）。
- パーサはファイル中の `enum 名前` を先に集め、`名前.種類` を enum の値として読む（`enum_names`）。
- ブロックの値は `Block.tail` が true のときだけ（最後の文が `;` のない式）。`}` で終わる文（`is_block_like`）には `;` 不要。

## 設計の決めごと

文法や仕様で決めたことは `docs/design-decisions.md` に記録する。
今の言語の決まりは `docs/spec.md`（言語仕様）にまとめる。仕様を変えたら同じコミットで spec.md も直す。
