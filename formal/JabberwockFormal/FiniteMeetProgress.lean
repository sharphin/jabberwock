import JabberwockFormal.LoopConvergence

set_option autoImplicit false
set_option warningAsError true

namespace JabberwockFormal.FiniteMeetProgress

/-- Finite analysis facts indexed by the local variable number. -/
abbrev Facts (n : Nat) := Fin n → Option Nat

/-- A reachable block retains a fact exactly when both edges agree. -/
def update {n : Nat} (old incoming : Facts n) : Facts n :=
  fun key => if old key = incoming key then old key else none

theorem retained_from_old {n : Nat} (old incoming : Facts n)
    (key : Fin n) (fact : Nat)
    (h : update old incoming key = some fact) :
    old key = some fact := by
  by_cases same : old key = incoming key
  · simpa [update, same] using h
  · simp [update, same] at h

theorem retained_from_incoming {n : Nat} (old incoming : Facts n)
    (key : Fin n) (fact : Nat)
    (h : update old incoming key = some fact) :
    incoming key = some fact := by
  have ho := retained_from_old old incoming key fact h
  by_cases same : old key = incoming key
  · exact same.symm.trans ho
  · simp [update, same] at h

theorem update_idempotent {n : Nat} (old incoming : Facts n) :
    update (update old incoming) incoming = update old incoming := by
  funext key
  by_cases same : old key = incoming key
  · simp [update, same]
  · simp [update, same]

/-- A missing fact remains missing after a reachable-block meet. -/
theorem absence_persists {n : Nat} (old incoming : Facts n)
    (key : Fin n) (absent : old key = none) :
    update old incoming key = none := by
  by_cases same : old key = incoming key
  · simp [update, same, absent]
  · simp [update, same]

/-- Any change to a reachable entry erases some existing positive fact. -/
theorem changed_update_erases_fact {n : Nat} (old incoming : Facts n)
    (changed : update old incoming ≠ old) :
    ∃ key : Fin n, ∃ fact : Nat,
      old key = some fact ∧ update old incoming key = none := by
  have differing : ∃ key : Fin n, update old incoming key ≠ old key := by
    apply Classical.byContradiction
    intro noDifference
    apply changed
    funext key
    by_cases equal : update old incoming key = old key
    · exact equal
    · exact False.elim (noDifference ⟨key, equal⟩)
  obtain ⟨key, different⟩ := differing
  cases oldValue : old key with
  | none =>
      exact False.elim (different ((absence_persists old incoming key oldValue).trans oldValue.symm))
  | some fact =>
      have erased : update old incoming key = none := by
        by_cases same : old key = incoming key
        · exact False.elim (different (by simp [update, same]))
        · simp [update, same]
      exact ⟨key, fact, oldValue, erased⟩

/-- Apply any finite sequence of predecessor merges. -/
def updateMany {n : Nat} (old : Facts n) : List (Facts n) → Facts n
  | [] => old
  | incoming :: rest => updateMany (update old incoming) rest

/-- Repeated merges never resurrect a removed fact. -/
theorem absent_after_updates {n : Nat} (old : Facts n)
    (incoming : List (Facts n)) (key : Fin n)
    (absent : old key = none) :
    updateMany old incoming key = none := by
  induction incoming generalizing old with
  | nil => exact absent
  | cons head tail ih =>
      exact ih (update old head) (absence_persists old head key absent)

end JabberwockFormal.FiniteMeetProgress
