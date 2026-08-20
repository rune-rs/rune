//! Tests that native functions which drive a loop themselves take budget
//! permits.
//!
//! The virtual machine only takes a permit for the instructions it executes
//! itself, so a native adapter which walks an iterator to completion is
//! invisible to a budget unless it takes permits of its own. A script can write
//! the same loop by hand, and that loop *is* budgeted, so the two have to be
//! interchangeable from the host's point of view.
//!
//! Every function is pinned separately. A single representative case would not
//! have caught `reduce`, `sum` and `product` being missed.

prelude!();

use crate::runtime::{budget, VmError, VmHaltInfo};

/// How many permits each case is given. Large enough for the setup the script
/// performs before it enters the native loop, far smaller than the number of
/// elements the loop would otherwise walk.
const BUDGET: usize = 1024;

/// Build `source` and run its `main` under a budget, returning the error.
fn limited(source: &str) -> VmError {
    let context = Context::with_default_modules().expect("Failed to build context");

    let mut sources = crate::tests::sources(source);
    let mut diagnostics = Diagnostics::new();

    let mut vm = crate::tests::vm(&context, &mut sources, &mut diagnostics, false)
        .expect("Source should compile");

    match budget::with(BUDGET, || vm.call(["main"], ())).call() {
        Ok(value) => panic!("Expected the budget to be exceeded, got {value:?}"),
        Err(error) => error,
    }
}

/// Assert that `source` is stopped by the budget rather than running forever.
#[track_caller]
fn assert_limited(source: &str) {
    let error = limited(source);

    assert!(
        matches!(
            error.error().kind(),
            VmErrorKind::Halted {
                halt: VmHaltInfo::Limited
            }
        ),
        "{source}: expected to be halted by the budget, got {error:?}"
    );
}

/// The driver loops which walk an iterator to its end.
#[test]
fn native_iterator_loops_are_budgeted() {
    let cases = [
        "pub fn main() { (0..).iter().count() }",
        "pub fn main() { (0..).iter().fold(0, |a, b| a) }",
        "pub fn main() { (0..).iter().reduce(|a, b| a) }",
        "pub fn main() { (0..).iter().find(|v| false) }",
        "pub fn main() { (0..).iter().any(|v| false) }",
        "pub fn main() { (0..).iter().all(|v| true) }",
        "pub fn main() { (0..).iter().nth(9223372036854775807) }",
    ];

    for source in cases {
        assert_limited(source);
    }
}

/// `sum` and `product` drive the iterator through the protocol directly rather
/// than through a resolved `next`, so they are a separate path.
///
/// Neither terminates on its own over an infinite range: `sum` would need some
/// four billion elements to overflow, and `product` never overflows at all
/// since the accumulator is zeroed by the range's first element.
#[test]
fn native_arithmetic_loops_are_budgeted() {
    let cases = [
        "pub fn main() { (0..).iter().sum::<i64>() }",
        "pub fn main() { (0..).iter().product::<i64>() }",
        "pub fn main() { (0..).iter().sum::<u64>() }",
        "pub fn main() { (0..).iter().product::<u64>() }",
        "pub fn main() { (0..).iter().map(|v| v as f64).sum::<f64>() }",
        "pub fn main() { (0..).iter().map(|v| v as f64).product::<f64>() }",
    ];

    for source in cases {
        assert_limited(source);
    }
}

/// The `collect` family allocates per element, so a memory limit catches it -
/// but only if the host set one. A budget on its own has to be enough.
#[test]
fn native_collect_loops_are_budgeted() {
    let cases = [
        "pub fn main() { (0..).iter().collect::<Vec>() }",
        "use std::collections::VecDeque; \
         pub fn main() { (0..).iter().collect::<VecDeque>() }",
        "use std::collections::HashSet; \
         pub fn main() { (0..).iter().collect::<HashSet>() }",
        "use std::collections::HashMap; \
         pub fn main() { (0..).iter().map(|v| (v, v)).collect::<HashMap>() }",
        "pub fn main() { (0..).iter().map(|v| (`${v}`, v)).collect::<Object>() }",
        "pub fn main() { (0..).iter().collect::<Tuple>() }",
        "pub fn main() { (0..).iter().map(|v| 'a').collect::<String>() }",
    ];

    for source in cases {
        assert_limited(source);
    }
}

/// Adapters whose `next` loops until it finds something never yield to the
/// machine at all when nothing ever matches.
#[test]
fn native_adapter_loops_are_budgeted() {
    let cases = [
        "pub fn main() { (0..).iter().filter(|v| false).next() }",
        "pub fn main() { (0..).iter().filter_map(|v| None).next() }",
        "pub fn main() { (0..).iter().flat_map(|v| []).next() }",
    ];

    for source in cases {
        assert_limited(source);
    }
}
