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


/-- A valid generation is exactly the condition under which this one-read
    optimization agrees with the checked program. This is deliberately
    an iff, preventing unsound promotion based on only one direction. -/
theorem elision_equivalent_iff_valid (saved current : Word) (payload : Nat) :
    checkedRead saved current payload = uncheckedRead saved current payload ↔
      JabberwockFormal.EmittedGate.valid saved current := by
  constructor
  · intro equal
    by_cases good : JabberwockFormal.EmittedGate.valid saved current
    · exact good
    · exact False.elim
        ((elision_changes_invalid_read saved current payload good) equal)
  · intro good
    exact elision_preserves_valid_read saved current payload good

/-- Even a matching saved marker must not turn retired storage into a value. -/
theorem retired_equal_generation_rejected (payload : Nat) :
    checkedRead JabberwockFormal.EmittedGate.retired
      JabberwockFormal.EmittedGate.retired payload = none :=
  retired_read_must_keep_check _ payload


/-- One abstract access records its saved/current generation and payload.
    The real compiler-to-model correspondence is an additional obligation. -/
structure Access where
  saved : Word
  current : Word
  payload : Nat

/-- Observable results of a sequence of checked accesses. -/
def checkedTrace (accesses : List Access) : List (Option Nat) :=
  accesses.map (fun a => checkedRead a.saved a.current a.payload)

/-- Observable results of the same sequence with every check omitted. -/
def uncheckedTrace (accesses : List Access) : List (Option Nat) :=
  accesses.map (fun a => uncheckedRead a.saved a.current a.payload)

/-- Check elimination over a trace is correct if *every* eliminated check
    has its own valid-generation witness. A single successful read is not
    sufficient evidence for all subsequent accesses. -/
theorem valid_trace_elision_preserves_results
    (accesses : List Access)
    (validEverywhere : ∀ a ∈ accesses,
      JabberwockFormal.EmittedGate.valid a.saved a.current) :
    checkedTrace accesses = uncheckedTrace accesses := by
  induction accesses with
  | nil => rfl
  | cons a rest ih =>
      have headValid : JabberwockFormal.EmittedGate.valid a.saved a.current :=
        validEverywhere a (by simp)
      have tailValid : ∀ b ∈ rest,
          JabberwockFormal.EmittedGate.valid b.saved b.current := by
        intro b member
        exact validEverywhere b (by simp [member])
      have headEqual := elision_preserves_valid_read
        a.saved a.current a.payload headValid
      have tailEqual := ih tailValid
      simpa [checkedTrace, uncheckedTrace, headEqual] using
        congrArg (fun t => checkedRead a.saved a.current a.payload :: t) tailEqual


end JabberwockFormal.CheckElision
