//! Differential regression oracle for the Rust interpreter's ref-check elision.
//! This tests implementation behavior, not Lean soundness or LLVM equivalence.
//! A failed run is a bug candidate; a passing run is not a formal proof.

use jabberwock::{run_source_with, Outcome};

fn compare(label: &str, source: &str) {
    let checked = run_source_with(source, false, false);
    let elided = run_source_with(source, false, true);
    compare_outcomes(label, source, &checked, &elided);
}

fn compare_outcomes(label: &str, source: &str, checked: &Outcome, elided: &Outcome) {
    let context = format!("case: {label}\nprogram:\n{source}");
    assert_eq!(checked.output, elided.output, "stdout changed: {context}");
    assert_eq!(checked.errput, elided.errput, "stderr changed: {context}");
    assert_eq!(checked.exit_code, elided.exit_code, "exit code changed: {context}");
    assert_eq!(
        checked.error.as_ref().map(|e| &e.title),
        elided.error.as_ref().map(|e| &e.title),
        "diagnostic changed: {context}"
    );
    assert_eq!(
        checked.live_objects, elided.live_objects,
        "heap live-object count changed: {context}"
    );
    assert_eq!(checked.ref_checks_elided, 0, "disabled mode elided a check: {context}");
}

#[test]
fn live_borrows_across_bounded_loops() {
    for limit in [0, 1, 2, 3, 8, 32] {
        for start in [-7, 0, 1, 29] {
            let source = format!(
                "fn main() {{\n    let mut owner = own {start};\n    let r = ref mut owner;\n    let mut i = 0;\n    while i < {limit} {{\n        *r = *r + 1;\n        i = i + 1;\n    }}\n    print(*owner)\n}}"
            );
            compare(&format!("live:limit={limit},start={start}"), &source);
        }
    }
}

#[test]
fn owner_revocation_at_each_loop_position() {
    for limit in [1, 2, 3, 8] {
        for drop_at in 0..limit {
            let source = format!(
                "fn main() {{\n    let owner = own 11;\n    let r = ref owner;\n    let mut i = 0;\n    while i < {limit} {{\n        if i == {drop_at} {{\n            drop(owner);\n        }}\n        print(*r);\n        i = i + 1;\n    }}\n}}"
            );
            compare(&format!("revocation:limit={limit},drop_at={drop_at}"), &source);
        }
    }
}

#[test]
fn owner_reassignment_and_branch_merges() {
    for reassigned in [false, true] {
        for enter_branch in [false, true] {
            let condition = if enter_branch { "true" } else { "false" };
            let action = if reassigned { "owner = own 99;" } else { "print(*r);" };
            let source = format!(
                "fn main() {{\n    let mut owner = own 7;\n    let r = ref owner;\n    if {condition} {{\n        {action}\n    }}\n    print(*r)\n}}"
            );
            compare(
                &format!("branch:reassigned={reassigned},taken={enter_branch}"),
                &source,
            );
        }
    }
}

#[test]
fn dangling_return_and_move_cannot_be_masked_by_elision() {
    const CASES: &[(&str, &str)] = &[
        (
            "return",
            "fn ref i64 stale() { let x = own 3; return ref x; }\nfn main() { let r = stale(); print(*r) }",
        ),
        (
            "move",
            "fn take(own i64 value) { drop(value); }\nfn main() { let x = own 3; let r = ref x; take(x); print(*r) }",
        ),
        (
            "drop",
            "fn main() { let x = own 3; let r = ref x; drop(x); print(*r) }",
        ),
    ];
    for (label, source) in CASES {
        compare(label, source);
    }
}
