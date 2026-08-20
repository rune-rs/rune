//! What a machine error says when it is emitted against the source it came
//! from.
//!
//! The error itself carries what the machine had - a slot, a hash - because
//! that is all it has at the point it is raised. What the name was is in the
//! unit next to the instruction which asked for it, so the emitter is where the
//! two are put back together.

prelude!();

use crate::termcolor;

/// Run `source` and emit the error it produced against it.
fn emitted(source: &str) -> rust_alloc::string::String {
    let context = Context::with_default_modules().expect("Failed to build context");

    let mut sources = crate::tests::sources(source);
    let mut diagnostics = Diagnostics::new();

    let mut vm = crate::tests::vm(&context, &mut sources, &mut diagnostics, true)
        .expect("Source should compile");

    let error = vm
        .call(Hash::EMPTY, ())
        .expect_err("The program should not have run");

    let mut buffer = termcolor::Buffer::no_color();
    error.emit(&mut buffer, &sources).expect("Failed to emit");

    rust_alloc::string::String::from_utf8(buffer.into_inner()).expect("Output should be utf-8")
}

/// Reaching for a field which is not there says which field it was.
///
/// The error names the static slot the field name is in, which is a number and
/// tells whoever wrote it nothing.
#[test]
fn a_missing_field_is_named() {
    let out = emitted("let a = #{}; a.x");

    assert!(out.contains("This corresponds to the field `x`"), "{out}");

    // The same for a field of a struct, which is reached the same way.
    let out = emitted("struct S { a } let s = S { a: 1 }; s.b");

    assert!(out.contains("This corresponds to the field `b`"), "{out}");
}

/// Calling an instance function which is not there says which one it was.
#[test]
fn a_missing_instance_function_is_named() {
    let out = emitted("let a = 1; a.nope()");

    assert!(
        out.contains("`::std::i64::nope` instance function"),
        "{out}"
    );
}
