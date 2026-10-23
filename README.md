# ward

`own` と世代付き `ref` だけで「速い・簡単・安全」の隙間を狙う、小さな言語の核（v0.1・インタプリタ）。
名前は jabberwock に変える予定（時期は未定）。

## 動かす

```
cargo run -- examples/counter.ward
cargo run -- --stats examples/counter.ward     # ref の検査をどれだけ消せたか表示
cargo run -- --no-elide examples/counter.ward  # 静的解析なし（全部実行時に検査）
cargo test
```

## 言語の見た目

```
fn inc(ref mut i64 r) {   // 型は「型 名前」の順。ref mut i64 = 書き換えてよい i64 の借用
    *r += 1;
}

fn main() {
    let mut count = own 0;   // own: 唯一の所有者。書き換えるので mut
    inc(ref mut count);      // ref mut: 書き換えてよい借用（ライフタイム注釈は存在しない）
    print(*count);           // 1
}
```

- ポインタは `own` と `ref` の2種類だけ。生ポインタはない。
- `own x` を読むと **ムーブ**。元の変数はそれ以降使えない。
- `ref p` は **世代付き参照**。所有者が解放されると、使った瞬間に確実に検出される。
- スコープを抜ける・`drop(x)`・再代入で `own` は解放される。中の `own` も連鎖して解放。
- 型は「型 名前」の順で、関数の引数・戻り値・構造体のフィールドに書く（`fn i64 add(i64 a, i64 b)`）。
  `let` は書かなくてよい（初期値から決まる）。書くなら `let i64 mut x = 0;`。
- 型は整数（`i8` `i16` `i32` `i64` `u8` `u16` `u32` `u64` `usize`）・`bool`・`str`・構造体の名前・`own T`・`ref T`・`ref mut T`。
  型の違反と mut の違反は実行前に見つかる。
- 数の書き方は使う先の型に合わせる（`let u8 b = 200;`）。入らない数（`let u8 b = 300;`）はエラー。型が決まらなければ `i64`。
- 値が欠けない整数の変換（`u8` → `i64` など）は自動。欠けうる変換は `x as u8` と書く（範囲の外は一周する）。
- 戻り値のある関数は、本体の最後の `;` のない式が戻り値になる（`return 値;` でもよい）。
- 文の終わりには `;`。改行はただの空白。
- 変数は普段は書き換え不可。書き換えるものだけ `let mut`（引数も `fn f(mut a)`）。
- `ref` は読むだけ、`ref mut` は書き換えてよい借用。`ref mut` で借りられるのは mut の持ち主だけ。
- `x += 1` などの複合代入と、文としての `i++` / `i--` がある（`print(i++)` のように式の中では使えない）。
- 値は整数・真偽値・文字列・構造体。`let` / `=` / `if` / `else` / `match` / `loop` / `while` / `for` / `break` / `continue` / `return` / `fn` / `struct`。組み込みは `print` `drop` `assert`。

## ループと if 式

```
for i in range(1, 10, 2) {            // range(終わり) / range(始め, 終わり) / range(始め, 終わり, 刻み)
    let kind = if i % 3 == 0 { "3の倍数" } else { "ふつう" };   // if は値にもなる
    print(i, kind);
}

for i in range(0) {
} else {
    print("1 回も回らなかったときだけ実行");   // Python の for-else とは意味が違う
}

loop {                                // 無限ループ。break / continue が使える
    break;
}
```

- 条件は `( )` で囲まない。`{ }` は必須。
- 値として使う if には else が必要。値はブロックの最後の **`;` のない式**（Rust と同じ）。
- `}` で終わる文（if・while・for・loop・match）のあとには `;` は要らない。
- do-while と三項演算子 `? :` はない。

```
let name = match n {                  // Java の switch の矢印形式。値にも文にもなる
    case 0 -> "zero";
    case 1, 2 -> "small";
    default -> "many";
};
```

言語仕様は [docs/spec.md](docs/spec.md)、決めた理由は [docs/design-decisions.md](docs/design-decisions.md) にまとめている。

## 構造体

```
struct Point { i64 x; i64 y; }
struct Player { str name; own Point pos; i64 score; }

fn move_by(ref mut Player p, i64 dx, i64 dy) {
    p.pos.x += dx;   // p は ref mut、p.pos は own。どちらも自動でたどる
}

fn main() {
    let mut hero = own Player { name: "勇者", pos: own Point { x: 0, y: 0 }, score: 0 };
    move_by(ref mut hero, 3, 4);
    hero.pos = own Point { x: 9, y: 9 };  // 古い Point はここで解放される
}
```

- フィールドを書き換えられるかは持ち主で決まる（`let mut` の変数か、`ref mut` を通しているか）。
- フィールドごとに所有する。構造体が解放されると、own のフィールドも連鎖して解放される。
- 構造体は値として読むとムーブ（中に own を持ちうるため）。
- `p.f` は `p` が own / ref なら自動で先をたどる。ref をたどるときの世代チェックも静的解析の対象。
- `ref p.f` でフィールドの own を借りられる。上書きされた後に使うと、時系列つきで検出される。
- フィールドから own や構造体を値として取り出すことはできない（`ref` で借りるか、`p.f = ...` で置き換える）。
- `if` / `while` の条件では構造体リテラルを書けない（`{` があいまいになるため）。括弧で囲めば書ける。

## enum と Option

```
enum Shape {
    Circle(i64 r);          // 中身は「型 名前」
    Rect(i64 w, i64 h);
    Empty;
}

fn i64 area(Shape s) {
    match s {                         // 全部の種類を書かないとエラー（default でまとめてもよい）
        case Circle(r) -> 3 * r * r;  // 中身を名前で受け取る
        case Rect(w, h) -> w * h;
        case Empty -> 0;
    }
}

fn Option<i64> find(i64 n) { ... }   // null はない。ないかもしれない値は Option
let x = Some(5);
let Option<i64> y = None;            // None だけでは型が決まらないので型を書く
```

- 値は `Shape.Circle(2)` のように作る。Option は `Some(x)` と `None`。
- 中身に own や構造体を持ちうる enum は、構造体と同じく値として読むとムーブ。中身が数などだけならコピー。
- `match ref s`（`s` は own か ref）は借りるだけ。中身の own は ref として受け取る。
- Option は最初からある普通の enum（`enum Option<T> { Some(T value); None; }`）。

## メソッド

```
struct Point {
    i64 x;
    i64 y;

    fn i64 len2() { x * x + y * y }          // this. は書かなくてよい
    fn mut move_by(i64 dx) { x += dx; }      // 書き換えるメソッドは fn mut
}
let mut p = Point { x: 3, y: 4 };
p.move_by(1);
print(p.len2());
```

## interface（Go 風）

```
interface Shape { fn i64 area(); }
fn describe<T: Shape>(ref T s) { print(s.area()); }   // area() を持つ構造体なら何でも渡せる
fn T max<T: ordered>(T a, T b) { if a > b { a } else { b } }
fn i64 area_of(ref Shape s) { s.area() }               // interface は型としても使える
```

## Result と `?`

```
fn Result<i64, str> div(i64 a, i64 b) {
    if b == 0 { Err("0 では割れません") } else { Ok(a / b) }
}
fn Result<i64, str> calc(i64 a, i64 b) {
    let x = div(a, b)?;    // Err ならそのまま返す
    Ok(x * 2)
}
```

- 例外はない。失敗は Result で返し、`?` で上へ渡す。

## ジェネリクス

```
fn T pick<T>(bool first, T a, T b) { if first { a } else { b } }
struct Pair<T> { T a; T b; }
let p = Pair { a: 1, b: 2 };          // Pair<i64>。型の引数は使う側では書かない
let Pair<str> q = Pair { a: "x", b: "y" };
```

- 型の引数は、引数・フィールド・入れる先の型から決まる。`T` の値どうしの計算や比較はまだできない。

## エラーは時系列で出る

```
error: 無効になった参照を使いました (dangling reference)
 --> examples/dangling_scope.ward:9:11
  |
7 |         r = ref a;
  |             ----- ここで `a` を借りた
8 |     }
  |     - ここで解放された: `a` はスコープを抜けたので解放された
9 |     print(*r)
  |           ^^ ここで使ったが、借りた先はもう存在しない
  |
  = note: 参照は、借りた元の所有者より長くは生きられません
```

## 静的解析で検査を消す

実行前に `src/analyze.rs` がプログラムを読み、「借りた先は必ず生きている」と証明できた `*r` の
世代チェックを消す。証明できない箇所だけ実行時の検査が残る。

- `let r = ref a;` のあと、`a` が再代入・ムーブ・`drop`・スコープ終了されていない間の `*r` は安全。
  （`a` のスロットを解放できるのは `a` という名前を使う操作だけなので）
- 関数の引数 `r` は、すべての呼び出し元が「生きている ref」を渡しているなら、関数の中ずっと安全。
  関数をまたいで繰り返し計算するので、再帰や ref の受け渡しにも効く。
- `if` の合流や `while` では、どちらの道でも成り立つことだけを残す（安全側）。
- デバッグビルドでは、消した検査も念のため確かめ、解析の誤りがあれば panic する。

```
$ cargo run -- --stats examples/counter.ward
count = 3
[stats] `*p` と `p.f` の箇所: 検査を消せた 3 / 実行時に残る 0
[stats] ref の世代チェック: 実行 0 回 / 省略 6 回
```

## 現状の正直な限界

- 証明できなかった箇所の検査は **実行時**。ダングリングはコンパイルエラーではなく、確実に検出される実行時エラー。
- 解析は保守的。`own` を返す関数の戻り値、ヒープに入れた ref、`ref p.f`（フィールドの借用）は追跡しない（検査が残る）。
- 今は木をたどるインタプリタなので、検査を消しても速度差は小さい。効いてくるのはネイティブコード生成後。
- 整数のあふれはその型の幅で一周する（定数どうしの計算なら警告が出る）。
- 小数（`f32` / `f64`）・配列はまだない。
- クロージャ・並行処理は未実装。
- 解放履歴（`Heap::history`）は消さないので、長時間走らせるとメモリが増える。

## 次の一手（難しくなる順）

1. ~~構造体を足す（フィールドごとの所有）~~（v0.3）。次は `ref p.f` も解析で追えるようにする
2. ~~静的解析で「この `ref` は絶対安全」と証明できた箇所の検査を消す~~（v0.2 で最初の版）
3. ~~型検査~~・~~整数型~~・~~Option と enum~~・~~ジェネリクス~~・~~Result と `?`~~・~~メソッド~~。次は静的メソッド・文字列の埋め込み・配列など
4. 並行処理でのデータ競合防止（最大の山）
5. C / Cranelift / LLVM へのコード生成、セルフホスト
