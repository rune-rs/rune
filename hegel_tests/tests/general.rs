use hegel::generators;
use rune::runtime::budget;
use rune::sync::Arc;
use rune::{Context, Diagnostics, Hash, Options, Source, Sources, Vm};
use rune_hegel_tests::eval_result;

/// Property: formatting any `i64` through a template string agrees with Rust's
/// integer formatting (the runtime formats integers through `itoa`).
#[test]
fn template_int_formatting_matches_rust() {
    let context = Context::with_default_modules().expect("failed to build context");

    hegel::Hegel::new(|tc| {
        let n: i64 = tc.draw(hegel::one_of!(
            generators::integers::<i64>(),
            generators::sampled_from(vec![i64::MIN, -1, 0, 1, i64::MAX]),
        ));

        let src = format!("`${{{n}}}`");

        match eval_result::<String>(&context, &src) {
            Ok(out) => assert_eq!(out, n.to_string(), "source: {src}"),
            Err(error) => panic!("failed to evaluate template {src}: {error:?}"),
        }
    })
    .run();
}

/// Property: a VM running a script that never terminates on its own is always
/// halted by an instruction budget — `budget::with` returns an error rather
/// than hanging or panicking, for any budget and any of the loop shapes below.
#[test]
fn budget_halts_infinite_loops() {
    let context = Context::with_default_modules().expect("failed to build context");
    let runtime = Arc::try_new(context.runtime().expect("runtime")).expect("runtime arc");

    hegel::Hegel::new(|tc| {
        let budget_size = tc.draw(hegel::one_of!(
            generators::integers::<usize>().min_value(1).max_value(100),
            generators::integers::<usize>().min_value(1).max_value(100_000),
        ));

        // All templates loop forever without erroring: the modulus keeps the
        // counter small so checked arithmetic can never overflow.
        let modulus = tc.draw(generators::integers::<i64>().min_value(1).max_value(1_000));
        let program = tc.draw(generators::sampled_from(vec![
            format!("let i = 0; while true {{ i = (i + 1) % {modulus}; }} i"),
            "loop { }".to_string(),
            format!("fn f(m) {{ let i = 0; while true {{ i = (i + 1) % m; }} }} f({modulus})"),
            "while true { while true { } }".to_string(),
        ]));

        let mut sources = Sources::new();
        sources.insert(Source::memory(&program).expect("source")).expect("insert");

        let mut diagnostics = Diagnostics::default();
        let mut options = Options::default();
        options.script(true);

        let unit = rune::prepare(&mut sources)
            .with_context(&context)
            .with_diagnostics(&mut diagnostics)
            .with_options(&options)
            .build()
            .expect("infinite loop programs must compile");

        let unit = Arc::try_new(unit).expect("unit arc");
        let mut vm = Vm::new(runtime.clone(), unit);

        let result = budget::with(budget_size, || vm.call(Hash::EMPTY, ())).call();

        assert!(
            result.is_err(),
            "an infinite loop terminated successfully under budget {budget_size}: {result:?}\nprogram: {program}"
        );
    })
    .run();
}
