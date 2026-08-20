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

use crate::alloc::limit;
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
        // `skip` walks and discards up front rather than lazily, so the walk is
        // a driver loop of its own over a count the script chose.
        "pub fn main() { (0..).iter().skip(9223372036854775807).next() }",
    ];

    for source in cases {
        assert_limited(source);
    }
}

/// Stepping an iterator natively is budgeted wherever it is done from, not only
/// in this module, so a container which is extended from an endless iterator is
/// stopped as well.
#[test]
fn native_extend_loops_are_budgeted() {
    let cases = [
        "pub fn main() { let v = []; v.extend(0..); v }",
        "use std::collections::VecDeque; \
         pub fn main() { let v = VecDeque::new(); v.extend(0..); v }",
        "use std::collections::HashSet; \
         pub fn main() { let v = HashSet::new(); v.extend(0..); v }",
        "use std::collections::HashMap; \
         pub fn main() { let v = HashMap::new(); v.extend((0..).iter().map(|v| (v, v))); v }",
    ];

    for source in cases {
        assert_limited(source);
    }
}

/// `collect` asks the iterator for a size hint and reserves space for it. The
/// hint is written by whoever implemented the iterator, and this module's own
/// documentation says code must not rely on it being correct, so honouring it
/// exactly turns a hint into an allocation request of any size a script likes.
///
/// An endless range hints at more elements than could ever be held, so before
/// this was clamped the reservation failed outright - and a hint just under
/// what fits would have succeeded, which is worse.
#[test]
fn collect_does_not_reserve_from_the_size_hint() {
    let source = "pub fn main() { (0..).iter().collect::<Vec>() }";

    let context = Context::with_default_modules().expect("Failed to build context");

    let mut sources = crate::tests::sources(source);
    let mut diagnostics = Diagnostics::new();

    let mut vm = crate::tests::vm(&context, &mut sources, &mut diagnostics, false)
        .expect("Source should compile");

    // Room for what is reserved up front and then some, but nowhere near what
    // the hint asks for.
    let result = budget::with(BUDGET, limit::with(1 << 20, || vm.call(["main"], ()))).call();

    let error = result.expect_err("Expected the budget to be exceeded");

    assert!(
        matches!(
            error.error().kind(),
            VmErrorKind::Halted {
                halt: VmHaltInfo::Limited
            }
        ),
        "expected to be halted by the budget, got {error:?}"
    );
}

/// What a halt says it was.
///
/// All three read as "Halted for unexpected reason `limited`", which describes
/// the ordinary end of a budget - the thing the host asked for - as something
/// unexpected, and says nothing at all about what to do differently for the
/// other two.
#[test]
fn a_halt_says_what_happened() {
    // The budget running out, which is what a host who set one is waiting for.
    let error = limited("pub fn main() { let n = 0; while true { n += 1; } n }");
    let message = rust_alloc::format!("{error}");

    assert!(message.contains("budget"), "{message}");
    assert!(!message.contains("unexpected"), "{message}");

    // A generator called as if it were a plain function, which is what an entry
    // point containing `yield` is - the calling convention is ignored for one.
    let context = Context::with_default_modules().expect("Failed to build context");

    let mut sources = crate::tests::sources("pub fn main() { yield 1; }");
    let mut diagnostics = Diagnostics::new();

    let mut vm = crate::tests::vm(&context, &mut sources, &mut diagnostics, false)
        .expect("Source should compile");

    let error = vm
        .call(["main"], ())
        .expect_err("A generator cannot be called for a value");

    let message = rust_alloc::format!("{error}");

    assert!(message.contains("generator"), "{message}");
    assert!(!message.contains("unexpected"), "{message}");

    // An asynchronous entry point is driven rather than halted, which is what
    // "the calling convention is ignored" is there for - so the message above
    // is about a generator specifically.
    let mut sources = crate::tests::sources("pub async fn main() { 1 }");
    let mut diagnostics = Diagnostics::new();

    let mut vm = crate::tests::vm(&context, &mut sources, &mut diagnostics, false)
        .expect("Source should compile");

    let value = vm
        .call(["main"], ())
        .expect("An asynchronous entry point is driven to its value");

    let value: i64 = crate::from_value(value).expect("The value is a number");
    assert_eq!(value, 1);
}
