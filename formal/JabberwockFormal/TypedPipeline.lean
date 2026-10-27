import JabberwockFormal.ScopedGoals

set_option autoImplicit false
set_option warningAsError true

namespace JabberwockFormal.TypedPipeline

/-!
A deliberately small, intrinsically typed source and two compiler stages.
The intermediate pass performs constant folding; the back end changes
representation to a target expression. Both stages preserve evaluation.
This is a G1/G3 specification fragment, NOT a correspondence proof for
jwc, the Rust compiler, LLVM, or the runtime.
-/

inductive Ty where
  | nat
  | bool

inductive Expr : Ty → Type where
  | number (n : Nat) : Expr .nat
  | truth (b : Bool) : Expr .bool
  | add (a b : Expr .nat) : Expr .nat
  | equal (a b : Expr .nat) : Expr .bool
  | choose {t : Ty} (condition : Expr .bool) (yes no : Expr t) : Expr t

def Value : Ty → Type
  | .nat => Nat
  | .bool => Bool

def eval : {t : Ty} → Expr t → Value t
  | _, .number n => n
  | _, .truth b => b
  | _, .add a b => eval a + eval b
  | _, .equal a b => decide (eval a = eval b)
  | _, .choose condition yes no =>
      if eval condition then eval yes else eval no

/-- A separate IR representation retains its result type in the constructor. -/
inductive IR : Ty → Type where
  | integer (n : Nat) : IR .nat
  | boolean (b : Bool) : IR .bool
  | sum (a b : IR .nat) : IR .nat
  | eq (a b : IR .nat) : IR .bool
  | select {t : Ty} (condition : IR .bool) (yes no : IR t) : IR t

def irEval : {t : Ty} → IR t → Value t
  | _, .integer n => n
  | _, .boolean b => b
  | _, .sum a b => irEval a + irEval b
  | _, .eq a b => decide (irEval a = irEval b)
  | _, .select condition yes no =>
      if irEval condition then irEval yes else irEval no

def lower : {t : Ty} → Expr t → IR t
  | _, .number n => .integer n
  | _, .truth b => .boolean b
  | _, .add a b => .sum (lower a) (lower b)
  | _, .equal a b => .eq (lower a) (lower b)
  | _, .choose condition yes no =>
      .select (lower condition) (lower yes) (lower no)

/-- Type preservation is enforced by the indexed lowering signature. -/
theorem lower_preserves_result_type {t : Ty} (e : Expr t) :
    ∃ result : IR t, result = lower e := by
  exact ⟨lower e, rfl⟩

/-- Behavioral preservation for every expression in this small typed language. -/
theorem lower_preserves_evaluation {t : Ty} (e : Expr t) :
    irEval (lower e) = eval e := by
  induction e with
  | number _ => rfl
  | truth _ => rfl
  | add a b ha hb => simp [lower, irEval, eval, ha, hb]
  | equal a b ha hb => simp [lower, irEval, eval, ha, hb]
  | choose condition yes no hc hy hn =>
      simp [lower, irEval, eval, hc, hy, hn]


/-- A typed IR rewrite that folds literal addition, literal equality and
    literal conditionals, leaving nonconstant operations intact. -/
def fold : {t : Ty} → IR t → IR t
  | _, .integer n => .integer n
  | _, .boolean b => .boolean b
  | _, .sum a b =>
      let a' := fold a
      let b' := fold b
      match a', b' with
      | .integer x, .integer y => .integer (x + y)
      | x, y => .sum x y
  | _, .eq a b =>
      let a' := fold a
      let b' := fold b
      match a', b' with
      | .integer x, .integer y => .boolean (decide (x = y))
      | x, y => .eq x y
  | _, .select c yes no =>
      let c' := fold c
      let yes' := fold yes
      let no' := fold no
      match c' with
      | .boolean true => yes'
      | .boolean false => no'
      | other => .select other yes' no'

/-- The optimizer cannot change the result type: its return type is IR t. -/
theorem fold_preserves_result_type {t : Ty} (ir : IR t) :
    ∃ optimized : IR t, optimized = fold ir := by
  exact ⟨fold ir, rfl⟩

/-- A standalone preservation obligation for each optimization category.
    No implementation correspondence with the real jwc/Rust passes is claimed. -/
theorem fold_constant_addition (a b : Nat) :
    irEval (fold (IR.sum (.integer a) (.integer b))) = a + b := by
  rfl

theorem fold_constant_equality (a b : Nat) :
    irEval (fold (IR.eq (.integer a) (.integer b))) = decide (a = b) := by
  rfl

theorem fold_constant_conditional {t : Ty} (yes no : IR t) :
    irEval (fold (IR.select (.boolean true) yes no)) =
      irEval (fold yes) := by
  rfl

end JabberwockFormal.TypedPipeline
