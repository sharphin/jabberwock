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

end JabberwockFormal.FiniteMeetProgress
