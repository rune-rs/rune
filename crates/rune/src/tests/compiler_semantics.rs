//! Direct execution tests for the compiler.
//!
//! These build and run a source through the whole pipeline rather than through
//! the shared helpers, so that basic semantics - control flow, constants and the
//! limits which bound them - are exercised without depending on how the rest of
//! the suite is invoked.

prelude!();

use crate::diagnostics::{Diagnostic, FatalDiagnosticKind};
use crate::Unit;

fn options() -> Options {
    let mut options = Options::default();
    options.script(true);
    options
}

/// Build `source`, returning the kind of the first fatal
/// diagnostic if it did not compile.
fn build(source: &str, options: &Options) -> Result<Unit, ErrorKind> {
    let context = Context::with_default_modules().expect("Failed to build context");

    let mut sources = Sources::new();
    sources
        .insert(Source::memory(source).expect("Failed to build source"))
        .expect("Failed to insert source");

    let mut diagnostics = Diagnostics::new();

    let result = crate::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .with_options(options)
        .build();

    if let Ok(unit) = result {
        return Ok(unit);
    }

    for diagnostic in diagnostics.into_diagnostics() {
        if let Diagnostic::Fatal(fatal) = diagnostic {
            if let FatalDiagnosticKind::CompileError(error) = fatal.into_kind() {
                return Err(error.into_kind());
            }
        }
    }

    panic!("Expected a compile error diagnostic");
}

/// Build and run `source`.
fn eval<T>(source: &str) -> T
where
    T: FromValue,
{
    let context = Context::with_default_modules().expect("Failed to build context");
    let unit = build(source, &options()).expect("Source should compile");

    let runtime = Arc::try_new(context.runtime().expect("Failed to build runtime"))
        .expect("Failed to allocate runtime");
    let unit = Arc::try_new(unit).expect("Failed to allocate unit");

    let mut vm = Vm::new(runtime, unit);
    let mut execution = vm.execute(Hash::EMPTY, ()).expect("Failed to execute");

    let output = block_on(execution.resume())
        .expect("Execution failed")
        .into_complete()
        .expect("Execution did not complete");

    crate::from_value(output).expect("Failed to convert output")
}

/// Binary operators are parsed by climbing precedences over the parser's work
/// stack, so how they group is worth pinning down independently of how deep
/// they are allowed to get.
#[test]
fn binary_precedence() {
    let cases = [
        ("1 + 2 * 3", 7),
        ("2 * 3 + 1", 7),
        ("1 + 2 * 3 - 4", 3),
        ("10 - 2 - 3", 5),
        ("100 / 5 / 2", 10),
        ("2 * 3 % 4", 2),
        ("1 | 2 & 3", 3),
        ("1 << 2 + 1", 8),
        ("1 + 2 * 3 * 4 + 5", 30),
        ("(1 + 2) * 3", 9),
        ("1 + (2 + (3 + 4))", 10),
    ];

    for (expr, expected) in cases {
        let value: i64 = eval(expr);
        assert_eq!(value, expected, "{expr}");
    }

    let cases = [
        ("true || false && false", true),
        ("1 + 2 == 3", true),
        ("1 + 2 * 2 == 5 && 2 * 2 + 1 == 5", true),
    ];

    for (expr, expected) in cases {
        let value: bool = eval(expr);
        assert_eq!(value, expected, "{expr}");
    }

    // Operators which share a precedence without being associative have to be
    // grouped, and the diagnostic covers the chain rather than the operator
    // which noticed it.
    let error = build("1 < 2 == true", &options()).expect_err("Should require a group");
    assert!(
        matches!(error, ErrorKind::PrecedenceGroupRequired),
        "{error:?}"
    );
}

/// An `if` without an `else` must not fall through into its branch when the
/// condition is false.
#[test]
fn if_without_else() {
    let value: i64 = eval(
        r#"
        let a = 0;
        if false {
            a = 1;
        }
        a
        "#,
    );

    assert_eq!(value, 0);

    let value: i64 = eval(
        r#"
        let a = 0;
        if true {
            a = 1;
        }
        a
        "#,
    );

    assert_eq!(value, 1);
}

/// The value of an `if` without an `else` is the unit produced by the fall
/// through path when no branch is taken.
#[test]
fn if_without_else_value() {
    let value: () = eval("if false { 1 }");
    assert_eq!(value, ());
}

#[test]
fn if_else() {
    let value: i64 = eval("if true { 1 } else { 2 }");
    assert_eq!(value, 1);

    let value: i64 = eval("if false { 1 } else { 2 }");
    assert_eq!(value, 2);
}

#[test]
fn else_if() {
    let value: i64 = eval("if false { 1 } else if true { 2 } else { 3 }");
    assert_eq!(value, 2);

    let value: i64 = eval("if false { 1 } else if false { 2 } else { 3 }");
    assert_eq!(value, 3);

    // Without a fallback the chain falls through to a unit.
    let value: () = eval("if false { 1 } else if false { 2 }");
    assert_eq!(value, ());
}

#[test]
fn if_let() {
    let value: i64 = eval("if let Some(a) = Some(1) { a } else { 2 }");
    assert_eq!(value, 1);

    let value: i64 = eval("if let Some(a) = None { a } else { 2 }");
    assert_eq!(value, 2);
}

/// A condition is assembled into a slot of its own, so the temporary produced
/// by a short circuiting operator must not clobber the left-hand side.
///
/// See <https://github.com/rune-rs/rune/issues/830>.
#[test]
fn conditional_operand_is_not_clobbered() {
    let value: bool = eval(
        r#"
        let value = true;

        if value && false {
            panic("should not be reached");
        }

        value
        "#,
    );

    assert!(value);

    let value: bool = eval(
        r#"
        let value = true;

        while value && false {
            panic("should not be reached");
        }

        value
        "#,
    );

    assert!(value);

    let value: bool = eval(
        r#"
        let value = true;

        match true {
            _ if value && false => panic("should not be reached"),
            _ => (),
        }

        value
        "#,
    );

    assert!(value);
}

/// A constant is evaluated into a `ConstValue`, which is a recursive
/// structure, so how deeply a constant nests is still bounded rather than being
/// allowed to overflow the native stack.
#[test]
fn deeply_nested_constants_are_bounded() {
    let cases = [
        format!("const A = {}1{}; A", "(".repeat(5000), ")".repeat(5000)),
        format!("const A = {}1{}; A", "[".repeat(5000), "]".repeat(5000)),
        format!("const A = {}1{}; A", "(".repeat(5000), ",)".repeat(5000)),
        format!("const A = {}1{}; A", "#{a: ".repeat(5000), "}".repeat(5000)),
        format!(
            "const fn f() {{ {}1{} }} const A = f(); A",
            "(".repeat(5000),
            ")".repeat(5000)
        ),
    ];

    for source in cases {
        let error = build(&source, &options()).expect_err("Should exceed the limit");

        assert!(
            matches!(error, ErrorKind::MaxConstDepth { .. }),
            "{error:?}"
        );
    }
}

/// How deeply a constant is allowed to nest is the `max-const-depth` option,
/// which `max-depth` can lower but not raise.
#[test]
fn const_depth_is_configurable() {
    let source = format!("const A = {}1{}; A", "(".repeat(64), ")".repeat(64));

    let mut lowered = options();
    lowered.max_const_depth = 8;

    let error = build(&source, &lowered).expect_err("Should exceed the limit");
    assert!(
        matches!(error, ErrorKind::MaxConstDepth { max: 8 }),
        "{error:?}"
    );

    // `max-depth` lowers it as well, since nothing downstream can walk nesting
    // it does not accept.
    let mut lowered = options();
    lowered.max_depth = 4;

    let error = build(&source, &lowered).expect_err("Should exceed the limit");
    assert!(
        matches!(
            error,
            ErrorKind::MaxConstDepth { max: 4 } | ErrorKind::MaxNesting { max: 4 }
        ),
        "{error:?}"
    );

    build(&source, &options()).expect("Source should compile under the default");
}

/// Imports are followed by recursing, since an import may point at another
/// import, so how many may be traversed is bounded by `max-import-depth`.
#[test]
fn import_depth_is_configurable() {
    let source = r#"
    mod a { pub const X = 1; }
    use a::X as Y;
    use Y as Z;
    use Z as W;
    W
    "#;

    let mut lowered = options();
    lowered.max_import_depth = 1;

    let error = build(source, &lowered).expect_err("Should exceed the limit");
    assert!(
        matches!(error, ErrorKind::ImportRecursionLimit { .. }),
        "{error:?}"
    );

    build(source, &options()).expect("Source should compile under the default");
}

/// A constant nested well within the limit still compiles.
#[test]
fn moderately_nested_constants_compile() {
    let source = format!("const A = {}1{}; A", "(".repeat(64), ")".repeat(64));

    let value: i64 = eval(&source);
    assert_eq!(value, 1);
}

/// Nesting hidden inside the output of a macro is subject to the same limit as
/// nesting written directly, since the expansion is re-parsed.
#[test]
fn macro_expansion_is_bounded() {
    let mut options = options();
    options.max_depth = 64;

    let direct = format!("let a = {}1{}; a", "(".repeat(200), ")".repeat(200));

    let error = build(&direct, &options).expect_err("Should exceed the limit");
    assert!(
        matches!(error, ErrorKind::MaxNesting { max: 64 }),
        "{error:?}"
    );

    let expanded = format!("let a = dbg!({}1{}); a", "(".repeat(200), ")".repeat(200));

    let error = build(&expanded, &options).expect_err("Should exceed the limit");
    assert!(
        matches!(error, ErrorKind::MaxNesting { max: 64 }),
        "{error:?}"
    );
}

/// A constant which does not terminate is stopped by the budget the interior
/// virtual machine runs under, which is the `const-budget` option.
#[test]
fn non_terminating_constants_are_bounded() {
    let source = r#"const A = { let i = 0; while true { i += 1; } i }; A"#;

    let error = build(source, &options()).expect_err("Should exceed the budget");

    assert!(
        matches!(error, ErrorKind::ConstBudgetExceeded { .. }),
        "{error:?}"
    );

    let mut lowered = options();
    lowered.const_budget = 128;

    let error = build(source, &lowered).expect_err("Should exceed the budget");

    assert!(
        matches!(error, ErrorKind::ConstBudgetExceeded { budget: 128 }),
        "{error:?}"
    );
}

/// A cycle between constants is reported rather than being followed.
#[test]
fn constant_cycles_are_detected() {
    let source = r#"const A = { B }; const B = { A }; A"#;

    build(source, &options()).expect_err("Should detect the cycle");
}

/// Every arm of a `match` writes where the match writes, so that has to be an
/// address of its own.
///
/// An arm whose body is a variable hands the address of that variable over
/// rather than writing anything, so the arms which followed wrote into whichever
/// variable the first one named - which overwrote it whenever a later arm was
/// the one taken.
#[test]
fn match_arms_do_not_write_into_a_variable() {
    let value: i64 = rune! {
        let v = 5;
        let m = match 1 { 0 => v, _ => -4 };
        v
    };

    assert_eq!(value, 5);

    let value: (i64, i64) = rune! {
        let v = 5;
        let m = match v { 0 => v, _ => -4 };
        (m, v)
    };

    assert_eq!(value, (-4, 5));

    // The arm which is a variable is still what the match evaluates to when it
    // is the one taken.
    let value: (i64, i64) = rune! {
        let v = 5;
        let m = match v { 5 => v, _ => -4 };
        (m, v)
    };

    assert_eq!(value, (5, 5));

    // The same holds when the variable is not the first arm.
    let value: (i64, i64) = rune! {
        let v = 5;
        let m = match 1 { 0 => -4, _ => v };
        (m, v)
    };

    assert_eq!(value, (5, 5));

    // A match which is used in the middle of an expression sees the same
    // variable it named.
    let value: i64 = rune! {
        let v = 5;
        (match v { 0 => v, _ => -4 }) + v
    };

    assert_eq!(value, 1);

    // Nesting, where each match names the variable the other writes.
    let value: (i64, i64) = rune! {
        let a = 1;
        let b = 2;
        let x = match 9 { 0 => a, _ => match 9 { 0 => b, _ => 7 } };
        (a, b)
    };

    assert_eq!(value, (1, 2));
}

/// Both sides of `&&` and `||` write where the expression writes, so that has
/// to be an address of its own.
///
/// A side which is a variable hands the address of that variable over rather
/// than writing anything, so the other side wrote into that variable whenever
/// the expression did not short-circuit past it.
#[test]
fn conditional_operands_do_not_write_into_a_variable() {
    let value: (bool, bool) = rune! {
        let out = { let v = true; let m = v && false; (m, v) };
        out
    };

    assert_eq!(value, (false, true));

    let value: (bool, bool) = rune! {
        let out = { let v = false; let m = v || true; (m, v) };
        out
    };

    assert_eq!(value, (true, false));

    // Short-circuiting still hands back what the left-hand side was.
    let value: (bool, bool) = rune! {
        let out = { let v = false; let m = v && true; (m, v) };
        out
    };

    assert_eq!(value, (false, false));

    let value: (bool, bool) = rune! {
        let out = { let v = true; let m = v || false; (m, v) };
        out
    };

    assert_eq!(value, (true, true));

    // The same holds when the variable is on the right.
    let value: (bool, bool, bool) = rune! {
        let out = { let v = true; let w = false; let m = w || v; (m, v, w) };
        out
    };

    assert_eq!(value, (true, true, false));

    // And when the operands are chained.
    let value: (bool, bool) = rune! {
        let out = { let v = true; let m = v && false && true; (m, v) };
        out
    };

    assert_eq!(value, (false, true));
}

/// An operand which is a variable is assembled by handing the address of that
/// variable over rather than by writing anything, and the operands of a
/// construct are read once every one of them has been assembled.
///
/// So an operand which came earlier saw whatever a later one wrote into the
/// variable it named, rather than what was in it when it was its turn.
#[test]
fn an_operand_is_read_when_it_is_its_turn() {
    // Binary operators.
    let value: i64 = rune! {
        let v = 0;
        v + { v = 1; 9 }
    };

    assert_eq!(value, 9);

    let value: i64 = rune! {
        let v = 0;
        v * { v = 3; 2 }
    };

    assert_eq!(value, 0);

    let value: bool = rune! {
        let v = 0;
        v == { v = 1; 0 }
    };

    assert!(value);

    // The operand which comes second still sees what came before it. The block
    // is parenthesised because one written where a statement is expected is a
    // statement of its own, so what follows it would begin the next one.
    let value: i64 = rune! {
        let v = 0;
        ({ v = 1; 9 }) + v
    };

    assert_eq!(value, 10);

    // Tuples, which have an instruction of their own for the small sizes.
    let value: (i64, i64) = rune! {
        let v = 0;
        let t = (v, { v = 1; 9 });
        t
    };

    assert_eq!(value, (0, 9));

    let value: (i64, i64, i64, i64) = rune! {
        let v = 0;
        let t = (v, v, v, { v = 1; 9 });
        t
    };

    assert_eq!(value, (0, 0, 0, 9));

    // Ranges.
    let value: i64 = rune! {
        let v = 0;
        let r = v..{ v = 1; 9 };
        r.start
    };

    assert_eq!(value, 0);

    // What is not written to is still not copied, so the shapes which are worth
    // leaving alone go on producing the same answers.
    let value: i64 = rune! {
        let a = 2;
        let b = 3;
        a * b + a
    };

    assert_eq!(value, 8);
}

/// An expression written as a block is a statement of its own when it is
/// written where a statement is expected, so what follows it begins the next
/// statement rather than continuing it.
///
/// It was parsed as the beginning of an expression instead, so an operator
/// which follows it was taken as a binary one over the value of the block.
/// `if a { return 1; } -1` subtracted from the value of the `if`, which is the
/// unit it produces without an `else`, rather than ending with `-1`.
#[test]
fn a_block_expression_ends_the_statement_it_is_written_as() {
    let value: i64 = rune! {
        fn f() {
            if false {
                return 1;
            }

            -1
        }

        f()
    };

    assert_eq!(value, -1);

    // The same for every other expression written as a block.
    let value: i64 = rune!(
        fn f() {
            match 1 {
                _ => (),
            }
            -1
        },
        f()
    );
    assert_eq!(value, -1);

    let value: i64 = rune!(
        fn f() {
            loop {
                break;
            }
            -1
        },
        f()
    );
    assert_eq!(value, -1);

    let value: i64 = rune!(
        fn f() {
            while false {}
            -1
        },
        f()
    );
    assert_eq!(value, -1);

    let value: i64 = rune!(
        fn f() {
            for _ in 0..1 {}
            -1
        },
        f()
    );
    assert_eq!(value, -1);

    let value: i64 = rune!(
        fn f() {
            {}
            -1
        },
        f()
    );
    assert_eq!(value, -1);

    // Where an expression is expected the operator still binds, since there is
    // no statement for the block to be.
    let value: i64 = rune! {
        let v = if true { 1 } else { 2 } - 1;
        v
    };

    assert_eq!(value, 0);

    let value: i64 = rune!(let v = { 5 } * 2; v);
    assert_eq!(value, 10);

    let value: i64 = rune! {
        fn f() { if true { 5 } else { 6 } }
        f() - 1
    };

    assert_eq!(value, 4);

    // And a block which is the value of a statement is still the value of it.
    let value: i64 = rune! {
        fn f() {
            if true { 7 } else { 8 }
        }

        f()
    };

    assert_eq!(value, 7);
}
