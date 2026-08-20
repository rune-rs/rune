//! Reaching a tuple field of a tuple field.
//!
//! Nothing about a number says what it is written next to, so `a.0.1` lexes its
//! two indices as the one number `0.1`. Two indices separated by a point is what
//! that can be where a field is expected and nothing else, so it is taken apart
//! again there. Written any other way - `a.0 .1`, `(a.0).1` - it already worked,
//! and what it said otherwise was "Unsupported tuple index `0`".

prelude!();

/// The indices are reached, however many of them there are.
#[test]
fn a_tuple_index_may_follow_a_tuple_index() {
    let value: i64 = eval("let t = ((1, 2), 3); t.0.0");
    assert_eq!(value, 1);

    let value: i64 = eval("let t = ((1, 2), 3); t.0.1");
    assert_eq!(value, 2);

    // Two numbers in a row is four indices.
    let value: i64 = eval("let t = ((((9, 0), 0), 0), 0); t.0.0.0.0");
    assert_eq!(value, 9);

    // The indices are not single digits.
    let value: i64 = eval("let t = (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, (0, 1, 2)); t.10.2");
    assert_eq!(value, 2);

    // The same however it is written.
    let value: i64 = eval("let t = ((1, 2), 3); t.0 .1");
    assert_eq!(value, 2);

    let value: i64 = eval("let t = ((1, 2), 3); (t.0).1");
    assert_eq!(value, 2);
}

/// It is a place, not just a value, so it can be written to.
#[test]
fn a_tuple_index_pair_can_be_assigned_to() {
    let value: i64 = eval("let t = ((1, 2), 3); t.0.1 = 7; t.0.1");
    assert_eq!(value, 7);

    let value: i64 = eval("let t = ((1, 2), 3); t.0.1 += 7; t.0.1");
    assert_eq!(value, 9);
}

/// A number which was written as a number rather than as two indices is still
/// not one which can index a tuple.
#[test]
fn a_number_which_is_not_two_indices_is_rejected() {
    for source in ["let t = (1, 2); t.0e1", "let t = (1, 2); t.0.1e2"] {
        let context = Context::with_default_modules().expect("Failed to build context");
        let mut sources = crate::tests::sources(source);
        let mut diagnostics = Diagnostics::new();

        let mut options = Options::default();
        options.script(true);

        let result = crate::prepare(&mut sources)
            .with_context(&context)
            .with_diagnostics(&mut diagnostics)
            .with_options(&options)
            .build();

        assert!(result.is_err(), "{source} should not compile");
    }

    // And a number written where a number belongs is still a number.
    let value: f64 = eval("let a = 0.5; a + 1.25");
    assert_eq!(value, 1.75);
}
