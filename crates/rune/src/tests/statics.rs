//! Tests for `static` items and the [`Globals`] storage that backs them.

prelude!();

use crate::runtime::Globals;
use crate::support::Error;
use crate::{termcolor, to_value, Statics, Unit};

/// The options these tests build under.
///
/// What a static is called is debug information, and the diagnostics below name
/// the static they are about, so this pins it on rather than taking whatever
/// `RUNEFLAGS` happens to say. Building with `debug-info=false` is a supported
/// thing to do; what these tests are about is what is said when it is not.
fn options() -> Options {
    let mut options = Options::from_default_env().expect("parsing RUNEFLAGS");
    options.debug_info(true);
    options
}

fn build(source: &str) -> Result<(Arc<Unit>, Arc<runtime::RuntimeContext>)> {
    let context = Context::with_default_modules()?;
    let runtime = Arc::try_new(context.runtime()?)?;

    let mut sources = Sources::new();
    sources.insert(Source::memory(source)?)?;

    let mut diagnostics = Diagnostics::new();

    let result = prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .with_options(&options())
        .build();

    if !diagnostics.is_empty() {
        let mut writer = termcolor::Buffer::no_color();
        diagnostics.emit(&mut writer, &sources)?;
        let out = String::from_utf8(writer.into_inner())?;
        assert!(result.is_ok(), "diagnostics:\n{out}");
    }

    Ok((Arc::try_new(result?)?, runtime))
}

/// Build the given source with the given statics declared.
///
/// Unlike [`build`] this doesn't assert that building succeeds, the emitted
/// diagnostics are handed back as an error instead.
fn build_declared(
    source: &str,
    statics: &Statics,
) -> Result<(Arc<Unit>, Arc<runtime::RuntimeContext>)> {
    let context = Context::with_default_modules()?;
    let runtime = Arc::try_new(context.runtime()?)?;

    let mut sources = Sources::new();
    sources.insert(Source::memory(source)?)?;

    let mut diagnostics = Diagnostics::new();

    let result = prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .with_options(&options())
        .with_statics(statics)
        .build();

    let mut writer = termcolor::Buffer::no_color();
    diagnostics.emit(&mut writer, &sources)?;
    let out = String::from_utf8(writer.into_inner())?;

    let Ok(unit) = result else {
        return Err(Error::msg(out));
    };

    Ok((Arc::try_new(unit)?, runtime))
}

/// Build the given source and run `f` against it.
fn with(source: &str, f: impl FnOnce(Arc<Unit>, Arc<runtime::RuntimeContext>) -> Result<()>) {
    let (unit, runtime) = build(source).expect("failed to build source");
    f(unit, runtime).expect("test body failed");
}

/// Build the given source with the given statics declared and run `f` against
/// it.
fn with_declared(
    source: &str,
    statics: &Statics,
    f: impl FnOnce(Arc<Unit>, Arc<runtime::RuntimeContext>) -> Result<()>,
) {
    let (unit, runtime) = build_declared(source, statics).expect("failed to build source");
    f(unit, runtime).expect("test body failed");
}

#[test]
#[cfg(not(miri))]
fn lazy_initialization() {
    with(
        r#"
        static N = 41;
        pub fn main() { N + 1 }
        "#,
        |unit, runtime| {
            let globals = Globals::new(unit.clone())?;

            // Nothing has read the static yet, so the initializer has not run.
            assert!(globals.get(["N"])?.is_none());

            let mut vm = Vm::new(runtime, unit).with_globals(globals.clone());
            let output: i64 = from_value(vm.call(["main"], ())?)?;
            assert_eq!(output, 42);

            let n: i64 = from_value(globals.get(["N"])?.context("N to be initialized")?)?;
            assert_eq!(n, 41);
            Ok(())
        },
    );
}

#[test]
#[cfg(not(miri))]
fn assignment_from_script() {
    with(
        r#"
        static N = 0;
        pub fn main() { N = 42; N }
        "#,
        |unit, runtime| {
            let globals = Globals::new(unit.clone())?;
            let mut vm = Vm::new(runtime, unit).with_globals(globals.clone());

            let output: i64 = from_value(vm.call(["main"], ())?)?;
            assert_eq!(output, 42);

            let n: i64 = from_value(globals.get(["N"])?.context("N to be initialized")?)?;
            assert_eq!(n, 42);
            Ok(())
        },
    );
}

#[test]
#[cfg(not(miri))]
fn persists_across_calls() {
    with(
        r#"
        static N = 0;
        pub fn main() { N += 1; N }
        "#,
        |unit, runtime| {
            let globals = Globals::new(unit.clone())?;
            let mut vm = Vm::new(runtime, unit).with_globals(globals);

            assert_eq!(from_value::<i64>(vm.call(["main"], ())?)?, 1);
            assert_eq!(from_value::<i64>(vm.call(["main"], ())?)?, 2);
            assert_eq!(from_value::<i64>(vm.call(["main"], ())?)?, 3);
            Ok(())
        },
    );
}

#[test]
#[cfg(not(miri))]
fn storages_are_isolated() {
    with(
        r#"
        static N = 0;
        pub fn main() { N += 1; N }
        "#,
        |unit, runtime| {
            let a = Globals::new(unit.clone())?;
            let b = Globals::new(unit.clone())?;

            let mut first = Vm::new(runtime.clone(), unit.clone()).with_globals(a);
            let mut second = Vm::new(runtime, unit).with_globals(b);

            assert_eq!(from_value::<i64>(first.call(["main"], ())?)?, 1);
            assert_eq!(from_value::<i64>(first.call(["main"], ())?)?, 2);
            // The second storage starts over from the initializer.
            assert_eq!(from_value::<i64>(second.call(["main"], ())?)?, 1);
            Ok(())
        },
    );
}

#[test]
#[cfg(not(miri))]
fn caller_write_wins_over_initializer() {
    with(
        r#"
        static N = 41;
        pub fn main() { N }
        "#,
        |unit, runtime| {
            let globals = Globals::new(unit.clone())?;
            globals.set(["N"], to_value(7i64)?)?;

            let mut vm = Vm::new(runtime, unit).with_globals(globals);
            assert_eq!(from_value::<i64>(vm.call(["main"], ())?)?, 7);
            Ok(())
        },
    );
}

#[test]
#[cfg(not(miri))]
fn cleared_static_is_reinitialized() {
    with(
        r#"
        static N = 41;
        pub fn main() { N }
        "#,
        |unit, runtime| {
            let globals = Globals::new(unit.clone())?;
            globals.set(["N"], to_value(7i64)?)?;

            let mut vm = Vm::new(runtime, unit).with_globals(globals.clone());
            assert_eq!(from_value::<i64>(vm.call(["main"], ())?)?, 7);

            globals.clear(["N"])?;
            assert!(!globals.is_initialized(0));
            assert_eq!(from_value::<i64>(vm.call(["main"], ())?)?, 41);
            Ok(())
        },
    );
}

#[test]
#[cfg(not(miri))]
fn static_without_initializer() {
    with(
        r#"
        static N;
        pub fn main() { N }
        "#,
        |unit, runtime| {
            let globals = Globals::new(unit.clone())?;
            let mut vm = Vm::new(runtime, unit).with_globals(globals.clone());

            // Nothing has assigned it, so reading it is an error which names
            // the static.
            let error = vm.call(["main"], ()).unwrap_err();
            let message = error.to_string();
            assert!(
                message.contains("uninitialized static") && message.contains('N'),
                "unexpected error: {message}"
            );

            globals.set(["N"], to_value(11i64)?)?;
            assert_eq!(from_value::<i64>(vm.call(["main"], ())?)?, 11);
            Ok(())
        },
    );
}

#[test]
#[cfg(not(miri))]
fn missing_storage_is_an_error() {
    with(
        r#"
        static N = 1;
        pub fn main() { N }
        "#,
        |unit, runtime| {
            // No `with_globals`, so the vm has nowhere to put the static.
            let mut vm = Vm::new(runtime, unit);
            let message = vm.call(["main"], ()).unwrap_err().to_string();
            assert!(
                message.contains("No storage has been configured"),
                "unexpected error: {message}"
            );
            Ok(())
        },
    );
}

#[test]
#[cfg(not(miri))]
fn unknown_static_by_item() {
    with(
        r#"
        static N = 1;
        pub fn main() { N }
        "#,
        |unit, _| {
            let globals = Globals::new(unit)?;
            assert!(globals.get(["MISSING"]).is_err());
            assert!(globals.set(["MISSING"], to_value(1i64)?).is_err());
            Ok(())
        },
    );
}

#[test]
#[cfg(not(miri))]
fn unused_static_still_gets_a_slot() {
    with(
        r#"
        static UNUSED = 1;
        pub fn main() { 0 }
        "#,
        |unit, _| {
            let globals = Globals::new(unit.clone())?;
            assert_eq!(unit.globals_len(), 1);
            // Addressable by the caller even though no script reads it.
            globals.set(["UNUSED"], to_value(5i64)?)?;
            assert_eq!(
                from_value::<i64>(globals.get(["UNUSED"])?.context("UNUSED")?)?,
                5
            );
            Ok(())
        },
    );
}

#[test]
#[cfg(not(miri))]
fn statics_in_modules() {
    with(
        r#"
        pub mod inner {
            pub static N = 3;
        }

        pub fn main() { inner::N += 1; inner::N }
        "#,
        |unit, runtime| {
            let globals = Globals::new(unit.clone())?;
            let mut vm = Vm::new(runtime, unit).with_globals(globals.clone());

            assert_eq!(from_value::<i64>(vm.call(["main"], ())?)?, 4);

            let n: i64 = from_value(globals.get(["inner", "N"])?.context("inner::N")?)?;
            assert_eq!(n, 4);
            Ok(())
        },
    );
}

#[test]
#[cfg(not(miri))]
fn shared_through_function_value() {
    with(
        r#"
        static N = 0;

        fn bump() { N += 1; N }

        pub fn main() { bump }
        "#,
        |unit, runtime| {
            let globals = Globals::new(unit.clone())?;
            let mut vm = Vm::new(runtime, unit).with_globals(globals.clone());

            // Pull the function out and call it from the outside. It has to see
            // the same storage.
            let f: Function = from_value(vm.call(["main"], ())?)?;
            assert_eq!(f.call::<i64>(())?, 1);
            assert_eq!(f.call::<i64>(())?, 2);

            let n: i64 = from_value(globals.get(["N"])?.context("N")?)?;
            assert_eq!(n, 2);
            Ok(())
        },
    );
}

#[test]
#[cfg(not(miri))]
fn shared_through_generator() {
    with(
        r#"
        static N = 0;

        pub fn main() {
            let g = || {
                N += 1;
                yield N;
                N += 1;
                yield N;
            };

            let out = [];

            for value in g() {
                out.push(value);
            }

            out
        }
        "#,
        |unit, runtime| {
            let globals = Globals::new(unit.clone())?;
            let mut vm = Vm::new(runtime, unit).with_globals(globals.clone());

            let out: Vec<i64> = from_value(vm.call(["main"], ())?)?;
            assert_eq!(out, [1, 2]);

            let n: i64 = from_value(globals.get(["N"])?.context("N")?)?;
            assert_eq!(n, 2);
            Ok(())
        },
    );
}

#[test]
#[cfg(not(miri))]
fn shared_through_async() {
    with(
        r#"
        static N = 0;

        async fn bump() { N += 1; N }

        pub async fn main() { bump().await + bump().await }
        "#,
        |unit, runtime| {
            let globals = Globals::new(unit.clone())?;
            let mut vm = Vm::new(runtime, unit).with_globals(globals.clone());

            let output: i64 = from_value(block_on(vm.async_call(["main"], ()))?)?;
            assert_eq!(output, 3);

            let n: i64 = from_value(globals.get(["N"])?.context("N")?)?;
            assert_eq!(n, 2);
            Ok(())
        },
    );
}

#[test]
#[cfg(not(miri))]
fn non_trivial_initializer() {
    with(
        r#"
        const BASE = 20;
        static N = BASE * 2 + 2;
        pub fn main() { N }
        "#,
        |unit, runtime| {
            let globals = Globals::new(unit.clone())?;
            let mut vm = Vm::new(runtime, unit).with_globals(globals);
            assert_eq!(from_value::<i64>(vm.call(["main"], ())?)?, 42);
            Ok(())
        },
    );
}

#[test]
#[cfg(not(miri))]
fn non_inline_initializer() {
    with(
        r#"
        static NAME = "hello";
        static ITEMS = [1, 2, 3];
        pub fn main() { `${NAME}-${ITEMS.len()}` }
        "#,
        |unit, runtime| {
            let globals = Globals::new(unit.clone())?;
            let mut vm = Vm::new(runtime, unit).with_globals(globals);
            let output: String = from_value(vm.call(["main"], ())?)?;
            assert_eq!(output, "hello-3");
            Ok(())
        },
    );
}

#[test]
#[cfg(not(miri))]
fn debug_names_are_recorded() {
    with(
        r#"
        static N = 1;
        pub fn main() { N }
        "#,
        |unit, _| {
            let debug = unit.debug_info().context("debug info")?;
            let slot = unit.global_slot(&hash!(N)).context("slot for N")?;
            let global = debug.global(slot).context("debug info for N")?;
            assert_eq!(global.path.to_string(), "N");
            Ok(())
        },
    );
}

#[test]
#[cfg(not(miri))]
fn build_vm_provisions_storage() -> Result<()> {
    let context = Context::with_default_modules()?;

    let mut sources = sources! {
        entry => {
            static N = 0;
            pub fn main() { N += 1; N }
        }
    };

    let mut vm = prepare(&mut sources).with_context(&context).build_vm()?;

    assert_eq!(vm.globals().len(), 1);
    assert_eq!(from_value::<i64>(vm.call(["main"], ())?)?, 1);
    assert_eq!(from_value::<i64>(vm.call(["main"], ())?)?, 2);
    Ok(())
}

#[test]
#[cfg(not(miri))]
fn static_in_pattern_is_an_error() {
    assert_errors! {
        r#"static N = 1; pub fn main(v) { match v { N => 1, _ => 0 } }"#,
        span!(41, 42), ErrorKind::StaticInPattern { item } => {
            assert_eq!(item.to_string(), "N");
        }
    };
}

#[test]
#[cfg(not(miri))]
fn static_in_const_is_an_error() {
    assert_errors! {
        r#"static N = 1; const C = N; pub fn main() { C }"#,
        span!(24, 25), ErrorKind::StaticInConstContext { item } => {
            assert_eq!(item.to_string(), "N");
        }
    };
}

#[test]
#[cfg(not(miri))]
fn static_in_static_initializer_is_an_error() {
    assert_errors! {
        r#"static A = 1; static B = A; pub fn main() { B }"#,
        span!(25, 26), ErrorKind::StaticInConstContext { item } => {
            assert_eq!(item.to_string(), "A");
        }
    };
}

#[test]
#[cfg(not(miri))]
fn non_const_initializer_is_an_error() {
    assert_errors! {
        r#"fn f() { 1 } static N = f(); pub fn main() { N }"#,
        span!(24, 27), _
    };
}

#[test]
#[cfg(not(miri))]
fn declared_static_is_used_by_script() -> Result<()> {
    let mut statics = Statics::new();
    statics.insert(["N"])?;

    with_declared(
        r#"pub fn main() { N += 1; N }"#,
        &statics,
        |unit, runtime| {
            let globals = Globals::new(unit.clone())?;
            globals.set(["N"], to_value(41i64)?)?;

            let mut vm = Vm::new(runtime, unit).with_globals(globals.clone());
            assert_eq!(from_value::<i64>(vm.call(["main"], ())?)?, 42);

            let n: i64 = from_value(globals.get(["N"])?.context("N to be initialized")?)?;
            assert_eq!(n, 42);
            Ok(())
        },
    );

    Ok(())
}

#[test]
#[cfg(not(miri))]
fn declared_static_starts_uninitialized() -> Result<()> {
    let mut statics = Statics::new();
    statics.insert(["N"])?;

    with_declared(r#"pub fn main() { N }"#, &statics, |unit, runtime| {
        let globals = Globals::new(unit.clone())?;
        assert!(globals.get(["N"])?.is_none());

        // There is no initializer to fall back on, since the declaration does
        // not come from a source.
        let mut vm = Vm::new(runtime, unit).with_globals(globals.clone());
        let message = vm.call(["main"], ()).unwrap_err().to_string();
        assert!(
            message.contains("uninitialized static") && message.contains('N'),
            "unexpected error: {message}"
        );

        globals.set(["N"], to_value(11i64)?)?;
        assert_eq!(from_value::<i64>(vm.call(["main"], ())?)?, 11);
        Ok(())
    });

    Ok(())
}

#[test]
#[cfg(not(miri))]
fn declared_static_in_module() -> Result<()> {
    let mut statics = Statics::new();
    statics.insert(["config", "LIMIT"])?;

    with_declared(
        r#"
        pub mod inner {
            pub fn get() { crate::config::LIMIT }
        }

        pub fn main() { config::LIMIT + inner::get() }
        "#,
        &statics,
        |unit, runtime| {
            let globals = Globals::new(unit.clone())?;
            globals.set(["config", "LIMIT"], to_value(7i64)?)?;

            let mut vm = Vm::new(runtime, unit).with_globals(globals);
            assert_eq!(from_value::<i64>(vm.call(["main"], ())?)?, 14);
            Ok(())
        },
    );

    Ok(())
}

#[test]
#[cfg(not(miri))]
fn declared_static_unused_by_script() -> Result<()> {
    let mut statics = Statics::new();
    statics.insert(["UNUSED"])?;

    with_declared(r#"pub fn main() { 0 }"#, &statics, |unit, _| {
        let globals = Globals::new(unit.clone())?;
        assert_eq!(unit.globals_len(), 1);

        // Addressable by the caller even though no script reads it.
        globals.set(["UNUSED"], to_value(5i64)?)?;
        assert_eq!(
            from_value::<i64>(globals.get(["UNUSED"])?.context("UNUSED")?)?,
            5
        );
        Ok(())
    });

    Ok(())
}

#[test]
#[cfg(not(miri))]
fn declared_static_without_sources() -> Result<()> {
    let mut statics = Statics::new();
    statics.insert(["N"])?;

    let mut sources = Sources::new();

    let unit = prepare(&mut sources).with_statics(&statics).build()?;

    let unit = Arc::try_new(unit)?;
    assert_eq!(unit.globals_len(), 1);

    let globals = Globals::new(unit)?;
    globals.set(["N"], to_value(3i64)?)?;
    assert_eq!(from_value::<i64>(globals.get(["N"])?.context("N")?)?, 3);
    Ok(())
}

#[test]
#[cfg(not(miri))]
fn declared_static_records_debug_name() -> Result<()> {
    let mut statics = Statics::new();
    statics.insert(["N"])?;

    with_declared(r#"pub fn main() { N }"#, &statics, |unit, _| {
        let debug = unit.debug_info().context("debug info")?;
        let slot = unit.global_slot(&hash!(N)).context("slot for N")?;
        let global = debug.global(slot).context("debug info for N")?;
        assert_eq!(global.path.to_string(), "N");
        Ok(())
    });

    Ok(())
}

#[test]
#[cfg(not(miri))]
fn declared_static_alongside_source_statics() -> Result<()> {
    let mut statics = Statics::new();
    statics.insert(["DECLARED"])?;

    with_declared(
        r#"
        static SOURCE = 1;
        pub fn main() { SOURCE + DECLARED }
        "#,
        &statics,
        |unit, runtime| {
            let globals = Globals::new(unit.clone())?;
            globals.set(["DECLARED"], to_value(41i64)?)?;

            let mut vm = Vm::new(runtime, unit).with_globals(globals);
            assert_eq!(from_value::<i64>(vm.call(["main"], ())?)?, 42);
            Ok(())
        },
    );

    Ok(())
}

#[test]
#[cfg(not(miri))]
fn declared_static_in_const_is_an_error() -> Result<()> {
    let mut statics = Statics::new();
    statics.insert(["N"])?;

    let message = build_declared(r#"const C = N; pub fn main() { C }"#, &statics)
        .unwrap_err()
        .to_string();

    assert!(
        message.contains("cannot be used in a constant context"),
        "unexpected diagnostics:\n{message}"
    );

    Ok(())
}

#[test]
#[cfg(not(miri))]
fn declaring_a_static_the_source_declares_is_an_error() -> Result<()> {
    let mut statics = Statics::new();
    statics.insert(["N"])?;

    for source in [
        r#"static N = 1; pub fn main() { N }"#,
        r#"const N = 1; pub fn main() { N }"#,
        r#"fn N() { 1 } pub fn main() { N() }"#,
    ] {
        let message = build_declared(source, &statics).unwrap_err().to_string();

        assert!(
            message.contains("conflicts with a static of the same name"),
            "unexpected diagnostics for `{source}`:\n{message}"
        );
    }

    Ok(())
}

#[test]
#[cfg(not(miri))]
fn declaring_a_static_with_a_bad_name_is_an_error() -> Result<()> {
    let mut statics = Statics::new();
    statics.insert(["not an ident"])?;

    let message = build_declared(r#"pub fn main() { 0 }"#, &statics)
        .unwrap_err()
        .to_string();

    assert!(
        message.contains("is not a valid item"),
        "unexpected diagnostics:\n{message}"
    );

    let mut statics = Statics::new();
    statics.insert([] as [&str; 0])?;

    let message = build_declared(r#"pub fn main() { 0 }"#, &statics)
        .unwrap_err()
        .to_string();

    assert!(
        message.contains("must be declared with a name"),
        "unexpected diagnostics:\n{message}"
    );

    Ok(())
}
