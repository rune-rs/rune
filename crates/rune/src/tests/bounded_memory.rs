//! Tests that a memory limit is what a script runs into rather than the
//! machine it is running on.
//!
//! Everything the language allocates goes through `rune-alloc`, which is what
//! makes the limit possible. Anything which reaches for the standard library's
//! own collections or for the global allocator escapes it, and what that looks
//! like is not an error the host can catch but the process being killed - so
//! each shape which allocates a lot is run under a limit and has to come back
//! with an error.

prelude!();

use crate::alloc::limit;
use crate::runtime::budget;

/// How much each case is allowed to allocate.
///
/// Enough for the machine, the unit and the values a small script produces,
/// nowhere near enough for what these ask for.
const LIMIT: usize = 1 << 20;

/// How many instructions each case is allowed to run, so that a loop which
/// allocates nothing per iteration still stops.
const BUDGET: usize = 1 << 18;

/// Run `source` under both limits, returning what it did.
fn constrained(source: &str) -> Result<(), rust_alloc::string::String> {
    let context = Context::with_default_modules().expect("Failed to build context");

    let mut sources = crate::tests::sources(source);
    let mut diagnostics = Diagnostics::new();

    let mut vm = crate::tests::vm(&context, &mut sources, &mut diagnostics, true)
        .expect("Source should compile");

    let result = budget::with(BUDGET, limit::with(LIMIT, || vm.call(Hash::EMPTY, ()))).call();

    match result {
        Ok(..) => Ok(()),
        Err(error) => Err(rust_alloc::format!("{error}")),
    }
}

/// Every shape which asks for more than it is allowed to have.
///
/// A case which allocates outside the limit does not fail this test, it kills
/// the process running it - which is the point.
#[test]
fn allocating_past_the_limit_is_an_error() {
    let cases = [
        // Growing a container one element at a time.
        "let v = []; while true { v.push(0); } v",
        "let v = []; while true { v.push([0, 0, 0, 0]); } v",
        "let s = \"\"; while true { s += \"xxxxxxxxxxxxxxxx\"; } s",
        "use std::collections::HashMap; \
         let m = HashMap::new(); let i = 0; while true { m.insert(i, i); i += 1; } m",
        "use std::collections::HashSet; \
         let s = HashSet::new(); let i = 0; while true { s.insert(i); i += 1; } s",
        "use std::collections::VecDeque; \
         let d = VecDeque::new(); while true { d.push_back(0); } d",
        "let o = #{}; let i = 0; while true { o[`k${i}`] = i; i += 1; } o",
        // Asking for it all at once.
        "let v = Vec::with_capacity(1000000000000); v",
        "let s = String::with_capacity(1000000000000); s",
        "let v = []; v.resize(1000000000000, 0); v",
        // Walking something endless into a container.
        "let v = (0..).iter().collect::<Vec>(); v",
        "let v = (0..).iter().map(|v| `${v}`).collect::<Vec>(); v",
        // Doubling.
        "let v = [0]; while true { v = [v, v]; } v",
        // Sorting allocates scratch space of its own.
        "let v = []; let i = 0; while i < 100000 { v.push(i); i += 1; } v.sort(); v",
        // Formatting builds a string.
        "let s = \"x\"; while true { s = format!(\"{s}{s}\"); } s",
    ];

    for source in cases {
        let error = constrained(source).expect_err("Should not have been allowed to finish");

        // Which limit stopped it is not the point - that one of them did is.
        assert!(
            error.contains("allocate") || error.contains("budget") || error.contains("capacity"),
            "{source}: {error}"
        );
    }
}

/// A script which stays inside the limit still runs, so the limit only rejects
/// what actually asks for too much.
#[test]
fn allocating_within_the_limit_runs() {
    let cases = [
        "let v = []; let i = 0; while i < 1000 { v.push(i); i += 1; } v.len()",
        "let s = \"\"; let i = 0; while i < 1000 { s += \"x\"; i += 1; } s.len()",
        "let v = []; let i = 0; while i < 1000 { v.push(1000 - i); i += 1; } v.sort(); v[0]",
    ];

    for source in cases {
        constrained(source).unwrap_or_else(|error| panic!("{source}: {error}"));
    }
}

/// Compiling is bounded by the same limit as running.
///
/// A host which decides how much a script may have has decided it for the whole
/// of what it does with it, and compilation is not exempt - it allocates far
/// more than running does. Anything in the compiler which reached for the
/// global allocator would show up here as the process dying rather than as an
/// error.
#[test]
fn compiling_past_the_limit_is_an_error() {
    let context = Context::with_default_modules().expect("Failed to build context");

    let mut source = rust_alloc::string::String::new();

    for i in 0..20000 {
        source.push_str(&rust_alloc::format!("fn f{i}() {{ let a = {i}; a + 1 }}\n"));
    }

    source.push('1');

    for bytes in [1 << 12, 1 << 16, 1 << 20] {
        let result = limit::with(bytes, || {
            let mut sources = Sources::new();
            sources.insert(Source::memory(&source)?)?;

            let mut diagnostics = Diagnostics::new();

            let mut options = Options::default();
            options.script(true);

            crate::prepare(&mut sources)
                .with_context(&context)
                .with_diagnostics(&mut diagnostics)
                .with_options(&options)
                .build()?;

            Ok::<_, crate::support::Error>(())
        })
        .call();

        assert!(result.is_err(), "{bytes} bytes should not have been enough");
    }

    // With room to work in it compiles, so the limit only rejects what actually
    // asks for too much.
    let result = limit::with(1 << 30, || {
        let mut sources = Sources::new();
        sources.insert(Source::memory(&source)?)?;

        let mut diagnostics = Diagnostics::new();

        let mut options = Options::default();
        options.script(true);

        crate::prepare(&mut sources)
            .with_context(&context)
            .with_diagnostics(&mut diagnostics)
            .with_options(&options)
            .build()?;

        Ok::<_, crate::support::Error>(())
    })
    .call();

    result.expect("Source should compile");
}

/// A size hint carried by a unit does not decide how much is reserved.
///
/// `StringConcat` says how long the result is expected to be and the machine
/// reserved exactly that, up front, before writing any of it. That number is
/// this compiler's estimate only while the unit was compiled here: with
/// `-O bytecode=true` the CLI writes a `.rnc` beside a script and reads it back
/// on the next run, so it is whatever that file says - and reserving what a
/// file asks for is how one loaded from somewhere else takes the host with it.
///
/// The unit is built by hand because that is the only way to write a hint the
/// compiler would never emit; the same clamp covers `SIZE_HINT` coming back
/// from a script's own iterator, which `bounded_natives.rs` exercises.
#[test]
fn a_size_hint_carried_by_a_unit_does_not_decide_what_is_reserved() {
    use crate::runtime::inst::Kind;
    use crate::runtime::unit::{DefaultStorage, UnitEncoder, UnitFn};
    use crate::runtime::{Address, Call, Inst, Output, StaticString, Unit};

    let mut storage = DefaultStorage::default();

    for kind in [
        Kind::Allocate { size: 2 },
        Kind::String {
            slot: 0,
            out: Output::keep(0),
        },
        // A hint no machine could honour, which is the point.
        Kind::StringConcat {
            addr: Address::new(0),
            len: 1,
            size_hint: usize::MAX / 2,
            out: Output::keep(1),
        },
        Kind::Return {
            addr: Address::new(1),
        },
    ] {
        storage
            .encode(Inst::new(kind))
            .expect("Instruction should encode");
    }

    let mut functions = crate::hash::Map::default();

    functions
        .try_insert(
            Hash::EMPTY,
            UnitFn::Offset {
                offset: 0,
                call: Call::Immediate,
                args: 0,
                captures: None,
            },
        )
        .expect("Allocating the function table");

    let mut static_strings = crate::alloc::Vec::new();

    static_strings
        .try_push(
            Arc::try_new(StaticString::new("aaaabbbb").expect("Allocating the string"))
                .expect("Allocating the string"),
        )
        .expect("Allocating the string table");

    let unit = Unit::new(
        storage,
        functions,
        static_strings,
        crate::alloc::Vec::new(),
        crate::alloc::Vec::new(),
        crate::alloc::Vec::new(),
        crate::hash::Map::default(),
        None,
        crate::hash::Map::default(),
        crate::alloc::Vec::new(),
        crate::hash::Map::default(),
    );

    let context = Context::with_default_modules().expect("Failed to build context");
    let runtime = Arc::try_new(context.runtime().expect("Runtime")).expect("Allocating runtime");
    let unit = Arc::try_new(unit).expect("Allocating unit");
    let mut vm = Vm::new(runtime, unit);

    let value = budget::with(BUDGET, limit::with(LIMIT, || vm.call(Hash::EMPTY, ())))
        .call()
        .expect("A hint is not a reason to allocate");

    let string: String = crate::from_value(value).expect("The result is a string");
    assert_eq!(string, "aaaabbbb");
}
