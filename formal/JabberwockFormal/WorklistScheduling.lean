import JabberwockFormal.WorklistExecution

set_option autoImplicit false
set_option warningAsError true

namespace JabberwockFormal.WorklistScheduling

open JabberwockFormal.FiniteMeetProgress
open JabberwockFormal.WorklistExecution

/-- The actual IR worklist queues a successor when its entry differs from
    the newly joined entry. The successor list here is a *single* prescribed
    item: outgoing CFG traversal and transfer functions are not modeled. -/
def enqueueIfChanged {blocks locals : Nat}
    (old incoming : Facts locals)
    (item : Incoming blocks locals)
    (pending : List (Incoming blocks locals)) :
    List (Incoming blocks locals) :=
  if (List.finRange locals).all (fun key => decide (old key = incoming key)) then
    pending else item :: pending

/-- A stable entry cannot cause the successor to be queued again. -/
theorem stable_entry_does_not_enqueue {blocks locals : Nat}
    (old incoming : Facts locals)
    (item : Incoming blocks locals)
    (pending : List (Incoming blocks locals))
    (stable : ∀ key, old key = incoming key) :
    enqueueIfChanged old incoming item pending = pending := by
  simp [enqueueIfChanged, List.all_eq_true, stable]

/-- Changed facts do cause a pending item to be added. -/
theorem changed_entry_enqueues {blocks locals : Nat}
    (old incoming : Facts locals)
    (item : Incoming blocks locals)
    (pending : List (Incoming blocks locals))
    (changed : ∃ key, old key ≠ incoming key) :
    enqueueIfChanged old incoming item pending = item :: pending := by
  obtain ⟨key, different⟩ := changed
  have notAll : (List.finRange locals).all (fun k => decide (old k = incoming k)) = false := by
    cases h : (List.finRange locals).all (fun k => decide (old k = incoming k)) with
    | false => rfl
    | true =>
        have same : old key = incoming key := by
          have all := (List.all_eq_true.mp h) key (List.mem_finRange key)
          exact of_decide_eq_true all
        exact False.elim (different same)
  simp [enqueueIfChanged, notAll]

/-- Every new queue item due to a reachable-block entry change has a
    concrete erased fact as a progress witness. -/
theorem enqueue_requires_erasure {blocks locals : Nat}
    (old incoming : Facts locals)
    (item : Incoming blocks locals)
    (pending : List (Incoming blocks locals))
    (scheduled : enqueueIfChanged old incoming item pending =
      item :: pending)
    (notAlreadyHead : pending ≠ item :: pending) :
    ∃ key : Fin locals, ∃ fact : Nat,
      old key = some fact ∧ update old incoming key = none := by
  by_cases stable : ∀ key, old key = incoming key
  · have unchanged := stable_entry_does_not_enqueue old incoming item pending stable
    have impossible : pending = item :: pending := unchanged.symm.trans scheduled
    exact False.elim (notAlreadyHead impossible)
  · have differing : ∃ key, old key ≠ incoming key := by
      exact Classical.not_forall.mp stable
    have changed : update old incoming ≠ old := by
      intro eq
      obtain ⟨key, diff⟩ := differing
      have h : update old incoming key = old key := congrFun eq key
      by_cases same : old key = incoming key
      · exact diff same |>.elim
      · cases hv : old key with
        | none => exact False.elim (diff (by simpa [hv] using same))
        | some fact => simp [update, same, hv] at h
    exact changed_update_erases_fact old incoming changed

end JabberwockFormal.WorklistScheduling
