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

/// What `as` cannot do says what it can, rather than naming a hash.
///
/// The type being converted to is not something the machine can name - all it
/// has is a hash - and a hash tells whoever wrote it nothing which the source in
/// front of them does not already say.
#[test]
fn an_unsupported_cast_says_what_as_converts() {
    // The value is not something `as` converts.
    let out = emitted(r#"let a = "x" as i64; a"#);

    assert!(
        out.contains("cannot be converted with `as`, which only converts between"),
        "{out}"
    );

    // The value is, but what it was asked to convert to is not.
    let out = emitted("let a = 1 as String; a");

    assert!(
        out.contains("to that type, which only converts to"),
        "{out}"
    );

    assert!(!out.contains("0x"), "the hash should not be in it: {out}");
}

/// Being unable to reach a value says what is being done with it.
///
/// The error carries the access flags, since that is all it has where it is
/// raised, and `M-000000` written into a message tells whoever reads it
/// nothing. Which of the three things is happening is the half worth saying,
/// and it is the most common thing to run into in a language where everything
/// is shared.
#[test]
fn an_unavailable_value_says_what_is_being_done_with_it() {
    let cases = [
        (
            "let a = #{f: 1}; let b = a; drop(b); a.f",
            "Cannot read, the value has been moved",
        ),
        (
            "let v = [1, 2, 3]; for x in v { v.push(1); } v",
            "Cannot write, the value is being read from",
        ),
        (
            "let v = [3, 1, 2, 5, 4]; v.sort_by(|a, b| { v.push(1); a.cmp(b) }); v",
            "Cannot write, the value is being written to",
        ),
        (
            "let v = [1, 2, 3]; for x in v { drop(v); } v",
            "Cannot take, the value is being read from",
        ),
    ];

    for (source, expected) in cases {
        let out = emitted(source);
        assert!(out.contains(expected), "{source}: {out}");
    }
}
