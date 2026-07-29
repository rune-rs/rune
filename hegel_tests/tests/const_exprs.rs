// A `const` expression is compiled into a unit of its own and run by a virtual
// machine at compile time (`compile/const_eval.rs`), while the same expression
// written without `const` is compiled into the unit being built and run by a
// machine at runtime. The two must agree, whether the expression produces a
// value or an error.

use hegel::generators;
use rune::Context;
use rune_hegel_tests::{eval_reference, eval_result, expr, render, ALL_OPS};

/// Property: a `const` expression evaluated while compiling yields the same
/// value as the identical expression evaluated at runtime, and fails where the
/// other fails.
#[test]
fn const_eval_matches_runtime_eval() {
    let context = Context::with_default_modules().expect("failed to build context");

    hegel::Hegel::new(|tc| {
        let depth = tc.draw(generators::integers::<u32>().max_value(3));
        let e = tc.draw(expr(depth, true, ALL_OPS.to_vec()));

        let mut src = String::new();
        render(&e, &mut src);

        let runtime = eval_result::<i64>(&context, &src);
        let constant = eval_result::<i64>(&context, &format!("const VALUE = {src}; VALUE"));

        match (eval_reference(&e), runtime, constant) {
            (Ok(expected), Ok(runtime), Ok(constant)) => {
                assert_eq!(runtime, expected, "runtime disagrees with the reference\nsource: {src}");
                assert_eq!(constant, expected, "const disagrees with the reference\nsource: {src}");
            }
            // Overflow and division by zero are reported rather than being
            // carried out, by both of them. Which error each reports is not
            // what is being compared here.
            (Err(..), Err(..), Err(..)) => {}
            (expected, runtime, constant) => panic!(
                "runtime and const evaluation disagree\n\
                 reference: {expected:?}\nruntime: {runtime:?}\nconst: {constant:?}\nsource: {src}"
            ),
        }
    })
    .run();
}
