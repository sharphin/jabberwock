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
  if update old incoming = old then pending else item :: pending

/-- A stable entry cannot cause the successor to be queued again. -/
theorem stable_entry_does_not_enqueue {blocks locals : Nat}
    (old incoming : Facts locals)
    (item : Incoming blocks locals)
    (pending : List (Incoming blocks locals))
    (stable : update old incoming = old) :
    enqueueIfChanged old incoming item pending = pending := by
  simp [enqueueIfChanged, stable]

/-- Changed facts do cause a pending item to be added. -/
theorem changed_entry_enqueues {blocks locals : Nat}
    (old incoming : Facts locals)
    (item : Incoming blocks locals)
    (pending : List (Incoming blocks locals))
    (changed : update old incoming ≠ old) :
    enqueueIfChanged old incoming item pending = item :: pending := by
  simp [enqueueIfChanged, changed]

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
  by_cases stable : update old incoming = old
  · have impossible : pending = item :: pending := by
      simpa [enqueueIfChanged, stable] using scheduled
    exact False.elim (notAlreadyHead impossible)
  · exact changed_update_erases_fact old incoming stable

end JabberwockFormal.WorklistScheduling
