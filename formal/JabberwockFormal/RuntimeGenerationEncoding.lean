import Lean.Elab.Tactic.Omega

set_option autoImplicit false
set_option warningAsError true

namespace JabberwockFormal.RuntimeGenerationEncoding

/-
  Arithmetic model of runtime/jw_rt.c:
    GEN_COUNTER_MASK = 2^62 - 1
    bit 62 is the retained marker
    UINT64_MAX is the retired sentinel
  The low two state bits are part of the masked counter, as in the C code.
  This proves the arithmetic encoding only; it does not prove C pointer
  lifetime, atomic ordering, or the allocator's storage invariants.
-/

def radix : Nat := 2 ^ 62
def maxCounter : Nat := radix - 1
/-- 4 * 2^62 - 1 = 2^64 - 1, the C runtime's GEN_RETIRED sentinel. -/
def retiredWord : Nat := 4 * radix - 1

/-- Valid non-retired packed word: optional bit-62 marker plus a 62-bit field. -/
def pack (retained : Bool) (counter : Nat) : Nat :=
  (if retained then radix else 0) + counter

/-- Exact arithmetic condition checked by gen_has_room on the masked field. -/
def hasRoom (counter step : Nat) : Prop :=
  counter + step ≤ maxCounter

instance (counter step : Nat) : Decidable (hasRoom counter step) := by
  unfold hasRoom
  infer_instance

/-- On success, gen_advance increments the packed word; otherwise it retires it. -/
def advance (retained : Bool) (counter step : Nat) : Nat :=
  if hasRoom counter step then pack retained (counter + step) else retiredWord

theorem hasRoom_bounds_sum {counter step : Nat}
    (room : hasRoom counter step) :
    counter + step ≤ maxCounter := room

theorem hasRoom_prevents_counter_wrap {counter step : Nat}
    (room : hasRoom counter step) :
    counter + step < radix := by
  have bound := hasRoom_bounds_sum room
  unfold maxCounter at bound
  unfold radix
  omega

theorem packed_word_is_below_retired
    (retained : Bool) {counter : Nat}
    (bounded : counter < radix) :
    pack retained counter < retiredWord := by
  cases retained <;> simp [pack, retiredWord, radix] at * <;> omega

/-- A successful positive increment cannot recreate the previous packed word. -/
theorem advance_changes_word
    (retained : Bool) {counter step : Nat}
    (room : hasRoom counter step) (positive : 0 < step) :
    advance retained counter step ≠ pack retained counter := by
  unfold advance
  rw [if_pos room]
  intro same
  cases retained <;> simp [pack] at same <;> omega

/-- The saved pre-free word cannot pass the runtime's equality gate after a
    successful increment, because the packed generation word has changed. -/
def accepts (saved current : Nat) : Prop :=
  current ≠ retiredWord ∧ saved = current

theorem old_word_rejected_after_advance
    (retained : Bool) {counter step : Nat}
    (room : hasRoom counter step) (positive : 0 < step) :
    ¬ accepts (pack retained counter) (advance retained counter step) := by
  unfold accepts advance
  rw [if_pos room]
  intro accepted
  have changed := advance_changes_word retained room positive
  exact changed accepted.2.symm

/-- If gen_has_room fails, the runtime retires the block; retired words never
    satisfy the reference-generation gate, regardless of the saved word. -/
theorem old_word_rejected_on_retirement
    (saved : Nat) (retained : Bool) (counter step : Nat)
    (noRoom : ¬ hasRoom counter step) :
    ¬ accepts saved (advance retained counter step) := by
  unfold accepts advance
  rw [if_neg noRoom]
  simp

/-- Every modeled live word differs from the retired sentinel. -/
theorem live_word_not_retired
    (retained : Bool) {counter : Nat}
    (bounded : counter < radix) :
    pack retained counter ≠ retiredWord := by
  have below := packed_word_is_below_retired retained bounded
  omega

end JabberwockFormal.RuntimeGenerationEncoding
