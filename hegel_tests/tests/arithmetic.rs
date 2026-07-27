// The VM implements integer arithmetic with checked operations
// (`runtime/vm/ops.rs`): `+`/`*` raise `VmErrorKind::Overflow`, `-` raises
// `Underflow`, and `/`/`%` raise `DivideByZero` when the checked operation
// fails. These properties compare generated expressions against a Rust-side
// checked `i64` reference implementation.
//
// `VmErrorKind` is not part of rune's public API, so the specific error kind
// can't be matched from outside the crate; these tests assert that overflow
// surfaces as a VM error rather than a value or panic.

use hegel::generators;
use rune::Context;
use rune_hegel_tests::{
    eval_reference, eval_result, expr, literal_gen, render, EvalError, RefError, ALL_OPS,
};

#[test]
fn arithmetic_matches_checked_i64_reference() {
    let context = Context::with_default_modules().expect("failed to build context");

    hegel::Hegel::new(|tc| {
        let depth = tc.draw(generators::integers::<u32>().max_value(4));
        let e = tc.draw(expr(depth, true, ALL_OPS.to_vec()));

        let mut src = String::new();
        render(&e, &mut src);

        let reference = match eval_reference(&e) {
            Err(RefError::NegOverflow) => {
                // Negating `i64::MIN` is unchecked in `Vm::op_neg`: panics in
                // debug, wraps in release (rune-rs/rune#1030). Excluded here so
                // the rest of the arithmetic surface stays covered.
                tc.reject()
            }
            other => other,
        };

        let result = eval_result::<i64>(&context, &src);

        match (reference, result) {
            (Ok(expected), Ok(actual)) => assert_eq!(actual, expected, "source: {src}"),
            (Ok(expected), Err(error)) => {
                panic!("expected {expected}, got error: {error:?}\nsource: {src}")
            }
            (Err(RefError::Arith(_)), Err(EvalError::Vm)) => {}
            (Err(reference), result) => {
                panic!("expected VM error for {reference:?}, got {result:?}\nsource: {src}")
            }
        }
    })
    .run();
}

#[test]
fn shift_ops_match_checked_reference() {
    let context = Context::with_default_modules().expect("failed to build context");

    hegel::Hegel::new(|tc| {
        let value = tc.draw(literal_gen());
        let shift: i64 = tc.draw(hegel::one_of!(
            generators::integers::<i64>().min_value(0).max_value(70),
            generators::integers::<i64>(),
            generators::sampled_from(vec![-1, 63, 64, (u32::MAX as i64) + 1]),
        ));
        let shl = tc.draw(generators::booleans());

        let symbol = if shl { "<<" } else { ">>" };
        let src = format!("({value}) {symbol} ({shift})");

        let expected = u32::try_from(shift).ok().and_then(|shift| {
            if shl {
                value.checked_shl(shift)
            } else {
                value.checked_shr(shift)
            }
        });

        let result = eval_result::<i64>(&context, &src);

        match (expected, result) {
            (Some(expected), Ok(actual)) => assert_eq!(actual, expected, "source: {src}"),
            (Some(expected), Err(error)) => {
                panic!("expected {expected}, got error: {error:?}\nsource: {src}")
            }
            (None, Err(EvalError::Vm)) => {}
            (None, result) => panic!("expected VM error, got {result:?}\nsource: {src}"),
        }
    })
    .run();
}
