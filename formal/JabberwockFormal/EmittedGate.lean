import JabberwockFormal.Jwc.ReferenceAccess
import JabberwockFormal.Rust.ReferenceAccess

set_option autoImplicit false
set_option warningAsError true

namespace JabberwockFormal.EmittedGate

/-- Boolean semantics of the emitted LLVM generation-check sequence.
    This assumes the loaded metadata is a valid accessible i64 address;
    pointer lifetime, atomic concurrency and optimization are not modeled. -/
abbrev Word := BitVec 64
def retired : Word := 18446744073709551615

def llvmDifferent (current saved : Word) : Bool := decide (current ≠ saved)
def llvmRetired (current : Word) : Bool := decide (current = retired)
def llvmBad (saved current : Word) : Bool :=
  llvmDifferent current saved || llvmRetired current

def llvmContinues (saved current : Word) : Bool := !llvmBad saved current

def valid (saved current : Word) : Prop :=
  current = saved ∧ current ≠ retired

theorem llvm_continues_iff (saved current : Word) :
    llvmContinues saved current = true ↔ valid saved current := by
  by_cases same : current = saved <;> by_cases dead : current = retired <;>
    simp [llvmContinues, llvmBad, llvmDifferent, llvmRetired, valid,
          same, dead]

theorem llvm_bad_iff (saved current : Word) :
    llvmBad saved current = true ↔ ¬ valid saved current := by
  by_cases same : current = saved <;> by_cases dead : current = retired <;>
    simp [llvmBad, llvmDifferent, llvmRetired, valid, same, dead]

theorem jwc_gate_correspondence (saved current : Word) :
    JabberwockFormal.Jwc.ReferenceAccess.emittedGate saved current =
      llvmContinues saved current := by
  by_cases same : current = saved <;> by_cases dead : current = retired <;>
    simp [JabberwockFormal.Jwc.ReferenceAccess.emittedGate,
          llvmContinues, llvmBad, llvmDifferent, llvmRetired,
          retired, JabberwockFormal.Jwc.ReferenceAccess.retired, same, dead]

theorem rust_gate_correspondence (saved current : Word) :
    JabberwockFormal.Rust.ReferenceAccess.emittedGate saved current =
      llvmContinues saved current := by
  by_cases same : current = saved <;> by_cases dead : current = retired <;>
    simp [JabberwockFormal.Rust.ReferenceAccess.emittedGate,
          llvmContinues, llvmBad, llvmDifferent, llvmRetired,
          retired, JabberwockFormal.Rust.ReferenceAccess.retired, same, dead]

theorem cross_compiler_gate_equivalence (saved current : Word) :
    JabberwockFormal.Jwc.ReferenceAccess.emittedGate saved current =
      JabberwockFormal.Rust.ReferenceAccess.emittedGate saved current := by
  rw [jwc_gate_correspondence, rust_gate_correspondence]

theorem no_retired_gate (saved : Word) :
    llvmContinues saved retired = false := by
  simp [llvmContinues, llvmBad, llvmRetired]

end JabberwockFormal.EmittedGate
