import JabberwockFormal.LoopConvergence

set_option autoImplicit false
set_option warningAsError true

namespace JabberwockFormal.FiniteMeetProgress

/-- Finite analysis facts indexed by the local variable number. -/
abbrev Facts (n : Nat) := Fin n → Option Nat

/-- A reachable block retains a fact exactly when both edges agree. -/
def update {n : Nat} (old incoming : Facts n) : Facts n :=
  fun key => if old key = incoming key then old key else none

/-- A meet never introduces a nonempty fact not already in the old state. -/
theorem retained_from_old {n : Nat} (old incoming : Facts n)
    (key : Fin n) (fact : Nat)
    (h : update old incoming key = some fact) :
    old key = some fact := by
  by_cases same : old key = incoming key
  · simpa [update, same] using h
  · simp [update, same] at h

/-- A meet also cannot introduce a fact missing on the incoming edge. -/
theorem retained_from_incoming {n : Nat} (old incoming : Facts n)
    (key : Fin n) (fact : Nat)
    (h : update old incoming key = some fact) :
    incoming key = some fact := by
  have ho := retained_from_old old incoming key fact h
  by_cases same : old key = incoming key
  · exact same.symm.trans ho
  · simp [update, same] at h

/-- Repeating an identical predecessor meet does not change the result. -/
theorem update_idempotent {n : Nat} (old incoming : Facts n) :
    update (update old incoming) incoming = update old incoming := by
  funext key
  by_cases same : old key = incoming key
  · simp [update, same]
  · simp [update, same]

/-- If a reachable block's meet changes its state, there is a concrete
    local whose previously known fact has been erased. -/
theorem changed_update_erases_fact {n : Nat} (old incoming : Facts n)
    (changed : update old incoming ≠ old) :
    ∃ key : Fin n, ∃ fact : Nat,
      old key = some fact ∧ update old incoming key = none := by
  by_contra missing
  apply changed
  funext key
  by_cases same : old key = incoming key
  · simp [update, same]
  · cases h : old key with
    | none => simp [update, same, h]
    | some fact =>
        have erased : update old incoming key = none := by
          simp [update, same]
        have contradiction : False := missing ⟨key, fact, h, erased⟩
        exact False.elim contradiction

/-- No new positive analysis fact can appear after a reachable-block
    update, even when an arbitrary incoming path arrives later. -/
theorem absence_persists {n : Nat} (old incoming : Facts n)
    (key : Fin n) (absent : old key = none) :
    update old incoming key = none := by
  by_cases same : old key = incoming key
  · simp [update, same, absent]
  · simp [update, same]

/-- A finite sequence of updates cannot resurrect an erased fact.
    The statement is for a single local and any number of incoming edges. -/
def updateMany {n : Nat} (old : Facts n) : List (Facts n) → Facts n
  | [] => old
  | incoming :: rest => updateMany (update old incoming) rest

theorem absent_after_updates {n : Nat} (old : Facts n)
    (incoming : List (Facts n)) (key : Fin n)
    (absent : old key = none) :
    updateMany old incoming key = none := by
  induction incoming generalizing old with
  | nil => exact absent
  | cons head tail ih =>
      exact ih (update old head) (absence_persists old head key absent)

end JabberwockFormal.FiniteMeetProgress
