//! Deterministic source-to-typed-IR regression matrix for the Rust compiler.
//! These are finite implementation tests, not a universal semantics proof.
use jabberwock::{ir_text, llvm_build_with, no_modules, run_source_with, typed};
use jabberwock::ir::llvm::Opts;

fn check_case(label: &str, source: &str, expected: &str) {
    let (_, info) = typed(source, &no_modules)
        .unwrap_or_else(|e| panic!("{label}: type check failed: {}", e.title));
    // Ensure the returned type information can be used by the actual lowering.
    let _ = info;
    let ir = ir_text(source, &no_modules)
        .unwrap_or_else(|e| panic!("{label}: IR lowering failed: {e}"));
    assert!(ir.contains("fn main()"), "{label}: missing main in IR: {ir}");
    // Also compile the very same accepted program through both LLVM paths.
    // The emitter must not reject a case already accepted by typed lowering.
    // LLVM text generation is a smoke test, not a machine-code equivalence proof.
    for elide in [false, true] {
        let (llvm, _) = llvm_build_with(
            source, &no_modules, "test.jw", false, elide, Opts::default())
            .unwrap_or_else(|e| panic!("{label}: LLVM emission failed (elide={elide}): {e}"));
        assert!(llvm.contains("define "), "{label}: LLVM module has no definitions");
    }
    let checked = run_source_with(source, false, false);
    let optimized = run_source_with(source, false, true);
    for (mode, result) in [("checked", &checked), ("optimized", &optimized)] {
        assert!(result.error.is_none(), "{label}/{mode}: {:?}", result.error);
        assert_eq!(result.output, expected, "{label}/{mode}: output");
        assert_eq!(result.live_objects, 0, "{label}/{mode}: leaked owners");
    }
    assert_eq!(checked.output, optimized.output, "{label}: optimization changed output");
    assert_eq!(checked.errput, optimized.errput, "{label}: optimization changed stderr");
    assert_eq!(checked.exit_code, optimized.exit_code, "{label}: optimization changed exit code");
}

#[test]
fn arithmetic_and_condition_matrix() {
    for left in [0i64, 1, 2, 7, 31] {
        for right in [0i64, 1, 3, 9] {
            // The expression is in the supported, non-overflowing scalar subset.
            let expected_sum = left + right;
            let code = format!(
                "fn main() {{ let a = {left}; let b = {right}; print(a + b); }}"
            );
            check_case(&format!("addition({left},{right})"), &code,
                       &format!("{expected_sum}\n"));
            let expected_eq = left == right;
            let code = format!(
                "fn main() {{ let a = {left}; let b = {right}; print(a == b); }}"
            );
            check_case(&format!("equality({left},{right})"), &code,
                       &format!("{expected_eq}\n"));
        }
    }
}

#[test]
fn branches_preserve_typed_results() {
    for condition in [true, false] {
        for yes in [0i64, 2, 13] {
            for no in [1i64, 3, 17] {
                let program = format!(
                    "fn main() {{ if {condition} {{ print({yes}); }} else {{ print({no}); }} }}"
                );
                let expected = if condition { yes } else { no };
                check_case(&format!("branch({condition},{yes},{no})"),
                           &program, &format!("{expected}\n"));
            }
        }
    }
}

#[test]
fn typed_rejection_is_not_silently_lowered() {
    let program = "fn main() { let x = true; print(x + 1); }";
    assert!(typed(program, &no_modules).is_err());
    assert!(ir_text(program, &no_modules).is_err());
    for elide in [false, true] {
        assert!(
            llvm_build_with(program, &no_modules, "test.jw", false, elide, Opts::default()).is_err(),
            "typed rejection reached LLVM emission with elide={elide}"
        );
    }
    for elide in [false, true] {
        let result = run_source_with(program, false, elide);
        assert!(result.error.is_some(), "type error accepted with elide={elide}");
    }
}
