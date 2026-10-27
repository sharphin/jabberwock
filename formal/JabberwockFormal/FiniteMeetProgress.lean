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
  have differing : ∃ key : Fin n, update old incoming key ≠ old key := by
    if h : ∀ key : Fin n, update old incoming key = old key then
      have equal : update old incoming = old := funext h
      exact False.elim (changed equal)
    else
      exact Classical.not_forall.mp h
  obtain ⟨key, hk⟩ := differing
  cases ho : old key with
  | none =>
      have hn : update old incoming key = none := by
        exact absence_persists old incoming key ho
      exact False.elim (hk (hn.trans ho.symm))
  | some fact =>
      have erased : update old incoming key = none := by
        by_cases same : old key = incoming key
        · exact False.elim (hk (by simp [update, same]))
        · simp [update, same]
      exact ⟨key, fact, ho, erased⟩


