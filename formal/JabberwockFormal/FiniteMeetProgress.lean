import JabberwockFormal.LoopConvergence

set_option autoImplicit false
set_option warningAsError true

namespace JabberwockFormal.FiniteMeetProgress

/-- A finite block-local domain of possible analysis facts. Each element
    represents a particular (variable, fact) pair; inconsistent pairs are
    excluded by the actual analyzer's representation, not by this model. -/
def update (old incoming : Finset α) [DecidableEq α] : Finset α :=
  old ∩ incoming

/-- The analysis state cannot grow after a reachable block's meet update. -/
theorem update_subset [DecidableEq α]
    (old incoming : Finset α) : update old incoming ⊆ old := by
  exact Finset.inter_subset_left

/-- Therefore the number of retained facts cannot grow. -/
theorem update_card_le [DecidableEq α]
    (old incoming : Finset α) :
    (update old incoming).card ≤ old.card := by
  exact Finset.card_le_card (update_subset old incoming)

/-- A single join cannot add a fact that was missing from the old state. -/
theorem update_cannot_create [DecidableEq α]
    (old incoming : Finset α) (fact : α)
    (newFact : fact ∈ update old incoming) : fact ∈ old := by
  exact (update_subset old incoming) newFact

/-- The reachable-block join is idempotent if the incoming facts have
    already been intersected; repeated propagation of an identical edge
    cannot keep modifying that state. -/
theorem repeat_same_edge [DecidableEq α]
    (old incoming : Finset α) :
    update (update old incoming) incoming = update old incoming := by
  simp [update, Finset.inter_assoc]

/-- No additional fact disappears if the next incoming edge includes every
    fact currently retained at that block. -/
theorem stable_if_supported [DecidableEq α]
    (old incoming : Finset α) (supported : old ⊆ incoming) :
    update old incoming = old := by
  exact Finset.inter_eq_left.mpr supported

end JabberwockFormal.FiniteMeetProgress
