//! Sorting is driven by a comparison which the script supplies.
//!
//! The standard library's sort notices a comparison which does not implement a
//! total order and panics, which in a host process is not something a script
//! should be able to cause. Nothing obliges a script's comparison to be
//! consistent - it can be inconsistent by accident, on purpose, or because it
//! failed and the placeholder ordering substituted for the failed call does not
//! agree with the rest - so the sort is one which does not care.

prelude!();

/// How many elements the vectors below hold.
///
/// Enough that the merges actually run - the check which used to panic only
/// fires once runs are being combined, so a handful of elements never reached
/// it.
const LEN: usize = 1000;

/// A script which fills `v` with `LEN` elements produced by `each`.
fn filled(each: &str) -> rust_alloc::string::String {
    format!(
        "use std::cmp::Ordering; \
         let v = []; \
         let i = 0; \
         while i < {LEN} {{ v.push({each}); i += 1; }} "
    )
}

/// An inconsistent comparison leaves the elements in an order which is not
/// worth describing, and that is all it does.
#[test]
fn an_inconsistent_comparison_is_not_a_panic() {
    let source = format!(
        "{} let n = [0]; \
         v.sort_by(|a, b| {{ \
             n[0] = n[0] + 1; \
             match n[0] % 3 {{ \
                 0 => Ordering::Less, \
                 1 => Ordering::Greater, \
                 _ => Ordering::Equal, \
             }} \
         }}); \
         v.len()",
        filled("i % 17")
    );

    let value: i64 = eval(&source);
    assert_eq!(value, LEN as i64);
}

/// A comparison which always answers the same way is inconsistent in the other
/// direction - it never lets anything be equal or greater.
#[test]
fn a_constant_comparison_is_not_a_panic() {
    let source = format!("{} v.sort_by(|a, b| Ordering::Less); v.len()", filled("i"));

    let value: i64 = eval(&source);
    assert_eq!(value, LEN as i64);
}

/// A comparison which fails is reported rather than swallowed, and it does not
/// take the process with it on the way out.
#[test]
fn a_failing_comparison_is_reported() {
    let source = format!("{} v.sort_by(|a, b| panic!(\"no\")); v.len()", filled("i"));

    let context = Context::with_default_modules().expect("Failed to build context");

    let error = crate::tests::run::<i64>(&context, &source, (), true)
        .expect_err("The comparison should be reported");

    assert!(error.to_string().contains("no"), "{error}");
}

/// Values which are not comparable make `sort` inconsistent the same way, since
/// what it falls back on for the pair it could not compare does not agree with
/// what it did for the pairs it could.
#[test]
fn sorting_incomparable_values_is_reported() {
    let source = format!(
        "{} v.push(\"not a number\"); v.sort(); v.len()",
        filled("i")
    );

    let context = Context::with_default_modules().expect("Failed to build context");

    crate::tests::run::<i64>(&context, &source, (), true)
        .expect_err("The comparison should be reported");
}

/// Sorting still sorts, and it is stable while it does.
#[test]
fn sorting_is_stable() {
    // Everything compares equal on what is being sorted by, so a stable sort
    // leaves the elements exactly as they were.
    let source = format!("{} v.sort_by(|a, b| (a % 1).cmp(b % 1)); v", filled("i"));

    let value: Vec<i64> = eval(&source);
    assert_eq!(value, (0..LEN as i64).collect::<Vec<i64>>());
}

/// And it puts things in order when it is told the truth about them.
#[test]
fn sorting_orders() {
    let source = format!("{} v.sort(); v", filled(&format!("{} - i", LEN - 1)));

    let value: Vec<i64> = eval(&source);
    assert_eq!(value, (0..LEN as i64).collect::<Vec<i64>>());
}
