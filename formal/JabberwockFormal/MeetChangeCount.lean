import JabberwockFormal.ErasureHistory

set_option autoImplicit false
set_option warningAsError true

namespace JabberwockFormal.MeetChangeCount

open JabberwockFormal.FiniteMeetProgress

/-- Count *actual strict state changes* of a reachable block when processing
    a concrete finite list of predecessor facts. Unchanged merges count zero. -/
noncomputable def changeCount {n : Nat} (initial : Facts n) :
    List (Facts n) → Nat
  | [] => 0
  | incoming :: rest => by
      classical
      exact (if update initial incoming = initial then 0 else 1) +
        changeCount (update initial incoming) rest

/-- Every input edge can cause at most one change. This is a list-length
    bound, distinct from the stronger number-of-locals bound. -/
theorem changeCount_le_history_length {n : Nat}
    (initial : Facts n) (history : List (Facts n)) :
    changeCount initial history ≤ history.length := by
  classical
  induction history generalizing initial with
  | nil =>
      exact Nat.le_refl 0
  | cons incoming rest ih =>
      by_cases stable : update initial incoming = initial
      · simpa [changeCount, stable] using
          Nat.le_trans (ih (update initial incoming)) (Nat.le_succ rest.length)
      · have bound := Nat.succ_le_succ (ih (update initial incoming))
        simpa only [changeCount, if_neg stable, List.length_cons,
          Nat.succ_eq_add_one, Nat.add_comm] using bound

/-- An empty reachable entry has no changes, no matter how many incoming
    predecessor edges it processes. -/
theorem absent_entry_has_zero_changes {n : Nat}
    (initial : Facts n) (history : List (Facts n))
    (absent : ∀ key : Fin n, initial key = none) :
    changeCount initial history = 0 := by
  classical
  induction history generalizing initial with
  | nil =>
      rfl
  | cons incoming rest ih =>
      have stable : update initial incoming = initial :=
        empty_entry_stable incoming initial absent
      simpa [changeCount, stable] using ih initial absent

/-- When the initial entry is already empty, its change count is zero;
    in particular, the n=0 case has the sharp numeric bound zero. -/
theorem zero_locals_have_zero_changes
    (initial : Facts 0) (history : List (Facts 0)) :
    changeCount initial history = 0 := by
  apply absent_entry_has_zero_changes
  intro key
  exact Fin.elim0 key

/-- A strict change contributes exactly one, then counting continues from
    the actual updated state. -/
theorem strict_change_count {n : Nat}
    (initial incoming : Facts n) (rest : List (Facts n))
    (strict : update initial incoming ≠ initial) :
    changeCount initial (incoming :: rest) =
      changeCount (update initial incoming) rest + 1 := by
  classical
  simp [changeCount, strict, Nat.add_comm]

end JabberwockFormal.MeetChangeCount
