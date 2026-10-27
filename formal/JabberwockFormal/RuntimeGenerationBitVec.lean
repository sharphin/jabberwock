import Lean.Elab.Tactic.Omega

set_option autoImplicit false
set_option warningAsError true

namespace JabberwockFormal.RuntimeGenerationBitVec

/-
  BitVec-level model of the uint64_t arithmetic in runtime/jw_rt.c.
  Unlike the Nat model, this module uses 64-bit modular operations directly.
  The source matcher binds this model to a narrow, exact C source pattern;
  that structural binding is not a proof of C compiler correctness or of
  pointer lifetime, concurrency, allocator behavior, or the whole runtime.
-/

abbrev Word := BitVec 64

def radix : Nat := 2 ^ 62
def limitMaximum : Nat := radix - 1
def wordMaximum : Nat := 2 ^ 64 - 1

def retiredWord : Word := BitVec.ofNat 64 wordMaximum
def counterMask : Word := BitVec.ofNat 64 limitMaximum

/-- Exact mathematical interpretation of the C unsigned comparisons in
    gen_has_room. The step bound is explicit so limit - step is non-wrapping
    on every accepted path. The masked expression remains a BitVec operation. -/
def guardSpec (gen step limit : Word) : Prop :=
  gen ≠ retiredWord ∧
  step ≤ limit ∧
  (gen &&& counterMask) ≤ limit - step

def guard (gen step limit : Word) : Bool :=
  decide (gen ≠ retiredWord) && step.ule limit &&
    (gen &&& counterMask).ule (limit - step)

theorem guard_iff_spec (gen step limit : Word) :
    guard gen step limit = true ↔ guardSpec gen step limit := by
  simp [guard, guardSpec, BitVec.ule_eq_decide_le]

theorem guard_rejects_retired (step limit : Word) :
    guard retiredWord step limit = false := by
  simp [guard]

/-- On the accepted branch, uint64_t subtraction is ordinary natural
    subtraction: the explicit unsigned comparison prevents underflow. -/
theorem unsigned_subtraction_matches_nat
    (limit step : Word) (ordered : step ≤ limit) :
    (limit - step).toNat = limit.toNat - step.toNat := by
  exact BitVec.toNat_sub_of_le ordered

/-- If the packed runtime word has bit 63 clear and the configured limit is
    below 2^62, a successful guard proves that uint64_t addition does not wrap.
    This is an arithmetic theorem about the BitVec model, not a pointer-safety
    or concurrency theorem. -/
theorem accepted_increment_no_wrap
    (gen step limit : Word)
    (validGen : gen.toNat < 2 ^ 63)
    (validLimit : limit.toNat < radix)
    (accepted : guard gen step limit = true) :
    (gen + step).toNat = gen.toNat + step.toNat := by
  have spec := (guard_iff_spec gen step limit).1 accepted
  have stepLe := spec.2.1
  have stepLeNat : step.toNat ≤ limit.toNat := by
    rw [BitVec.le_def] at stepLe
    exact stepLe
  have stepBound : step.toNat < radix := by
    unfold radix
    omega
  have numericBound : 2 ^ 63 + 2 ^ 62 < 2 ^ 64 := by
    decide
  have sumBelow : gen.toNat + step.toNat < 2 ^ 63 + 2 ^ 62 := by
    exact Nat.add_lt_add validGen (by simpa [radix] using stepBound)
  have sumBound : gen.toNat + step.toNat < 2 ^ 64 := by
    exact lt_trans sumBelow numericBound
  exact BitVec.toNat_add_of_lt sumBound

/-- A positive accepted increment changes the concrete 64-bit word. -/
theorem accepted_increment_changes_word
    (gen step limit : Word)
    (validGen : gen.toNat < 2 ^ 63)
    (validLimit : limit.toNat < radix)
    (accepted : guard gen step limit = true)
    (positive : 0 < step.toNat) :
    gen + step ≠ gen := by
  intro same
  have natSame := congrArg BitVec.toNat same
  rw [accepted_increment_no_wrap gen step limit validGen validLimit accepted] at natSame
  omega

end JabberwockFormal.RuntimeGenerationBitVec
