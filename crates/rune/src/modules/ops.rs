//! The `std::ops` module.

use core::cmp::Ordering;

use crate as rune;
use crate::runtime::{
    ControlFlow, EnvProtocolCaller, Function, Generator, GeneratorState, Iterator, Protocol, Range,
    RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive, Value, Vm, VmResult,
};
use crate::{ContextError, Module};

#[rune::module(::std::ops)]
/// Overloadable operators.
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module_meta);

    {
        m.ty::<RangeFrom>()?;
        m.type_meta::<RangeFrom>()?
            .make_named_struct(&["start"])?
            .docs([
                "Type for a from range expression `start..`.",
                "",
                "# Examples",
                "",
                "```rune",
                "let range = 0..;",
                "",
                "assert!(!range.contains(-10));",
                "assert!(range.contains(5));",
                "assert!(range.contains(10));",
                "assert!(range.contains(20));",
                "",
                "assert!(range is std::ops::RangeFrom);",
                "```",
                "",
                "Ranges can contain any type:",
                "",
                "```rune",
                "let range = 'a'..;",
                "assert_eq!(range.start, 'a');",
                "range.start = 'b';",
                "assert_eq!(range.start, 'b');",
                "```",
                "",
                "Certain ranges can be used as iterators:",
                "",
                "```rune",
                "let range = 'a'..;",
                "assert_eq!(range.iter().take(5).collect::<Vec>(), ['a', 'b', 'c', 'd', 'e']);",
                "```",
            ]);
        m.field_function(Protocol::GET, "start", |r: &RangeFrom| r.start.clone())?;
        m.field_function(Protocol::SET, "start", |r: &mut RangeFrom, value: Value| {
            r.start = value;
        })?;
        m.function_meta(RangeFrom::iter__meta)?;
        m.function_meta(RangeFrom::contains__meta)?;
        m.function_meta(RangeFrom::into_iter__meta)?;
        m.function_meta(RangeFrom::partial_eq__meta)?;
        m.function_meta(RangeFrom::eq__meta)?;
        m.function_meta(RangeFrom::partial_cmp__meta)?;
        m.function_meta(RangeFrom::cmp__meta)?;
    }

    {
        m.ty::<RangeFull>()?;
        m.type_meta::<RangeFull>()?.make_empty_struct()?.docs([
            "Type for a full range expression `..`.",
            "",
            "# Examples",
            "",
            "```rune",
            "let range = ..;",
            "",
            "assert!(range.contains(-10));",
            "assert!(range.contains(5));",
            "assert!(range.contains(10));",
            "assert!(range.contains(20));",
            "",
            "assert!(range is std::ops::RangeFull);",
            "```",
        ]);
        m.function_meta(RangeFull::contains)?;
    }

    {
        m.ty::<RangeInclusive>()?;
        m.type_meta::<RangeInclusive>()?
            .make_named_struct(&["start", "end"])?
            .docs([
                "Type for an inclusive range expression `start..=end`.",
                "",
                "# Examples",
                "",
                "```rune",
                "let range = 0..=10;",
                "",
                "assert!(!range.contains(-10));",
                "assert!(range.contains(5));",
                "assert!(range.contains(10));",
                "assert!(!range.contains(20));",
                "",
                "assert!(range is std::ops::RangeInclusive);",
                "```",
                "",
                "Ranges can contain any type:",
                "",
                "```rune",
                "let range = 'a'..='f';",
                "assert_eq!(range.start, 'a');",
                "range.start = 'b';",
                "assert_eq!(range.start, 'b');",
                "assert_eq!(range.end, 'f');",
                "range.end = 'g';",
                "assert_eq!(range.end, 'g');",
                "```",
                "",
                "Certain ranges can be used as iterators:",
                "",
                "```rune",
                "let range = 'a'..='e';",
                "assert_eq!(range.iter().collect::<Vec>(), ['a', 'b', 'c', 'd', 'e']);",
                "```",
            ]);
        m.field_function(Protocol::GET, "start", |r: &RangeInclusive| r.start.clone())?;
        m.field_function(
            Protocol::SET,
            "start",
            |r: &mut RangeInclusive, value: Value| {
                r.start = value;
            },
        )?;
        m.field_function(Protocol::GET, "end", |r: &RangeInclusive| r.end.clone())?;
        m.field_function(
            Protocol::SET,
            "end",
            |r: &mut RangeInclusive, value: Value| {
                r.end = value;
            },
        )?;
        m.function_meta(RangeInclusive::iter__meta)?;
        m.function_meta(RangeInclusive::contains__meta)?;
        m.function_meta(RangeInclusive::into_iter__meta)?;
        m.function_meta(RangeInclusive::partial_eq__meta)?;
        m.function_meta(RangeInclusive::eq__meta)?;
        m.function_meta(RangeInclusive::partial_cmp__meta)?;
        m.function_meta(RangeInclusive::cmp__meta)?;
    }

    {
        m.ty::<RangeToInclusive>()?;
        m.type_meta::<RangeToInclusive>()?
            .make_named_struct(&["end"])?
            .docs([
                "Type for an inclusive range expression `..=end`.",
                "",
                "# Examples",
                "",
                "```rune",
                "let range = ..=10;",
                "assert!(range.contains(-10));",
                "assert!(range.contains(5));",
                "assert!(range.contains(10));",
                "assert!(!range.contains(20));",
                "",
                "assert!(range is std::ops::RangeToInclusive);",
                "```",
                "",
                "Ranges can contain any type:",
                "",
                "```rune",
                "let range = ..='f';",
                "assert_eq!(range.end, 'f');",
                "range.end = 'g';",
                "assert_eq!(range.end, 'g');",
                "```",
            ]);
        m.field_function(Protocol::GET, "end", |r: &RangeToInclusive| r.end.clone())?;
        m.field_function(
            Protocol::SET,
            "end",
            |r: &mut RangeToInclusive, value: Value| {
                r.end = value;
            },
        )?;
        m.function_meta(RangeToInclusive::contains__meta)?;
        m.function_meta(RangeToInclusive::partial_eq__meta)?;
        m.function_meta(RangeToInclusive::eq__meta)?;
        m.function_meta(RangeToInclusive::partial_cmp__meta)?;
        m.function_meta(RangeToInclusive::cmp__meta)?;
    }

    {
        m.ty::<RangeTo>()?;
        m.type_meta::<RangeTo>()?
            .make_named_struct(&["end"])?
            .docs([
                "Type for an inclusive range expression `..end`.",
                "",
                "# Examples",
                "",
                "```rune",
                "let range = ..10;",
                "assert!(range.contains(-10));",
                "assert!(range.contains(5));",
                "assert!(!range.contains(10));",
                "assert!(!range.contains(20));",
                "",
                "assert!(range is std::ops::RangeTo);",
                "```",
                "",
                "Ranges can contain any type:",
                "",
                "```rune",
                "let range = ..'f';",
                "assert_eq!(range.end, 'f');",
                "range.end = 'g';",
                "assert_eq!(range.end, 'g');",
                "```",
            ]);
        m.field_function(Protocol::GET, "end", |r: &RangeTo| r.end.clone())?;
        m.field_function(Protocol::SET, "end", |r: &mut RangeTo, value: Value| {
            r.end = value;
        })?;
        m.function_meta(RangeTo::contains__meta)?;
        m.function_meta(RangeTo::partial_eq__meta)?;
        m.function_meta(RangeTo::eq__meta)?;
        m.function_meta(RangeTo::partial_cmp__meta)?;
        m.function_meta(RangeTo::cmp__meta)?;
    }

    {
        m.ty::<Range>()?;
        m.type_meta::<Range>()?
            .make_named_struct(&["start", "end"])?
            .docs([
                "Type for a range expression `start..end`.",
                "",
                "# Examples",
                "",
                "```rune",
                "let range = 0..10;",
                "assert!(!range.contains(-10));",
                "assert!(range.contains(5));",
                "assert!(!range.contains(10));",
                "assert!(!range.contains(20));",
                "",
                "assert!(range is std::ops::Range);",
                "```",
                "",
                "Ranges can contain any type:",
                "",
                "```rune",
                "let range = 'a'..'f';",
                "assert_eq!(range.start, 'a');",
                "range.start = 'b';",
                "assert_eq!(range.start, 'b');",
                "assert_eq!(range.end, 'f');",
                "range.end = 'g';",
                "assert_eq!(range.end, 'g');",
                "```",
                "",
                "Certain ranges can be used as iterators:",
                "",
                "```rune",
                "let range = 'a'..'e';",
                "assert_eq!(range.iter().collect::<Vec>(), ['a', 'b', 'c', 'd']);",
                "```",
            ]);
        m.field_function(Protocol::GET, "start", |r: &Range| r.start.clone())?;
        m.field_function(Protocol::SET, "start", |r: &mut Range, value: Value| {
            r.start = value;
        })?;
        m.field_function(Protocol::GET, "end", |r: &Range| r.end.clone())?;
        m.field_function(Protocol::SET, "end", |r: &mut Range, value: Value| {
            r.end = value;
        })?;
        m.function_meta(Range::iter__meta)?;
        m.function_meta(Range::into_iter__meta)?;
        m.function_meta(Range::contains__meta)?;
        m.function_meta(Range::partial_eq__meta)?;
        m.function_meta(Range::eq__meta)?;
        m.function_meta(Range::partial_cmp__meta)?;
        m.function_meta(Range::cmp__meta)?;
    }

    {
        m.ty::<ControlFlow>()?.docs([
            " Used to tell an operation whether it should exit early or go on as usual.",
            "",
            " This acts as the basis of the [`TRY`] protocol in Rune.",
            "",
            " [`TRY`]: crate::Protocol::TRY",
            "",
            "# Examples",
            "",
            "```rune",
            "use std::ops::ControlFlow;",
            "",
            "let c = ControlFlow::Continue(42);",
            "assert_eq!(c.0, 42);",
            "assert_eq!(c, ControlFlow::Continue(42));",
            "```",
        ]);
    }

    m.ty::<Function>()?.docs([
        "The type of a function in Rune.",
        "",
        "Functions can be called using call expression syntax, such as `<expr>()`.",
        "",
        "There are multiple different kind of things which can be coerced into a function in Rune:",
        "* Regular functions.",
        "* Closures (which might or might not capture their environment).",
        "* Built-in constructors for tuple types (tuple structs, tuple variants).",
        "",
        "# Examples",
        "",
        "```rune",
        "// Captures the constructor for the `Some(<value>)` tuple variant.",
        "let build_some = Some;",
        "assert_eq!(build_some(42), Some(42));",
        "",
        "fn build(value) {",
        "    Some(value)",
        "}",
        "",
        "// Captures the function previously defined.",
        "let build_some = build;",
        "assert_eq!(build_some(42), Some(42));",
        "```",
    ]);

    {
        m.ty::<Generator<Vm>>()?.docs([
            "The return value of a function producing a generator.",
            "",
            "Functions which contain the `yield` keyword produces generators.",
            "",
            "# Examples",
            "",
            "```rune",
            "use std::ops::Generator;",
            "",
            "fn generate() {",
            "    yield 1;",
            "    yield 2;",
            "}",
            "",
            "let g = generate();",
            "assert!(g is Generator)",
            "```",
        ]);

        m.function_meta(generator_next)?;
        m.function_meta(generator_resume)?;
        m.function_meta(generator_iter)?;
        m.function_meta(generator_into_iter)?;
    }

    {
        m.generator_state(["GeneratorState"])?
            .docs(["Enum indicating the state of a generator."]);

        m.function_meta(generator_state_partial_eq)?;
        m.function_meta(generator_state_eq)?;
    }

    m.function_meta(partial_eq)?;
    m.function_meta(eq)?;
    m.function_meta(partial_cmp)?;
    m.function_meta(cmp)?;
    Ok(m)
}

/// Perform a partial equality check over two values.
///
/// This produces the same behavior as the equality operator (`==`).
///
/// For non-builtin types this leans on the behavior of the [`PARTIAL_EQ`]
/// protocol.
///
/// # Panics
///
/// Panics if we're trying to compare two values which are not comparable.
///
/// # Examples
///
/// ```rune
/// use std::ops::partial_eq;
///
/// assert!(partial_eq(1.0, 1.0));
/// assert!(!partial_eq(1.0, 2.0));
/// ```
#[rune::function]
fn partial_eq(lhs: Value, rhs: Value) -> VmResult<bool> {
    Value::partial_eq(&lhs, &rhs)
}

/// Perform a partial equality check over two values.
///
/// This produces the same behavior as the equality operator (`==`).
///
/// For non-builtin types this leans on the behavior of the [`EQ`] protocol.
///
/// # Panics
///
/// Panics if we're trying to compare two values which are not comparable.
///
/// # Examples
///
/// ```rune
/// use std::ops::eq;
///
/// assert!(eq(1.0, 1.0));
/// assert!(!eq(1.0, 2.0));
/// ```
#[rune::function]
fn eq(lhs: Value, rhs: Value) -> VmResult<bool> {
    Value::eq(&lhs, &rhs)
}

/// Perform a partial comparison over two values.
///
/// This produces the same behavior as when comparison operators like less than
/// (`<`) is used.
///
/// For non-builtin types this leans on the behavior of the [`PARTIAL_CMP`]
/// protocol.
///
/// # Panics
///
/// Panics if we're trying to compare two values which are not comparable.
///
/// # Examples
///
/// ```rune
/// use std::ops::partial_cmp;
/// use std::cmp::Ordering;
///
/// assert_eq!(partial_cmp(1.0, 1.0), Some(Ordering::Equal));
/// assert_eq!(partial_cmp(1.0, 2.0), Some(Ordering::Less));
/// assert_eq!(partial_cmp(1.0, f64::NAN), None);
/// ```
#[rune::function]
fn partial_cmp(lhs: Value, rhs: Value) -> VmResult<Option<Ordering>> {
    Value::partial_cmp(&lhs, &rhs)
}

/// Perform a total comparison over two values.
///
/// For non-builtin types this leans on the behavior of the [`CMP`] protocol.
///
/// # Panics
///
/// Panics if we're trying to compare two values which are not comparable.
///
/// ```rune,should_panic
/// use std::ops::cmp;
///
/// let _ = cmp(1.0, f64::NAN);
/// ```
///
/// # Examples
///
/// ```rune
/// use std::ops::cmp;
/// use std::cmp::Ordering;
///
/// assert_eq!(cmp(1, 1), Ordering::Equal);
/// assert_eq!(cmp(1, 2), Ordering::Less);
/// ```
#[rune::function]
fn cmp(lhs: Value, rhs: Value) -> VmResult<Ordering> {
    Value::cmp(&lhs, &rhs)
}

/// Advance a generator producing the next value yielded.
///
/// Unlike [`Generator::resume`], this can only consume the yielded values.
///
/// # Examples
///
/// ```rune
/// use std::ops::{Generator, GeneratorState};
///
/// fn generate() {
///     yield 1;
///     yield 2;
/// }
///
/// let g = generate();
///
/// assert_eq!(g.next(), Some(1));
/// assert_eq!(g.next(), Some(2));
/// assert_eq!(g.next(), None);
/// ``
#[rune::function(instance, path = next)]
fn generator_next(this: &mut Generator<Vm>) -> VmResult<Option<Value>> {
    this.next()
}

/// Advance a generator producing the next [`GeneratorState`].
///
/// # Examples
///
/// ```rune
/// use std::ops::{Generator, GeneratorState};
///
/// fn generate() {
///     let n = yield 1;
///     yield 2 + n;
/// }
///
/// let g = generate();
///
/// assert_eq!(g.resume(()), GeneratorState::Yielded(1));
/// assert_eq!(g.resume(1), GeneratorState::Yielded(3));
/// assert_eq!(g.resume(()), GeneratorState::Complete(()));
/// ``
#[rune::function(instance, path = resume)]
fn generator_resume(this: &mut Generator<Vm>, value: Value) -> VmResult<GeneratorState> {
    this.resume(value)
}

#[rune::function(instance, path = iter)]
fn generator_iter(this: Generator<Vm>) -> Iterator {
    this.rune_iter()
}

#[rune::function(instance, protocol = INTO_ITER)]
fn generator_into_iter(this: Generator<Vm>) -> Iterator {
    this.rune_iter()
}

/// Test for partial equality over a generator state.
///
/// # Examples
///
/// ```rune
/// use std::ops::{Generator, GeneratorState};
///
/// fn generate() {
///     let n = yield 1;
///     yield 2 + n;
/// }
///
/// let g = generate();
///
/// assert_eq!(g.resume(()), GeneratorState::Yielded(1));
/// assert_eq!(g.resume(1), GeneratorState::Yielded(3));
/// assert_eq!(g.resume(()), GeneratorState::Complete(()));
/// ``
#[rune::function(instance, protocol = PARTIAL_EQ)]
fn generator_state_partial_eq(this: &GeneratorState, other: &GeneratorState) -> VmResult<bool> {
    this.partial_eq_with(other, &mut EnvProtocolCaller)
}

/// Test for total equality over a generator state.
///
/// # Examples
///
/// ```rune
/// use std::ops::{Generator, GeneratorState};
/// use std::ops::eq;
///
/// fn generate() {
///     let n = yield 1;
///     yield 2 + n;
/// }
///
/// let g = generate();
///
/// assert!(eq(g.resume(()), GeneratorState::Yielded(1)));
/// assert!(eq(g.resume(1), GeneratorState::Yielded(3)));
/// assert!(eq(g.resume(()), GeneratorState::Complete(())));
/// ``
#[rune::function(instance, protocol = EQ)]
fn generator_state_eq(this: &GeneratorState, other: &GeneratorState) -> VmResult<bool> {
    this.eq_with(other, &mut EnvProtocolCaller)
}
