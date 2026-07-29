// A generated program is run twice: once by the machine and once by a reference
// which says what the program means. The two must agree on what the program
// evaluated to and on what it left in its variables.
//
// What this is for is the way values are moved between registers as control
// flows through a program. An expression whose body is a variable hands the
// address of that variable over rather than writing anything, so a construct
// which lets more than one of its parts write where it writes has to allocate
// an address of its own first - and where it did not, a later part wrote into
// whichever variable an earlier one named. That is invisible in the value the
// program produced, which is why the variables are compared as well.

use hegel::generators;
use rune::Context;
use rune_hegel_tests::{eval_result, program, render_source, run_reference, Outcome, Vars};

/// Property: a generated program evaluates to what the reference says it does,
/// and leaves the same behind in its variables.
#[test]
fn programs_evaluate_as_the_reference_does() {
    let context = Context::with_default_modules().expect("failed to build context");

    hegel::Hegel::new(|tc| {
        let depth = tc.draw(generators::integers::<u32>().min_value(1).max_value(3));
        let boolean = tc.draw(generators::booleans());
        let p = tc.draw(program(depth, boolean));

        // A program which overflows is an error rather than a value, and which
        // error the machine reports is not what is being compared here.
        let mut vars = Vars::default();
        let Ok(expected) = run_reference(&p, &mut vars) else {
            tc.assume(false);
            return;
        };

        let source = render_source(&p);

        let (ints, bools) = match expected {
            Outcome::Int(expected) => {
                let (actual, ints, bools) = evaluate::<i64>(&context, &source);
                assert_eq!(actual, expected, "source: {source}");
                (ints, bools)
            }
            Outcome::Bool(expected) => {
                let (actual, ints, bools) = evaluate::<bool>(&context, &source);
                assert_eq!(actual, expected, "source: {source}");
                (ints, bools)
            }
        };

        assert_eq!(ints, vars.ints(), "variables differ\nsource: {source}");
        assert_eq!(bools, vars.bools(), "variables differ\nsource: {source}");
    })
    .settings(hegel::Settings::new().test_cases(1000))
    .run();
}

fn evaluate<T>(context: &Context, source: &str) -> (T, Vec<i64>, Vec<bool>)
where
    T: rune::runtime::FromValue,
{
    match eval_result::<(T, Vec<i64>, Vec<bool>)>(context, source) {
        Ok(value) => value,
        Err(error) => panic!("failed to evaluate: {error:?}\nsource: {source}"),
    }
}
