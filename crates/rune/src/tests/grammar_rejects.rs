//! Negative tests for syntax which the grammar accepts but which is not a
//! legal program.
//!
//! The grammar is shared with the formatter, so it is deliberately permissive:
//! it accepts modifiers, labels and attributes wherever they could plausibly be
//! written, and it treats most punctuation as optional so that an incomplete
//! source still produces a tree to format. Rejecting what it lets through is
//! the job of indexing and lowering.
//!
//! Every source below parses. None of them may compile.

prelude!();

use rust_alloc::string::{String, ToString};

use crate::diagnostics::Diagnostic;

/// Compile `source` as a script, returning the message of the first fatal
/// diagnostic.
///
/// Panics if the source compiles, since that is the thing each test is here to
/// rule out.
#[track_caller]
fn reject(source: &str) -> String {
    let mut diagnostics = Diagnostics::new();

    if crate::tests::compile_helper(source, &mut diagnostics).is_ok() {
        panic!("source compiled but should have been rejected:\n{source}");
    }

    for diagnostic in diagnostics.into_diagnostics() {
        if let Diagnostic::Fatal(error) = diagnostic {
            return error.to_string();
        }
    }

    panic!("source did not compile but produced no fatal diagnostic:\n{source}");
}

/// Assert that `source` is rejected with the given message.
#[track_caller]
fn denied(source: &str, expected: &str) {
    let actual = reject(source);
    assert_eq!(actual, expected, "for source:\n{source}");
}

/// The grammar hangs modifiers off the front of every expression. None of them
/// mean anything there, apart from the ones a closure or a block consumes.
#[test]
fn deny_modifiers_on_expression() {
    denied("let x = pub 1;", "unsupported visibility modifier");
    denied("let x = pub(crate) 1;", "unsupported visibility modifier");
    denied("let x = const 1;", "unsupported `const` modifier");
    denied("let x = static 1;", "unsupported `static` modifier");
    denied("let x = async 1;", "unsupported `async` modifier");
    denied("let x = move 1;", "unsupported `move` modifier");
    denied("let x = [move 1];", "unsupported `move` modifier");
    denied("let x = #{a: pub 1};", "unsupported visibility modifier");
    denied("match 1 { _ => pub 2 }", "unsupported visibility modifier");
    denied(
        "match 1 { _ if pub true => () }",
        "unsupported visibility modifier",
    );
}

/// A block takes `async` or `const`, and `move` only alongside `async`, since
/// that is the only one which captures. Nothing else may carry a modifier, even
/// though every one of these has a block the modifier could be mistaken for.
#[test]
fn deny_modifiers_on_block_like() {
    denied("let x = move { 1 };", "unsupported `move` modifier");
    denied("let x = const move { 1 };", "unsupported `move` modifier");
    denied("async if true {}", "unsupported `async` modifier");
    denied("async loop { break }", "unsupported `async` modifier");
    denied("async while false {}", "unsupported `async` modifier");
    denied("async for a in [] {}", "unsupported `async` modifier");
    denied("async match 1 { _ => () }", "unsupported `async` modifier");

    // The block is one level in, so the modifier is not the one it would carry
    // if it had been written directly in front of it.
    denied("let x = async ({ 1 });", "unsupported `async` modifier");
    denied("let x = async [{ 1 }];", "unsupported `async` modifier");
    denied("let x = const ({ 1 });", "unsupported `const` modifier");
}

/// A modifier is consumed by the closure or block it belongs to, so a repeat of
/// it has nothing left to apply to.
#[test]
fn deny_duplicate_modifiers() {
    denied("let f = async async || 1;", "duplicate `async` modifier");
    denied("let f = move move || 1;", "duplicate `move` modifier");
    denied("let x = const const { 1 };", "duplicate `const` modifier");
}

/// The modifiers of an item are parsed in a fixed order, so writing them in any
/// other one leaves the rest of them unconsumed.
#[test]
fn deny_out_of_order_modifiers() {
    denied(
        "async pub fn f() {}",
        "Expected end of syntax but got `pub` keyword while parsing modifiers",
    );
    denied(
        "const pub fn f() {}",
        "Expected end of syntax but got `pub` keyword while parsing modifiers",
    );
    denied(
        "async const FOO = 1;",
        "Expected end of syntax but got `const` keyword while parsing modifiers",
    );
}

/// The grammar lets any item carry any modifier, since which ones are legal
/// depends on the item it turns out to be.
#[test]
fn deny_modifiers_on_item() {
    denied("const struct S;", "unsupported `const` modifier");
    denied("static enum E { A }", "unsupported `static` modifier");
    denied("async mod m {}", "unsupported `async` modifier");
    denied("move mod m {}", "unsupported `move` modifier");
    denied("move use std::string;", "unsupported `move` modifier");
    denied("struct S; move impl S {}", "unsupported `move` modifier");
    denied("move struct S;", "unsupported `move` modifier");

    // A function takes `const` and `async`, but not the other two.
    denied("static fn f() {}", "unsupported `static` modifier");
    denied("move fn f() {}", "unsupported `move` modifier");

    denied("const static FOO = 1;", "unsupported `static` modifier");
    denied("static move FOO = 1;", "unsupported `move` modifier");
}

/// The initializer of a `const` or a `static` is deferred for constant
/// evaluation, so it has to be walked as an expression rather than as a bare
/// stream, or the prefix the grammar allows in front of it goes unchecked.
#[test]
fn deny_modifiers_on_const_initializer() {
    denied("const FOO = pub 1;", "unsupported visibility modifier");
    denied("const FOO = async 1;", "unsupported `async` modifier");
    denied("const FOO = static 1;", "unsupported `static` modifier");
    denied("const FOO = move 1;", "unsupported `move` modifier");
    denied("const FOO = #[bogus] 1;", "unsupported attribute `bogus`");

    denied("static FOO = pub 1;", "unsupported visibility modifier");
    denied("static FOO = move 1;", "unsupported `move` modifier");
    denied("static FOO = #[bogus] 1;", "unsupported attribute `bogus`");
}

/// Labels are accepted in front of every expression, but only a loop can carry
/// one, and only one of them.
#[test]
fn deny_labels() {
    denied("let x = 'a: 1;", "labels are not supported for expression");
    denied("'a: if true {}", "labels are not supported for expression");
    denied(
        "'a: match 1 { _ => () }",
        "labels are not supported for expression",
    );
    denied("'a: 'b: loop { break 'a; }", "Multiple labels provided");
    denied(
        "'a: loop { break 'a 'b; }",
        "Expected end of syntax but got a label while parsing a `break` expression",
    );
}

/// Attributes are accepted wherever one could be written, and every one which
/// is not recognized has to be reported rather than dropped.
#[test]
fn deny_unsupported_attributes() {
    denied("let x = #[bogus] 1;", "unsupported attribute `bogus`");
    denied("struct S { #[bogus] a }", "unsupported attribute `bogus`");
    denied("struct S(#[bogus] a);", "unsupported attribute `bogus`");
    denied("enum E { #[bogus] A }", "unsupported attribute `bogus`");
    denied("fn f(#[bogus] a) {}", "unsupported attribute `bogus`");
    denied("let f = |#[bogus] a| a;", "unsupported attribute `bogus`");
    denied("fn f() { #![bogus] }", "unsupported attribute `bogus`");
}

/// Most punctuation is consumed with `bump_if`, so its absence produces a tree
/// which is missing a child rather than a parse error.
#[test]
fn deny_missing_tokens() {
    denied(
        "let x",
        "Expected `=` but got an expression while parsing a variable declaration",
    );
    denied(
        "for x { }",
        "Expected `in` keyword but got a block while parsing a `for` expression",
    );
    denied(
        "if let Some(x) { }",
        "Expected `=` but got end of syntax while parsing the `let` condition of a loop",
    );
    denied(
        "while let Some(x) { }",
        "Expected `=` but got end of syntax while parsing the `let` condition of a loop",
    );
    denied(
        "match 1 { 1 }",
        "Expected `=>` but got end of syntax while parsing a match arm",
    );
    denied(
        "match 1 { 1 if true }",
        "Expected `=>` but got end of syntax while parsing a match arm",
    );
    denied(
        "async fn f() { select { default } }",
        "Expected `=>` but got end of syntax while parsing a select arm",
    );
    denied(
        "let x = (1;",
        "Expected `)` delimiter but got end of syntax while parsing a group expression",
    );
    denied(
        "struct S { a",
        "Expected `}` delimiter while parsing a struct body",
    );
}

/// The name of an item is optional in the grammar, so that a half-written item
/// still formats.
#[test]
fn deny_missing_names() {
    denied("fn () {}", "expected function name");
    denied("struct;", "expected struct name");
    denied("enum { A }", "expected enum name");
    denied("mod { }", "expected module name");
    denied("impl { fn f() {} }", "Expected a path but got an impl");
}

/// A separator is consumed with `bump_while`, so a run of them parses.
#[test]
fn deny_repeated_separators() {
    denied(
        "struct S { a,,, b }",
        "Expected one `,` but got 3 of them while parsing a struct body",
    );
    denied(
        "struct S(a,,, b);",
        "Expected one `,` but got 3 of them while parsing a tuple body",
    );
    denied(
        "enum E { A,,, B }",
        "Expected one `,` but got 3 of them while parsing an enum declaration",
    );
    denied(
        "let f = |a,,, b| a;",
        "Expected one `,` but got 3 of them while parsing closure arguments",
    );
    denied(
        "fn f() { let x = 1 let y = 2; }",
        "Expected one `;` while parsing the body of a block",
    );
}

/// The field of a field access is parsed as a whole path, since that is what
/// makes `a.b` and a bare `b` the same production.
#[test]
fn deny_path_field_access() {
    denied(
        "let a = #{b: 1}; a.b::c",
        "Expected the generics of a path but got an identifier while parsing an indexed path",
    );
    denied("let a = #{b: 1}; a.b::<i64>", "Unsupported field access");
    denied(
        "let a = #{b: 1}; a.self",
        "Expected a path but got an indexed path",
    );
}

/// An import is a flat run of components, so the shapes which cannot mean
/// anything are rejected once the path is resolved.
#[test]
fn deny_malformed_imports() {
    denied(
        "use std::*::string;",
        "Another segment can't follow wildcard `*` or group imports",
    );
    denied(
        "use std::{string}::foo;",
        "Another segment can't follow wildcard `*` or group imports",
    );
    denied(
        "use std::* as s;",
        "Use aliasing is not supported for wildcard `*` or group imports",
    );
    denied(
        "use std::self::string;",
        "Segment is only supported in the first position",
    );
}

/// The body of an `impl` is parsed as a plain block.
#[test]
fn deny_non_functions_in_impl() {
    denied(
        "struct S; impl S { let x = 1; }",
        "Expected end of syntax but got a variable declaration while parsing the body of a block",
    );
    denied(
        "struct S; impl S { struct T; }",
        "Expected a function declaration but got a struct declaration",
    );
    denied(
        "struct S; impl S { 1 + 1 }",
        "Expected end of syntax but got an expression while parsing the body of a block",
    );
}

/// Assorted productions the grammar admits which have no meaning.
#[test]
fn deny_unsupported_expressions() {
    denied(
        "let mut x = 1;",
        "The `mut` modifier is not supported in Rune, everything is mutable by default",
    );
    denied(
        "let y = (let x = 1);",
        "Expected an expression but got `let` keyword",
    );
    denied(
        "let x = 1..=;",
        "Unsupported range, you probably want `..` instead of `..=`",
    );
    denied(
        "let x = Vec<i64>;",
        "Group required in expression to determine precedence",
    );
    denied(";", "Expected an expression or local but got an error");
    denied("let x = ;", "Expected an expression but got an error");
}

/// A macro and an attribute take a raw token stream, which is read until the
/// delimiter it was opened with is closed again.
///
/// The end of the input closes nothing, so reading past it went on forever:
/// every source below hung the compiler rather than being rejected, and grew
/// the tree it was building while it did.
#[test]
fn deny_an_unclosed_token_stream() {
    denied("n!(", "Expected `)` delimiter but got eof");
    denied("n!(a", "Expected `)` delimiter but got eof");
    denied("n!((", "Expected `)` delimiter but got eof");
    denied("n![", "Expected `]` delimiter but got eof");
    denied("n!{", "Expected `}` delimiter but got eof");
    denied("let x = n!(;", "Expected `)` delimiter but got eof");
    denied("fn f() { n!( }", "Expected `)` delimiter but got eof");
    denied("#[a", "Expected `]` delimiter but got eof");

    // A closed one is still read.
    let out: i64 = rune!(
        #[test]
        fn ignored() {}
        let x = [1, 2, 3];
        x.len()
    );
    assert_eq!(out, 3);
}
