import JabberwockFormal.EmittedGate

set_option autoImplicit false
set_option warningAsError true

namespace JabberwockFormal.CheckElision

/-!
A narrow, executable semantics for a generation-checked read and its
unchecked counterpart. The precondition for check elimination must be
proved independently at each real compiler transformation site.

This is NOT a proof about actual LLVM memory loads, pointer provenance,
concurrent writes, or either compiler's check-elision implementation.
-/

abbrev Word := BitVec 64

/-- The checked read either yields the payload or refuses the access. -/
def checkedRead (saved current : Word) (payload : Nat) : Option Nat :=
  if JabberwockFormal.EmittedGate.llvmContinues saved current
  then some payload else none

/-- Eliminating the check has precisely the indicated extra obligation. -/
def uncheckedRead (_saved _current : Word) (payload : Nat) : Option Nat :=
  some payload

/-- A valid generation witness suffices for check elimination in this model. -/
theorem elision_preserves_valid_read (saved current : Word) (payload : Nat)
    (valid : JabberwockFormal.EmittedGate.valid saved current) :
    checkedRead saved current payload = uncheckedRead saved current payload := by
  have h := (JabberwockFormal.EmittedGate.llvm_continues_iff saved current).mpr valid
  simp [checkedRead, uncheckedRead, h]

/-- Invalid generations must be rejected by the checked variant. -/
theorem checked_read_rejects_invalid (saved current : Word) (payload : Nat)
    (invalid : ¬ JabberwockFormal.EmittedGate.valid saved current) :
    checkedRead saved current payload = none := by
  have h : JabberwockFormal.EmittedGate.llvmContinues saved current = false := by
    cases decision : JabberwockFormal.EmittedGate.llvmContinues saved current with
    | false => rfl
    | true =>
        exact False.elim (invalid
          ((JabberwockFormal.EmittedGate.llvm_continues_iff saved current).mp decision))
  simp [checkedRead, h]

/-- Without the generation witness, unchecked reads can change the result. -/
theorem elision_changes_invalid_read (saved current : Word) (payload : Nat)
    (invalid : ¬ JabberwockFormal.EmittedGate.valid saved current) :
    checkedRead saved current payload ≠ uncheckedRead saved current payload := by
  rw [checked_read_rejects_invalid saved current payload invalid]
  simp [uncheckedRead]

/-- In particular, retirement is never an acceptable elision witness. -/
theorem retired_read_must_keep_check (saved : Word) (payload : Nat) :
    checkedRead saved JabberwockFormal.EmittedGate.retired payload = none := by
  simp [checkedRead, JabberwockFormal.EmittedGate.no_retired_gate]

end JabberwockFormal.CheckElision
