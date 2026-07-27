// Const expressions are evaluated at compile time by the IR interpreter
// (`compile/ir/eval.rs`), while the same expression written without `const`
// runs through the VM (`runtime/vm/ops.rs`). The two evaluators must agree.

use hegel::generators;
use rune::Context;
use rune_hegel_tests::{eval_reference, eval_result, expr, render, ArithOp};

/// Property: a `const` expression evaluated by the compile-time IR interpreter
/// yields the same value as the identical expression evaluated by the VM at
/// runtime.
#[test]
fn const_eval_matches_runtime_eval() {
    let context = Context::with_default_modules().expect("failed to build context");

    hegel::Hegel::new(|tc| {
        let depth = tc.draw(generators::integers::<u32>().max_value(3));

        // `allow_neg = false`: unary expressions are not supported in const
        // contexts, so `-expr` would be a compile error rather than a
        // disagreement. `%` is likewise excluded: const evaluation reports 'op
        // not supported yet' for it.
        let ops = vec![ArithOp::Add, ArithOp::Sub, ArithOp::Mul, ArithOp::Div];
        let e = tc.draw(expr(depth, false, ops));

        // Overflowing const expressions panic the compiler in debug builds
        // (rune-rs/rune#1039); excluded here.
        tc.assume(eval_reference(&e).is_ok());

        let mut src = String::new();
        render(&e, &mut src);

        let runtime = eval_result::<i64>(&context, &src);
        let constant = eval_result::<i64>(&context, &format!("const VALUE = {src}; VALUE"));

        match (runtime, constant) {
            (Ok(runtime), Ok(constant)) => {
                assert_eq!(runtime, constant, "runtime and const evaluation disagree\nsource: {src}")
            }
            (runtime, constant) => panic!(
                "expected both evaluations to succeed\nruntime: {runtime:?}\nconst: {constant:?}\nsource: {src}"
            ),
        }
    })
    .run();
}
