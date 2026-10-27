import Init.Data.Int.DivMod.Lemmas
import Init.Data.Int.Bitwise
import Lean.Elab.Tactic.Omega

set_option autoImplicit false
set_option warningAsError true

namespace JabberwockFormal.MachineIntegers

/-!
Integer value semantics for spec sections 3.3 and 6.2. This is a specification,
not an extraction of either compiler. Operands have already been evaluated and
coerced by the type checker. Source parsing, expression evaluation order, floats,
heap effects, LLVM instructions and implementation correspondence remain open.
No theorem here discharges a whole-compiler obligation.
-/

inductive IntTy where
  | i8 | i16 | i32 | i64 | u8 | u16 | u32 | u64 | usize
  deriving DecidableEq, Repr

def bits : IntTy → Nat
  | .i8 | .u8 => 8
  | .i16 | .u16 => 16
  | .i32 | .u32 => 32
  | .i64 | .u64 | .usize => 64

def signed : IntTy → Bool
  | .i8 | .i16 | .i32 | .i64 => true
  | _ => false

def modulus (t : IntTy) : Int := 2 ^ bits t
def half (t : IntTy) : Int := 2 ^ (bits t - 1)
def lower (t : IntTy) : Int := if signed t then -half t else 0
def upper (t : IntTy) : Int := if signed t then half t else modulus t
def InRange (t : IntTy) (x : Int) : Prop := lower t ≤ x ∧ x < upper t

def wrap (t : IntTy) (x : Int) : Int :=
  let u := x % modulus t
  if signed t then (if half t ≤ u then u - modulus t else u) else u

theorem modulus_positive (t : IntTy) : 0 < modulus t := by
  cases t <;> decide

theorem half_positive (t : IntTy) : 0 < half t := by
  cases t <;> decide

theorem modulus_twice_half (t : IntTy) : modulus t = 2 * half t := by
  cases t <;> decide

/-- Wrapping is defined for every mathematical integer, including overflow. -/
theorem wrap_in_range (t : IntTy) (x : Int) : InRange t (wrap t x) := by
  have hm := modulus_positive t
  have hh := half_positive t
  have htwice := modulus_twice_half t
  have hlo := Int.emod_nonneg x (by omega : modulus t ≠ 0)
  have hhi := Int.emod_lt_of_pos x hm
  cases hs : signed t with
  | false => simpa [InRange, wrap, lower, upper, hs] using And.intro hlo hhi
  | true =>
      simp only [InRange, wrap, lower, upper, hs, Bool.true_eq_true, ite_true]
      split <;> omega

theorem wrap_eq_self (t : IntTy) (x : Int) (bounded : InRange t x) :
    wrap t x = x := by
  have hm := modulus_positive t
  have hh := half_positive t
  have htwice := modulus_twice_half t
  cases hs : signed t with
  | false =>
      have bounds : 0 ≤ x ∧ x < modulus t := by
        simpa [InRange, lower, upper, hs] using bounded
      simp [wrap, hs, Int.emod_eq_of_lt bounds.1 bounds.2]
  | true =>
      have bounds : -half t ≤ x ∧ x < half t := by
        simpa [InRange, lower, upper, hs] using bounded
      by_cases negative : x < 0
      · have reduced : x % modulus t = x + modulus t := by
          have h := Int.emod_eq_of_lt
            (by omega : 0 ≤ x + modulus t)
            (by omega : x + modulus t < modulus t)
          simpa only [Int.add_emod_right] using h
        have high : half t ≤ x + modulus t := by omega
        simp [wrap, hs, reduced, high]
      · have reduced : x % modulus t = x :=
          Int.emod_eq_of_lt (by omega) (by omega)
        have low : ¬half t ≤ x := by omega
        simp [wrap, hs, reduced, low]

theorem wrap_idempotent (t : IntTy) (x : Int) :
    wrap t (wrap t x) = wrap t x := wrap_eq_self t _ (wrap_in_range t x)

/-- A value carries a proof of its range, not an unbounded Nat payload. -/
structure Value (t : IntTy) where
  integer : Int
  bounded : InRange t integer

def pack (t : IntTy) (x : Int) : Value t := ⟨wrap t x, wrap_in_range t x⟩
def cast {from : IntTy} (to : IntTy) (v : Value from) : Value to := pack to v.integer

def widensTo (from to : IntTy) : Bool :=
  if from = to then true
  else if from = .usize ∨ to = .usize then
    decide (to = .usize ∧ (from = .u8 ∨ from = .u16 ∨ from = .u32))
  else if signed from = signed to then decide (bits from ≤ bits to)
  else if signed from = false ∧ signed to = true then decide (bits from < bits to)
  else false

def common (a b : IntTy) : Option IntTy :=
  if widensTo a b then some b else if widensTo b a then some a else none

/-- The nine-type implicit conversion table never loses a representable value. -/
theorem widening_preserves_range (a b : IntTy) (x : Int)
    (allowed : widensTo a b = true) (input : InRange a x) : InRange b x := by
  cases a <;> cases b <;>
    simp_all [widensTo, InRange, lower, upper, modulus, half, bits, signed] <;> omega

theorem widening_preserves_value (a b : IntTy) (v : Value a)
    (allowed : widensTo a b = true) : (cast b v).integer = v.integer := by
  exact wrap_eq_self b v.integer (widening_preserves_range a b v.integer allowed v.bounded)

theorem usize_is_distinct :
    widensTo .u64 .usize = false ∧ widensTo .usize .u64 = false := by decide

inductive Fault where
  | divisionByZero
  | negativeShift
  deriving DecidableEq, Repr

inductive Arithmetic where
  | add | sub | mul
  deriving DecidableEq, Repr

def arithmetic (t : IntTy) (op : Arithmetic) (a b : Value t) : Value t :=
  pack t (match op with
    | .add => a.integer + b.integer
    | .sub => a.integer - b.integer
    | .mul => a.integer * b.integer)

inductive Bitwise where
  | and | or | xor
  deriving DecidableEq, Repr

def bitwise (t : IntTy) (op : Bitwise) (a b : Value t) : Value t :=
  pack t (match op with
    | .and => a.integer &&& b.integer
    | .or => a.integer ||| b.integer
    | .xor => a.integer ^^^ b.integer)

def complement {t : IntTy} (a : Value t) : Value t := pack t (-a.integer - 1)

/-- The static precondition rejects unary minus on unsigned types. -/
def negate (t : IntTy) (_permitted : signed t = true) (a : Value t) : Value t :=
  pack t (-a.integer)

inductive Comparison where
  | eq | ne | lt | le | gt | ge
  deriving DecidableEq, Repr

def compare {t : IntTy} (op : Comparison) (a b : Value t) : Bool :=
  match op with
  | .eq => decide (a.integer = b.integer)
  | .ne => decide (a.integer ≠ b.integer)
  | .lt => decide (a.integer < b.integer)
  | .le => decide (a.integer ≤ b.integer)
  | .gt => decide (a.integer > b.integer)
  | .ge => decide (a.integer ≥ b.integer)

inductive Quotient where
  | div | rem
  deriving DecidableEq, Repr

/-- Truncate towards zero. Lean's ordinary Int division has different rounding. -/
def quotient (t : IntTy) (op : Quotient) (a b : Int) : Except Fault (Value t) :=
  if b = 0 then .error .divisionByZero
  else .ok (pack t (match op with
    | .div => a.tdiv b
    | .rem => a.tmod b))

theorem zero_divisor_rejected (t : IntTy) (op : Quotient) (a : Int) :
    quotient t op a 0 = .error .divisionByZero := by simp [quotient]

theorem quotient_success_iff (t : IntTy) (op : Quotient) (a b : Int) :
    (∃ v, quotient t op a b = .ok v) ↔ b ≠ 0 := by
  by_cases hz : b = 0
  · simp [quotient, hz]
  · simp [quotient, hz]

inductive Shift where
  | left | right
  deriving DecidableEq, Repr

/-- Guard before converting the count to Nat or computing a power of two.
    A large count has a constant result; it is never masked modulo the width. -/
def shift (t : IntTy) (op : Shift) (a : Value t) (count : Int) : Except Fault (Value t) :=
  if count < 0 then .error .negativeShift
  else if (bits t : Int) ≤ count then
    .ok (pack t (match op with
      | .left => 0
      | .right => if a.integer < 0 then -1 else 0))
  else .ok (pack t (match op with
    | .left => a.integer * 2 ^ count.toNat
    | .right => a.integer / 2 ^ count.toNat))

theorem negative_shift_rejected (t : IntTy) (op : Shift) (a : Value t)
    (count : Int) (negative : count < 0) :
    shift t op a count = .error .negativeShift := by simp [shift, negative]

theorem shift_success_iff (t : IntTy) (op : Shift) (a : Value t) (count : Int) :
    (∃ v, shift t op a count = .ok v) ↔ 0 ≤ count := by
  by_cases negative : count < 0
  · have notNonnegative : ¬0 ≤ count := by omega
    simp [shift, negative, notNonnegative]
  · have nonnegative : 0 ≤ count := by omega
    by_cases large : (bits t : Int) ≤ count <;> simp [shift, negative, nonnegative, large]

theorem oversized_left_is_zero (t : IntTy) (a : Value t) (count : Int)
    (nonnegative : 0 ≤ count) (large : (bits t : Int) ≤ count) :
    shift t .left a count = .ok (pack t 0) := by
  simp [shift, large, show ¬count < 0 from by omega]

/-- Range preservation includes the specified error exits. It does not prove
    that the production type checker supplies these operand types. -/
theorem successful_value_in_range {t : IntTy} (result : Except Fault (Value t))
    (v : Value t) (_success : result = .ok v) : InRange t v.integer := v.bounded

/-! Concrete boundary obligations, checked with proof terms when Lean is run. -/

theorem i8_add_wraps : wrap .i8 (127 + 1) = -128 := by decide
theorem u64_mul_wraps :
    wrap .u64 (18446744073709551615 * 18446744073709551615) = 1 := by decide
theorem i8_min_div_neg_one :
    (quotient .i8 .div (-128) (-1)).map Value.integer = .ok (-128) := by decide
theorem i8_min_rem_neg_one :
    (quotient .i8 .rem (-128) (-1)).map Value.integer = .ok 0 := by decide
theorem negative_division_truncates :
    (quotient .i64 .div (-7) 3).map Value.integer = .ok (-2) := by decide
theorem remainder_has_dividend_sign :
    (quotient .i64 .rem (-7) 3).map Value.integer = .ok (-1) := by decide
theorem negative_right_shift_rounds_down :
    (shift .i8 .right (pack .i8 (-7)) 1).map Value.integer = .ok (-4) := by decide
theorem signed_large_right_shift :
    (shift .i64 .right (pack .i64 (-1)) 18446744073709551615).map Value.integer =
      .ok (-1) := by decide
theorem unsigned_large_right_shift :
    (shift .u64 .right (pack .u64 18446744073709551615) 64).map Value.integer =
      .ok 0 := by decide
theorem narrowing_wraps : (cast .u8 (pack .i64 300)).integer = 44 := by decide

end JabberwockFormal.MachineIntegers
