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


/-- The optimizer chooses which accesses omit generation checks.
    Keeping a check is always sound; an omitted check requires validity. -/
structure PlannedAccess where
  access : Access
  elide : Bool

def optimizedRead (p : PlannedAccess) : Option Nat :=
  if p.elide then
    uncheckedRead p.access.saved p.access.current p.access.payload
  else
    checkedRead p.access.saved p.access.current p.access.payload

def checkedPlan (plan : List PlannedAccess) : List (Option Nat) :=
  plan.map (fun p => checkedRead p.access.saved p.access.current p.access.payload)

def optimizedPlan (plan : List PlannedAccess) : List (Option Nat) :=
  plan.map optimizedRead

/-- A per-access soundness obligation. This is the condition that the real
    compiler analysis must establish before eliding a given check. -/
def PlanValid (plan : List PlannedAccess) : Prop :=
  ∀ p ∈ plan, p.elide = true →
    JabberwockFormal.EmittedGate.valid p.access.saved p.access.current

private theorem optimized_read_eq_iff (p : PlannedAccess) :
    checkedRead p.access.saved p.access.current p.access.payload =
      optimizedRead p ↔
    (p.elide = true →
      JabberwockFormal.EmittedGate.valid p.access.saved p.access.current) := by
  cases h : p.elide with
  | false =>
      simp [optimizedRead, h]
  | true =>
      simp only [optimizedRead, h, ite_true]
      constructor
      · intro equal _
        exact (elision_equivalent_iff_valid
          p.access.saved p.access.current p.access.payload).mp equal
      · intro valid
        exact (elision_equivalent_iff_valid
          p.access.saved p.access.current p.access.payload).mpr (valid rfl)

/-- Exact characterization of selective check elimination over a whole trace,
    not just the sufficient condition for eliding every check. -/
theorem plan_semantics_iff_valid (plan : List PlannedAccess) :
    checkedPlan plan = optimizedPlan plan ↔ PlanValid plan := by
  induction plan with
  | nil =>
      simp [checkedPlan, optimizedPlan, PlanValid]
  | cons p rest ih =>
      simp only [checkedPlan, optimizedPlan, List.map_cons, List.cons.injEq]
      constructor
      · intro equal
        have head := (optimized_read_eq_iff p).mp equal.1
        have tail := ih.mp equal.2
        intro q member elided
        rcases List.mem_cons.mp member with equal | later
        · subst q
          exact head elided
        · exact tail q later elided
      · intro valid
        constructor
        · apply (optimized_read_eq_iff p).mpr
          intro elided
          exact valid p (List.mem_cons_self ..) elided
        · apply ih.mpr
          intro q member elided
          exact valid q (List.mem_cons_of_mem p member) elided

end JabberwockFormal.CheckElision
