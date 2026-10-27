import JabberwockFormal.LoopConvergence

set_option autoImplicit false
set_option warningAsError true

namespace JabberwockFormal.FiniteMeetProgress

/-- One finite analysis entry is a vector of optional facts.
    Each position represents one local variable and cannot gain new
    information through the intersection operation. -/
def update {α : Type} [DecidableEq α]
    (old incoming : List (Option α)) : List (Option α) :=
  (old.zip incoming).map fun (a, b) => if a = b then a else none

/-- A fact retained by a finite-vector meet was present at the same
    position in the previous state. The proof uses ordinary list
    membership; index-specific correspondence remains separate. -/
theorem update_retained_from_old {α : Type} [DecidableEq α]
    (old incoming : List (Option α)) (fact : α)
    (h : some fact ∈ update old incoming) :
    some fact ∈ old := by
  unfold update at h
  simp only [List.mem_map] at h
  obtain ⟨pair, hp, heq⟩ := h
  obtain ⟨a, b⟩ := pair
  have ha : a = some fact := by
    by_cases same : a = b
    · simpa [same] using heq
    · simp [same] at heq
  have hold : a ∈ old := (List.mem_zip.mp hp).1
  simpa [ha] using hold

end JabberwockFormal.FiniteMeetProgress
