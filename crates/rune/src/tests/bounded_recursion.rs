//! Tests that compilation is not bounded by the size of the call stack.
//!
//! Chained expressions - `a + b + c + ..`, `a.b().c()`, `a[0][1]` - grow with
//! the *width* of otherwise flat source code, so they cannot be capped by any
//! limit which is reasonable to impose on lexical nesting. They are assembled
//! over a heap allocated work stack bounded by `max-depth` instead.
//!
//! Lexical nesting is parsed over an explicit stack too, and the parser is
//! configured from the same `max-depth` option. The bound is still enforced so
//! that nothing downstream is handed nesting it cannot walk, and so that a
//! constant, whose value is a recursive structure, cannot nest arbitrarily.
//! Exceeding it is reported as a diagnostic rather than as a stack overflow.
//!
//! The syntax tree parser macros use over their own input is the exception. It
//! recurses, and so does everything which walks what it produced, so it has a
//! much smaller bound of its own which chains count towards as well.
//!
//! The formatter walks the same tree over its own explicit stack, so anything
//! which parses can also be formatted. That matters more there than elsewhere,
//! since it is handed arbitrary text which does not have to compile.

prelude!();

use rust_alloc::string::String;

use crate::diagnostics::{Diagnostic, FatalDiagnosticKind};
use crate::{SourceId, Unit, VmError};

/// How deep the chains built below are.
///
/// The recursive assembler this replaced overflowed at a few hundred, so this is
/// well past what used to abort the process while staying quick to compile.
const DEEP: usize = 5000;

fn options(max_depth: usize) -> Options {
    let mut options = Options::default();
    options.script(true);
    options.max_depth = max_depth;
    // What is under test here is that the compiler walks nesting without
    // recursing, which is a separate question from how deeply items are allowed
    // to nest, so the bound on that is lifted rather than worked around.
    options.max_item_depth = usize::MAX;
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

/// Build and run `source` with generous limits.
fn eval_deep<T>(source: &str) -> T
where
    T: FromValue,
{
    let context = Context::with_default_modules().expect("Failed to build context");
    let unit = build(source, &options(usize::MAX)).expect("Source should compile");

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

/// `1 + 1 + 1 + ..` - the left-hand side of each operator is another binary
/// expression, so this recursed once per term in the assembler this replaced.
#[test]
fn deep_binary_chain() {
    let mut source = String::from("let a = 1");

    for _ in 1..DEEP {
        source.push_str(" + 1");
    }

    source.push_str("; a");

    let value: i64 = eval_deep(&source);
    assert_eq!(value, DEEP as i64);
}

/// `true && true && ..` - conditional operators short circuit, so each term
/// also emits a jump past the remainder of the chain.
#[test]
fn deep_conditional_chain() {
    let mut source = String::from("let a = true");

    for _ in 1..DEEP {
        source.push_str(" && true");
    }

    source.push_str("; a");

    let value: bool = eval_deep(&source);
    assert!(value);
}

/// `0.max(1).max(1)..` - the target of each call is the previous call.
#[test]
fn deep_method_chain() {
    let mut source = String::from("let a = 0");

    for _ in 0..DEEP {
        source.push_str(".max(1)");
    }

    source.push_str("; a");

    let value: i64 = eval_deep(&source);
    assert_eq!(value, 1);
}

/// Prefix operators are a chain as far as the assembler is concerned, but the
/// parser treats them as lexical nesting, so the same `max-depth` option bounds
/// them through the parser rather than through the work stack.
#[test]
fn deep_unary_chain() {
    let source = format!("let a = {}1; a", "--".repeat(20));

    let value: i64 = eval_deep(&source);
    assert_eq!(value, 1);

    let source = format!("let a = {}1; a", "--".repeat(200));

    let error = build(&source, &options(64)).expect_err("Should exceed the limit");
    assert!(
        matches!(error, ErrorKind::MaxNesting { max: 64 }),
        "{error:?}"
    );
}

/// Field accesses, indexing and `?` chain the same way, but need a receiver
/// which is awkward to produce a value for, so they are only compiled.
#[test]
fn deep_chains_compile() {
    let cases = [
        ("let b = #{f: 1}; let a = b", ".f"),
        ("let b = [0]; let a = b", "[0]"),
    ];

    for (head, link) in cases {
        let mut source = String::from(head);

        for _ in 0..DEEP {
            source.push_str(link);
        }

        source.push_str("; a");

        build(&source, &options(usize::MAX)).expect("Source should compile");
    }
}

/// The work stack is bounded, so a chain past the limit is a diagnostic rather
/// than an abort.
#[test]
fn max_depth_is_enforced() {
    let mut source = String::from("let a = 1");

    for _ in 1..DEEP {
        source.push_str(" + 1");
    }

    source.push_str("; a");

    let error = build(&source, &options(100)).expect_err("Should exceed the depth limit");
    assert!(
        matches!(error, ErrorKind::MaxDepth { max: 100 }),
        "{error:?}"
    );

    // The same source compiles once the limit accommodates it.
    build(&source, &options(DEEP)).expect("Source should compile");
}

/// Lexical nesting is bounded by the same `max-depth` option, reported through
/// its own diagnostic since the parser is what enforces it.
#[test]
fn nesting_is_bounded_by_max_depth() {
    let source = format!("let a = {}1{};", "(".repeat(40), ")".repeat(40));

    let error = build(&source, &options(20)).expect_err("Should exceed the limit");
    assert!(
        matches!(error, ErrorKind::MaxNesting { max: 20 }),
        "{error:?}"
    );

    build(&source, &options(usize::MAX)).expect("Source should compile");
}

/// Parse `source` with the given depth limit, without lowering it.
///
/// Used to test the parser in isolation: a construct which the parser handles
/// over an explicit stack can be parsed far deeper than a full compilation of
/// the same source would get, so it cannot be exercised end to end.
fn parse(source: &str, max_depth: usize) -> Result<(), ErrorKind> {
    let result = crate::grammar::text(SourceId::EMPTY, source)
        .max_nesting(max_depth)
        .root();

    match result {
        Ok(..) => Ok(()),
        Err(error) => Err(error.into_kind()),
    }
}

/// Patterns are parsed over an explicit stack, so the parser does not recurse
/// over how deeply they nest.
///
/// The limit is still enforced, so that the passes downstream of the parser are
/// never handed nesting they cannot walk.
#[test]
fn deep_patterns_parse() {
    let source = format!("let {}a{} = 1;", "(".repeat(10000), ")".repeat(10000));

    parse(&source, usize::MAX).expect("Deeply nested pattern should parse");

    let error = parse(&source, 64).expect_err("Should exceed the limit");
    assert!(
        matches!(error, ErrorKind::MaxNesting { max: 64 }),
        "{error:?}"
    );
}

/// Expressions, blocks, statements and the items which contain blocks are all
/// parsed over an explicit stack, so the parser itself imposes no bound on how
/// deeply they nest.
///
/// The limit is still enforced, so that the passes downstream of the parser are
/// never handed nesting they cannot walk.
#[test]
fn deep_expressions_parse() {
    let cases = [
        format!("let a = {}1{};", "(".repeat(5000), ")".repeat(5000)),
        format!("let a = {}1;", "-".repeat(5000)),
        format!("let a = {}1{};", "[".repeat(5000), "]".repeat(5000)),
        format!("{}{}", "{".repeat(5000), "}".repeat(5000)),
        format!("{}{}", "if true {".repeat(2000), "}".repeat(2000)),
        format!("{}{}", "mod a {".repeat(2000), "}".repeat(2000)),
        format!("{}{}", "fn a() {".repeat(2000), "}".repeat(2000)),
        format!(
            "let a = {}1{};",
            "match true { _ => ".repeat(1000),
            "}".repeat(1000)
        ),
    ];

    for source in cases {
        parse(&source, usize::MAX).expect("Deeply nested source should parse");

        let error = parse(&source, 8).expect_err("Should exceed the limit");
        assert!(
            matches!(error, ErrorKind::MaxNesting { max: 8 }),
            "{error:?}"
        );
    }
}

/// Paths nest through generic arguments, which no limit applies to, so the
/// parser has to walk them iteratively to avoid overflowing on them.
#[test]
fn deep_paths_parse() {
    let source = format!("let a = a{}b{};", "::<".repeat(5000), ">".repeat(5000));
    parse(&source, usize::MAX).expect("Deeply nested path should parse");
}

/// Constants are compiled by the ordinary assembler and run in a virtual
/// machine, so a constant which is flat in the source but deeply nested in the
/// tree goes over the same work stack as everything else rather than over the
/// native stack.
#[test]
fn deep_const_chain() {
    let source = format!("const A = {}1; A", "1 + ".repeat(20000));

    let value: i64 = eval_deep(&source);
    assert_eq!(value, 20001);
}

/// A constant is stored as a `ConstValue`, which is built, walked, cloned and
/// dropped by recursing over it.
///
/// The value a constant evaluates to is produced by running it, so a constant
/// function which nests a value in a loop can produce one of any depth, which
/// the nesting written in the source says nothing about. Exceeding the bound
/// is a diagnostic rather than a stack overflow.
#[test]
fn deep_const_value_is_bounded() {
    let source_for = |depth: usize| {
        format!(
            "const fn nest() {{ \
                 let a = []; let i = 0; \
                 while i < {depth} {{ a = [a]; i += 1; }} \
                 a \
             }} \
             const A = nest(); A"
        )
    };

    // Well within the bound, so it evaluates.
    let source = source_for(8);
    build(&source, &options(usize::MAX)).expect("Source should compile");

    // Past it, so it is reported.
    let source = source_for(4000);
    let error = build(&source, &options(usize::MAX)).expect_err("Should exceed the limit");
    assert!(
        error
            .to_string()
            .contains("nested too deeply to be a constant"),
        "{error:?}"
    );
}

/// A constant function which calls itself is assembled once and recurses in the
/// virtual machine, so the depth it reaches costs call frames rather than
/// compiler stack frames.
#[test]
fn const_fn_recursion_is_bounded() {
    // Deeper than anything the compiler could walk over its own stack.
    let source = "const fn count(n) { if n == 0 { 0 } else { count(n - 1) + 1 } } \
                  const A = count(2000); A";
    build(source, &options(usize::MAX)).expect("Source should compile");

    // Recursion which never bottoms out spends the budget instead of the stack.
    let source = "const fn inf(n) { inf(n + 1) } const A = inf(0); A";
    let error = build(source, &options(usize::MAX)).expect_err("Should exceed the budget");
    assert!(
        matches!(error, ErrorKind::ConstBudgetExceeded { .. }),
        "{error:?}"
    );
}

/// Lowering and assembly walk nesting over an explicit stack, so deeply nested
/// source compiles and runs when `max-depth` accommodates it.
#[test]
fn deep_nesting_compiles() {
    let cases = [
        format!("let a = {}1{}; a", "(".repeat(DEEP), ")".repeat(DEEP)),
        format!("let a = {}1; a", "-".repeat(DEEP)),
        format!("let a = {}1{}; a", "[".repeat(DEEP), "]".repeat(DEEP)),
        format!("{}{}", "{".repeat(DEEP), "}".repeat(DEEP)),
        format!(
            "let a = {}1{}; a",
            "if true {".repeat(DEEP),
            "}".repeat(DEEP)
        ),
        format!(
            "let a = {}1{}; a",
            "match true { _ => ".repeat(DEEP),
            "}".repeat(DEEP)
        ),
        format!(
            "let a = {}1{}; a",
            "loop { break ".repeat(DEEP),
            "}".repeat(DEEP)
        ),
        format!("let {}a{} = 1; a", "(".repeat(DEEP), ")".repeat(DEEP)),
        format!(
            "fn f(v) {{ v }} let a = {}1{}; a",
            "f(".repeat(DEEP),
            ")".repeat(DEEP)
        ),
    ];

    for source in cases {
        if let Err(error) = build(&source, &options(usize::MAX)) {
            panic!("Deeply nested source should compile: {error:?}");
        }
    }
}

/// Every level of lexical nesting pushes a component onto the path of the item
/// being built, and allocating an item hashes its whole path and stores a copy
/// of it, so nesting `n` deep costs `O(n^2)` in both time and memory.
///
/// Nothing else bounds how long a path may get - `max-depth` bounds the work
/// stacks, and `MAX_ITEM_RECURSION` in `query` bounds how deeply items depend on
/// each other, which is a different thing - so `max-item-depth` does.
#[test]
fn item_depth_is_bounded() {
    let mut options = options(usize::MAX);
    options.max_item_depth = 64;

    // A block, and so everything which is written with one, costs a level.
    for source in [
        format!("{}1{}", "{".repeat(128), "}".repeat(128)),
        format!("let a = {}1{}; a", "if true {".repeat(128), "}".repeat(128)),
        format!(
            "let a = {}1{}; a",
            "loop { break ".repeat(128),
            "}".repeat(128)
        ),
        format!("{}{}", "for _ in 0..1 {".repeat(128), "}".repeat(128)),
        format!(
            "{} fn f() {{ 1 }} {}",
            "mod m {".repeat(128),
            "}".repeat(128)
        ),
    ] {
        let error = build(&source, &options).expect_err("Should exceed the limit");

        assert!(
            matches!(error, ErrorKind::MaxItemDepth { max: 64 }),
            "{error:?}"
        );
    }

    // A closure costs two levels, one for itself and one for its body.
    let source = format!("let a = {}1{}; a", "|| {".repeat(64), "}".repeat(64));
    let error = build(&source, &options).expect_err("Should exceed the limit");
    assert!(
        matches!(error, ErrorKind::MaxItemDepth { max: 64 }),
        "{error:?}"
    );

    // Well within the bound, so it compiles.
    let source = format!("{}1{}", "{".repeat(32), "}".repeat(32));
    build(&source, &options).expect("Source should compile");

    // Width is not depth. A chain of siblings pushes and pops one level at a
    // time, so any number of them is fine.
    let source = "{ 1 } ".repeat(4096);
    build(&source, &options).expect("Source should compile");
}

/// The default bounds it too, so a host which never looked at the options is
/// not the one paying for the quadratic.
#[test]
fn item_depth_is_bounded_by_default() {
    let mut options = Options::default();
    options.script(true);

    let source = format!("{}1{}", "{".repeat(4096), "}".repeat(4096));
    let error = build(&source, &options).expect_err("Should exceed the limit");

    assert!(
        matches!(error, ErrorKind::MaxItemDepth { max: 128 }),
        "{error:?}"
    );
}

/// Items nest through `mod`, which does not go through expression parsing.
#[test]
fn max_depth_applies_to_items() {
    let mut source = String::new();

    for i in 0..100 {
        source.push_str(&format!("mod m{i} {{"));
    }

    source.push_str(&"}".repeat(100));

    let error = build(&source, &options(64)).expect_err("Should exceed the limit");
    assert!(
        matches!(error, ErrorKind::MaxNesting { max: 64 }),
        "{error:?}"
    );
}

/// Patterns nest independently of expressions.
#[test]
fn max_depth_applies_to_patterns() {
    let source = format!(
        "let {}a{} = {}1{};",
        "(".repeat(100),
        ",)".repeat(100),
        "(".repeat(100),
        ",)".repeat(100)
    );

    let error = build(&source, &options(64)).expect_err("Should exceed the limit");
    assert!(
        matches!(error, ErrorKind::MaxNesting { max: 64 }),
        "{error:?}"
    );
}

/// Items nest through `mod` and `fn`, each contributing a component to the item
/// path, so nested items build an equally deep tree in `Names`. Dismantling that
/// tree is iterative, so it does not overflow either.
#[test]
fn deep_items_compile() {
    let cases = [
        format!("{}{}", "mod m {".repeat(DEEP), "}".repeat(DEEP)),
        format!("{}1{}", "fn f() {".repeat(DEEP), "}".repeat(DEEP)),
    ];

    for source in cases {
        if let Err(error) = build(&source, &options(usize::MAX)) {
            panic!("Deeply nested items should compile: {error:?}");
        }
    }
}

/// The default of the `max-ast-depth` option, which is what bounds the syntax
/// tree parser unless a macro is handed something else.
const MAX_AST_DEPTH: usize = Options::DEFAULT.max_ast_depth;

/// Parse `source` as `T` with the syntax tree parser.
///
/// This is the parser macros use over their own input, which is the only thing
/// which still reaches it. It builds a tree which it, and everything which
/// consumes what it produced, walks by recursing, so its bound is much smaller
/// than `max-depth`.
fn parse_ast<T>(source: &str, max_depth: usize) -> Result<(), ErrorKind>
where
    T: parse::Parse,
{
    let mut p = parse::Parser::new(source, SourceId::EMPTY, false).with_max_depth(max_depth);

    match p.parse_all::<T>() {
        Ok(..) => Ok(()),
        Err(error) => Err(error.into_kind()),
    }
}

/// Expressions, patterns and items all nest through the syntax tree parser, and
/// exceeding its bound is a diagnostic rather than a stack overflow.
#[test]
fn ast_nesting_is_bounded() {
    let max = 16;
    let deep = max * 8;

    let cases = [
        format!("{}1{}", "(".repeat(deep), ")".repeat(deep)),
        format!("{}1", "-".repeat(deep)),
        format!("{}1{}", "[".repeat(deep), "]".repeat(deep)),
        format!("{}1{}", "if true {".repeat(deep), "}".repeat(deep)),
    ];

    for source in cases {
        let error = parse_ast::<ast::Expr>(&source, max).expect_err("Should exceed the limit");
        assert!(
            matches!(error, ErrorKind::MaxAstDepth { max: m } if m == max),
            "{error:?}"
        );
    }

    let source = format!("{}a{}", "(".repeat(deep), ")".repeat(deep));
    let error = parse_ast::<ast::Pat>(&source, max).expect_err("Should exceed the limit");
    assert!(
        matches!(error, ErrorKind::MaxAstDepth { max: m } if m == max),
        "{error:?}"
    );

    let source = format!("{}{}", "mod m {".repeat(deep), "}".repeat(deep));
    let error = parse_ast::<ast::Item>(&source, max).expect_err("Should exceed the limit");
    assert!(
        matches!(error, ErrorKind::MaxAstDepth { max: m } if m == max),
        "{error:?}"
    );

    // A parser which was not configured is bounded by the option's default.
    let source = format!("{}1{}", "(".repeat(deep), ")".repeat(deep));
    let error =
        parse_ast::<ast::Expr>(&source, MAX_AST_DEPTH).expect_err("Should exceed the limit");
    assert!(
        matches!(error, ErrorKind::MaxAstDepth { max } if max == MAX_AST_DEPTH),
        "{error:?}"
    );
}

/// Chains cost the syntax tree parser no frames, since it parses them over a
/// loop, but each link is another level of the tree it leaves behind - and that
/// tree is walked by recursing. So they are bounded by the same limit as
/// nesting, unlike everywhere else, where a chain is bounded by `max-depth`.
#[test]
fn ast_chains_are_bounded() {
    let max = 16;
    let deep = max * 8;

    let mut binary = String::from("1");
    let mut chain = String::from("0");
    let mut short = String::from("1");

    for _ in 1..deep {
        binary.push_str(" + 1");
        chain.push_str(".max(1)");
    }

    for _ in 1..max / 2 {
        short.push_str(" + 1");
    }

    for source in [binary, chain] {
        let error = parse_ast::<ast::Expr>(&source, max).expect_err("Should exceed the limit");
        assert!(
            matches!(error, ErrorKind::MaxAstDepth { max: m } if m == max),
            "{error:?}"
        );
    }

    parse_ast::<ast::Expr>(&short, max).expect("Chain within the limit should parse");

    // Sibling expressions do not accumulate - each one is released along with
    // the expression it belongs to.
    let source = format!("{{ {} }}", format!("let a = {short};").repeat(64));
    parse_ast::<ast::Block>(&source, max).expect("Sibling expressions should parse");
}

/// `println!` parses its arguments with the syntax tree parser and evaluates its
/// format argument as a constant, so it is a way into both from ordinary source.
///
/// Input which is too deep for the parser is a diagnostic rather than an
/// overflow, whether it got that way by nesting or by chaining, and how deep is
/// too deep comes from the `max-ast-depth` option the macro is expanded under.
#[test]
fn macro_input_is_bounded() {
    let default = options(usize::MAX);

    let mut lowered = default.clone();
    lowered.max_ast_depth = 16;

    // Deeper than the lowered option, but well within its default.
    let deep = lowered.max_ast_depth * 2;

    let mut arg = String::from("\"x\"");

    for _ in 1..deep {
        arg.push_str(" + \"x\"");
    }

    let cases = [
        format!("println!({}\"x\"{});", "(".repeat(deep), ")".repeat(deep)),
        format!("println!({arg});"),
    ];

    for source in cases {
        let error = build(&source, &lowered).expect_err("Should exceed the limit");
        assert!(
            matches!(error, ErrorKind::MaxAstDepth { max } if max == lowered.max_ast_depth),
            "{error:?}"
        );

        // The same source compiles under the default.
        build(&source, &default).expect("Source should compile");
    }
}

/// `MacroContext::eval` evaluates an expression a macro has parsed out of its
/// own input. The expression is re-parsed by the grammar and lowered by the
/// same passes as everything else, so what it evaluates is walked over their
/// work stacks rather than over the native stack.
#[test]
fn macro_eval_is_bounded() {
    let mut arg = String::from("\"x\"");

    for _ in 1..MAX_AST_DEPTH / 2 {
        arg.push_str(" + \"x\"");
    }

    let source = format!("println!({arg});");
    build(&source, &options(usize::MAX)).expect("Source should compile");

    // `max-depth` bounds the work stacks the evaluated expression is walked
    // over, so it is reported before the evaluation gets that far.
    let error = build(&source, &options(4)).expect_err("Should exceed the depth limit");
    assert!(
        matches!(
            error,
            ErrorKind::MaxNesting { max: 4 } | ErrorKind::MaxDepth { max: 4 }
        ),
        "{error:?}"
    );
}

/// Imports nest through groups, which the grammar walks over an explicit stack.
///
/// Nothing downstream recurses over them - the worker processes a group over a
/// queue - so no limit applies, and the only bound is memory.
#[test]
fn deep_imports_parse() {
    let source = format!("use a::{}c{};", "{b::".repeat(DEEP), "}".repeat(DEEP));
    parse(&source, usize::MAX).expect("Deeply nested import should parse");
}

/// The same nesting, resolved end to end rather than only parsed.
#[test]
fn deep_imports_compile() {
    // Nesting the modules the import refers to is what makes this expensive, so
    // it is shallower than the parse above.
    let depth = 500;

    let mut source = String::from("mod m0 {");

    for i in 1..depth {
        source.push_str(&format!(" pub mod m{i} {{"));
    }

    source.push_str(" pub const X = 1;");
    source.push_str(&"}".repeat(depth));

    source.push_str("\nuse m0::");

    for i in 1..depth {
        source.push_str(&format!("{{m{i}::"));
    }

    source.push_str("{X}");
    source.push_str(&"}".repeat(depth - 1));
    source.push_str(";\nX");

    let value: i64 = eval_deep(&source);
    assert_eq!(value, 1);
}

/// Format `source`, returning the kind of the error if it could not be
/// formatted.
///
/// The formatter is handed arbitrary text which does not have to compile, so
/// what it does with nesting matters more than it does elsewhere.
fn format(source: &str, options: &Options) -> Result<(), ErrorKind> {
    let mut diagnostics = Diagnostics::new();

    match crate::fmt::layout_source_with(source, SourceId::EMPTY, options, &mut diagnostics) {
        Ok(..) => Ok(()),
        Err(error) => Err(error.into_kind()),
    }
}

/// Paths nest through generic arguments, which no limit applies to, so the
/// formatter walks them over an explicit stack.
///
/// The closing `>` of each level is spaced apart, since `>>` lexes as a shift
/// and the nesting is what is being tested here.
#[test]
fn deep_paths_format() {
    let source = format!("let a = a{}b{};", "::<".repeat(DEEP), " >".repeat(DEEP));
    format(&source, &options(usize::MAX)).expect("Deeply nested path should format");
}

/// Imports nest through groups, which no limit applies to either.
#[test]
fn deep_imports_format() {
    let source = format!("use a::{}c{};", "{b::".repeat(DEEP), "}".repeat(DEEP));
    format(&source, &options(usize::MAX)).expect("Deeply nested import should format");
}

/// Expressions, patterns, items and the blocks which contain them nest through
/// each other, and the formatter walks all of it over an explicit stack.
///
/// Anything the parser accepts can therefore be formatted, so the formatter
/// needs no bound of its own beyond the one the parse was performed under.
#[test]
fn deep_nesting_formats() {
    let cases = [
        format!("let a = {}1{};", "(".repeat(DEEP), ")".repeat(DEEP)),
        format!("let a = {}1;", "-".repeat(DEEP)),
        format!("let a = {}1{};", "[".repeat(DEEP), "]".repeat(DEEP)),
        format!("{}{}", "{".repeat(DEEP), "}".repeat(DEEP)),
        format!("{}{}", "if true {".repeat(DEEP), "}".repeat(DEEP)),
        format!("{}{}", "mod a {".repeat(DEEP), "}".repeat(DEEP)),
        format!("{}{}", "fn a() {".repeat(DEEP), "}".repeat(DEEP)),
        format!("{}{}", "while true {".repeat(DEEP), "}".repeat(DEEP)),
        format!("{}{}", "for a in b {".repeat(DEEP), "}".repeat(DEEP)),
        format!("{}{}", "loop {".repeat(DEEP), "}".repeat(DEEP)),
        format!(
            "let a = {}1{};",
            "match true { _ => ".repeat(DEEP),
            "}".repeat(DEEP)
        ),
        format!("let a = {}1{};", "#{a: ".repeat(DEEP), "}".repeat(DEEP)),
        format!("let a = {}1{};", "(".repeat(DEEP), ",)".repeat(DEEP)),
        format!("let {}a{} = 1;", "(".repeat(DEEP), ")".repeat(DEEP)),
        format!("let {}a{} = 1;", "[".repeat(DEEP), "]".repeat(DEEP)),
        format!("let a = {}1;", "|| ".repeat(DEEP)),
        format!("let a = {}1{};", "f(".repeat(DEEP), ")".repeat(DEEP)),
        format!("let a = 1{};", ".f".repeat(DEEP)),
        format!("let a = b{};", "[0]".repeat(DEEP)),
        format!("let a = {}1{};", "1..(".repeat(DEEP), ")".repeat(DEEP)),
        format!("let a = 1{};", " + 1".repeat(DEEP)),
    ];

    for source in cases {
        if let Err(error) = format(&source, &options(usize::MAX)) {
            panic!("Deeply nested source should format: {error:?}");
        }
    }
}

/// `a + (a + (a + ..))` nests through the operands of binary operators, which
/// the grammar parses over its work stack rather than by recursing.
///
/// Precedence climbing still recurses, but only over the precedences
/// themselves, so it is bounded by a constant rather than by the source.
#[test]
fn deep_binary_nesting_parses() {
    let source = format!("let a = {}1{};", "1 + (".repeat(DEEP), ")".repeat(DEEP));
    parse(&source, usize::MAX).expect("Deeply nested operands should parse");

    let error = parse(&source, 8).expect_err("Should exceed the limit");
    assert!(
        matches!(error, ErrorKind::MaxNesting { max: 8 }),
        "{error:?}"
    );

    // Climbing to a higher precedence for every operator, repeatedly.
    let mut source = String::from("let a = 1");

    for _ in 0..DEEP {
        source.push_str(" || 1 && 1 == 1 + 1 * 1");
    }

    source.push(';');
    parse(&source, usize::MAX).expect("Precedence climbing should parse");
}

/// The same nesting, compiled and run rather than only parsed.
#[test]
fn deep_binary_nesting_compiles() {
    let depth = 1000;
    let source = format!("let a = {}1{}; a", "1 + (".repeat(depth), ")".repeat(depth));

    let value: i64 = eval_deep(&source);
    assert_eq!(value, depth as i64 + 1);
}

/// Expression shapes which nest through the lowering driver rather than through
/// the call stack.
///
/// Each of these was a cycle in the call graph of one pass or another, so they
/// are exercised at a depth which would abort if any of them recursed.
#[test]
fn deep_shapes_compile() {
    let n = 2000;

    let cases = [
        format!("let a = {}1{};", "#{a: ".repeat(n), "}".repeat(n)),
        format!("let a = {}1{};", "(".repeat(n), ",)".repeat(n)),
        format!("let a = {}1;", "|| ".repeat(n)),
        format!("let a = {}1{};", "1..(".repeat(n), ")".repeat(n)),
        format!("{}{}", "for a in [] {".repeat(n), "}".repeat(n)),
        format!("{}{}", "while false {".repeat(n), "}".repeat(n)),
        format!("let a = {}1{};", "async {".repeat(n), "}".repeat(n)),
        format!("let a = 1; {}1;", "a = ".repeat(n)),
        format!("let a = {}true;", "!".repeat(n)),
        format!("async fn f() {{ let a = 1{}; }}", ".await".repeat(n)),
    ];

    for source in cases {
        if let Err(error) = build(&source, &options(usize::MAX)) {
            panic!("Deeply nested source should compile: {error:?}");
        }
    }
}

/// An expansion - a macro, a template literal or a format specification -
/// produces a tree of its own, which lowering cannot park in the frames of the
/// tree it is walking, so it recurses over how deeply expansions nest.
///
/// `max-macro-depth` is what keeps that from overflowing, so it is small: the
/// stack was exhausted at around 25 levels unoptimised when this was measured.
#[test]
fn nested_expansions_are_bounded() {
    let max = Options::DEFAULT.max_macro_depth;

    let cases = [
        format!(
            "let a = {}1{};",
            "`${".repeat(max * 4),
            "}`".repeat(max * 4)
        ),
        format!(
            "let a = {}1{};",
            "dbg!(".repeat(max * 4),
            ")".repeat(max * 4)
        ),
    ];

    for source in cases {
        let error = build(&source, &options(usize::MAX)).expect_err("Should exceed the limit");
        assert!(
            matches!(error, ErrorKind::MaxMacroRecursion { max: m, .. } if m == max),
            "{error:?}"
        );
    }

    // Nesting within the limit still compiles.
    let source = format!("let a = {}1{}; a", "`${".repeat(4), "}`".repeat(4));
    let value: String = eval_deep(&source);
    assert_eq!(value, "1");
}

/// Build and run `source`, returning what it evaluated to.
///
/// Unlike [`eval_deep`] this hands back a runtime error rather than panicking
/// on one, since bounding a walk over a value is reported that way.
fn run_deep(source: &str) -> Result<Value, VmError> {
    let context = Context::with_default_modules().expect("Failed to build context");
    let unit = build(source, &options(usize::MAX)).expect("Source should compile");

    let runtime = Arc::try_new(context.runtime().expect("Failed to build runtime"))
        .expect("Failed to allocate runtime");
    let unit = Arc::try_new(unit).expect("Failed to allocate unit");

    let mut vm = Vm::new(runtime, unit);
    vm.execute(Hash::EMPTY, ())
        .expect("Failed to execute")
        .complete()
}

/// Every way a script can make one value part of another.
///
/// A type which is missing from here is dropped in place, which costs a frame
/// for every level a script nests it, so each of these has to be taken apart
/// over the worklist instead.
const NESTED: [&str; 43] = [
    "tuple",
    "vec",
    "object",
    "struct",
    "variant",
    "option",
    "result",
    "range",
    "closure",
    "generator",
    "future",
    "push",
    "branch",
    "shared",
    "control-flow",
    "vec-deque",
    "hash-map",
    // An iterator adapter is made of the iterator it wraps, and wrapping one
    // again is all it takes to nest them.
    "iter-once",
    "iter-chain",
    "iter-enumerate",
    "iter-filter",
    "iter-filter-map",
    "iter-flat-map",
    "iter-map",
    "iter-peekable",
    "iter-rev",
    "iter-skip",
    "iter-take",
    // Nesting through what the closure an adapter was handed captures, rather
    // than through the iterator it wraps.
    "iter-capture",
    // An iterator over a collection keeps it alive through a reference rather
    // than through a slot, and wrapping it in a collection again is all it
    // takes to nest them.
    "slice-iter",
    "object-iter",
    "object-keys",
    "object-values",
    "vec-deque-iter",
    "hash-map-iter",
    "generator-iter",
    "option-iter",
    // A set cannot hold a set, but it can hold a vector which is filled in
    // afterwards, so what it walks over nests all the same.
    "hash-set",
    "hash-set-iter",
    "hash-set-difference",
    "hash-set-intersection",
    "hash-set-union",
    // Nesting through what the closure a split was handed captures.
    "string-split",
];

/// How deeply the values built below nest.
///
/// Dropping a chain recursively overflowed somewhere in the thousands, and a
/// test runs on a thread with a much smaller stack than the main one, so this
/// is well past what used to abort the process.
const DEEP_VALUE: usize = 20000;

/// Build a value nested `depth` deep at runtime, which is nesting no limit the
/// compiler applies has any bearing on.
fn nested_value(depth: usize, kind: &str) -> String {
    let (decl, init, step) = match kind {
        "tuple" => ("", "()", "a = (a,);"),
        "vec" => ("", "[]", "a = [a];"),
        "object" => ("", "#{}", "a = #{a: a};"),
        "struct" => ("struct S { a }", "0", "a = S { a };"),
        "variant" => ("enum E { V { a } }", "0", "a = E::V { a };"),
        "option" => ("", "0", "a = Some(a);"),
        "result" => ("", "0", "a = Ok(a);"),
        "range" => ("", "0", "a = a..a;"),
        "closure" => ("", "0", "a = move || a;"),
        "generator" => ("fn g(v) { yield v; }", "0", "a = g(a);"),
        // A future which is never awaited still holds everything the execution
        // it drives was given.
        "future" => ("", "0", "let b = a; a = async { b };"),
        // Nesting which branches, so that the way back out of one value has to
        // be found again before the next one is descended into.
        "branch" => ("", "0", "a = [a, [i]];"),
        // The same value nested twice over, so that descending into it is not
        // what drops it the first time around.
        "shared" => ("", "0", "a = (a, a);"),
        // Nesting through a native function which mutates a container, rather
        // than through an instruction which builds one.
        "push" => ("", "[]", "let b = []; b.push(a); a = b;"),
        "control-flow" => (
            "use std::ops::ControlFlow;",
            "0",
            "a = ControlFlow::Continue(a);",
        ),
        "vec-deque" => (
            "use std::collections::VecDeque;",
            "VecDeque::new()",
            "let b = VecDeque::new(); b.push_back(a); a = b;",
        ),
        "hash-map" => (
            "use std::collections::HashMap;",
            "HashMap::new()",
            "let b = HashMap::new(); b.insert(0, a); a = b;",
        ),
        "iter-once" => ("use std::iter::once;", "0", "a = once(a);"),
        "iter-chain" => ("", "[1].iter()", "a = a.chain([1]);"),
        "iter-enumerate" => ("", "[1].iter()", "a = a.enumerate();"),
        "iter-filter" => ("", "[1].iter()", "a = a.filter(|x| true);"),
        "iter-filter-map" => ("", "[1].iter()", "a = a.filter_map(|x| Some(x));"),
        "iter-flat-map" => ("", "[1].iter()", "a = a.flat_map(|x| [x]);"),
        "iter-map" => ("", "[1].iter()", "a = a.map(|x| x);"),
        "iter-peekable" => ("", "[1].iter()", "a = a.peekable();"),
        "iter-rev" => ("", "[1].iter()", "a = a.rev();"),
        "iter-skip" => ("", "[1].iter()", "a = a.skip(0);"),
        "iter-take" => ("", "[1].iter()", "a = a.take(1);"),
        "iter-capture" => ("", "0", "let b = a; a = [1].iter().map(|x| b);"),
        "slice-iter" => ("", "0", "a = [a].iter();"),
        "object-iter" => ("", "0", "a = #{a: a}.iter();"),
        "object-keys" => ("", "0", "a = #{a: a}.keys();"),
        "object-values" => ("", "0", "a = #{a: a}.values();"),
        "vec-deque-iter" => (
            "use std::collections::VecDeque;",
            "0",
            "let b = VecDeque::new(); b.push_back(a); a = b.iter();",
        ),
        "hash-map-iter" => (
            "use std::collections::HashMap;",
            "0",
            "let b = HashMap::new(); b.insert(0, a); a = b.iter();",
        ),
        "generator-iter" => ("fn g(v) { yield v; }", "0", "a = g(a).iter();"),
        "option-iter" => ("", "0", "a = Some(a).iter();"),
        "hash-set" => (
            "use std::collections::HashSet;",
            "0",
            "let s = HashSet::new(); let v = []; s.insert(v); v.push(a); a = s;",
        ),
        "hash-set-iter" => (
            "use std::collections::HashSet;",
            "0",
            "let s = HashSet::new(); let v = []; s.insert(v); v.push(a); a = s.iter();",
        ),
        "hash-set-difference" => (
            "use std::collections::HashSet;",
            "0",
            "let s = HashSet::new(); let v = []; s.insert(v); v.push(a); \
             a = s.difference(HashSet::new());",
        ),
        "hash-set-intersection" => (
            "use std::collections::HashSet;",
            "0",
            "let s = HashSet::new(); let v = []; s.insert(v); v.push(a); \
             a = HashSet::new().intersection(s);",
        ),
        "hash-set-union" => (
            "use std::collections::HashSet;",
            "0",
            "let s = HashSet::new(); let v = []; s.insert(v); v.push(a); \
             a = s.union(HashSet::new());",
        ),
        "string-split" => ("", "0", "let b = a; a = \"xy\".split(|c| b is Object);"),
        other => panic!("unknown kind: {other}"),
    };

    format!("{decl} let a = {init}; let i = 0; while i < {depth} {{ {step} i += 1; }}")
}

/// A graph of values is taken apart over a worklist rather than by recursing
/// into it, so a deeply nested value can be dropped no matter how deep it is.
///
/// This nesting is built by the machine, so it is bounded by neither `max-depth`
/// nor anything else the compiler applies.
#[test]
fn deep_values_drop() {
    for kind in NESTED {
        let source = format!("{} let b = 1; b", nested_value(DEEP_VALUE, kind));

        let value = run_deep(&source).expect("Deeply nested value should drop");
        let value: i64 = crate::from_value(value).expect("Failed to convert");
        assert_eq!(value, 1, "{kind}");
    }
}

/// How deep the chains of awaits below are.
///
/// An await used to drive the callee as a machine of its own, nested inside the
/// caller's, which cost a native frame per level - a little under a kilobyte in
/// release and several in debug. A few thousand levels aborted the process on
/// the main thread and a few hundred on the much smaller one a test runs on, so
/// this is well past what used to overflow either.
const DEEP_AWAIT: usize = 20000;

/// Awaiting an async call which has yet to run splices it into the awaiting
/// machine as an ordinary call frame, so a chain of awaits costs stack the same
/// way a chain of ordinary calls does - on the heap, where the memory limit can
/// see it - rather than costing native frames.
#[test]
fn deep_await_chain_does_not_recurse() {
    let source = format!(
        "async fn rec(n) {{ if n == 0 {{ 0 }} else {{ rec(n - 1).await + 1 }} }} \
         let a = rec({DEEP_AWAIT}).await; a"
    );

    let value: i64 = eval_deep(&source);
    assert_eq!(value, DEEP_AWAIT as i64);
}

/// An async function reached through a function pointer builds its machine on
/// the way in rather than at the call site, so it splices through a second
/// construction site which has to be flattened as well.
#[test]
fn deep_await_chain_through_a_function_pointer() {
    let source = format!(
        "async fn rec(n, f) {{ if n == 0 {{ 0 }} else {{ f(n - 1, f).await + 1 }} }} \
         let a = rec({DEEP_AWAIT}, rec).await; a"
    );

    let value: i64 = eval_deep(&source);
    assert_eq!(value, DEEP_AWAIT as i64);
}

/// The machine keeps the worklist it takes values apart over, so the memory
/// that takes is grown once rather than for every value it destroys.
#[test]
fn machine_keeps_its_worklist() {
    // Overwriting the register the value is held in is the machine taking it
    // apart, which is what the worklist is grown for. The value holds more at
    // once than the worklist keeps without allocating, since that is what
    // growing it takes.
    let source = format!("let a = [{}]; a = 0; 1", "[], ".repeat(64));

    let context = Context::with_default_modules().expect("Failed to build context");
    let unit = build(&source, &options(usize::MAX)).expect("Source should compile");

    let runtime = Arc::try_new(context.runtime().expect("Failed to build runtime"))
        .expect("Failed to allocate runtime");
    let unit = Arc::try_new(unit).expect("Failed to allocate unit");

    let mut vm = Vm::new(runtime, unit);

    assert_eq!(
        vm.worklist_capacity(),
        0,
        "A machine should start without a worklist"
    );

    let value = vm
        .execute(Hash::EMPTY, ())
        .expect("Failed to execute")
        .complete()
        .expect("Execution should complete");

    drop(value);

    assert!(
        vm.worklist_capacity() > 0,
        "The machine should have kept the worklist it grew"
    );
}

/// The machine is not the only thing which drops values.
///
/// A native function which empties a container drops what it held without the
/// machine seeing it, and a value handed back to the caller is dropped by the
/// caller once the machine is gone. Taking a graph apart is the destructor's
/// job for that reason.
#[test]
fn deep_values_drop_outside_of_the_machine() {
    // `Vec::clear` drops the chain from inside a native function.
    let source = format!(
        "{} let v = [a]; a = 0; v.clear(); 1",
        nested_value(DEEP_VALUE, "vec")
    );

    let value = run_deep(&source).expect("Deeply nested value should drop");
    let value: i64 = crate::from_value(value).expect("Failed to convert");
    assert_eq!(value, 1);

    // Handed back to us, so this drops it once the machine is gone.
    let source = format!("{} a", nested_value(DEEP_VALUE, "vec"));
    let value = run_deep(&source).expect("Deeply nested value should be returned");
    drop(value);
}

/// Comparing, ordering and formatting a value all descend into the values it is
/// made of by recursing, so how deeply they may nest is bounded, and exceeding
/// it is an error the script can catch rather than an abort it cannot.
#[test]
fn deep_value_walks_are_bounded() {
    let cases = [
        ("if a == b { 1 } else { 0 }", 1),
        ("if a < b { 1 } else { 0 }", 0),
        ("let s = format!(\"{a:?}\"); 1", 1),
    ];

    for (body, expected) in cases {
        // Shallow enough to walk.
        let source = format!(
            "{} {} {body}",
            nested_value(8, "tuple"),
            nested_value(8, "tuple")
                .replace("let a", "let b")
                .replace("a = (a,);", "b = (b,);")
        );

        let value = run_deep(&source).expect("Shallow value should walk");
        let value: i64 = crate::from_value(value).expect("Failed to convert");
        assert_eq!(value, expected, "{body}");

        // Too deep to walk.
        let source = format!(
            "{} {} {body}",
            nested_value(2000, "tuple"),
            nested_value(2000, "tuple")
                .replace("let a", "let b")
                .replace("a = (a,);", "b = (b,);")
        );

        let error = run_deep(&source).expect_err("Should exceed the limit");
        assert!(
            error.to_string().contains("nested too deeply"),
            "{body}: {error}"
        );
    }
}

/// Every way a script can get a native function to call back into it.
///
/// The native frame in between means the call cannot be spliced into the
/// machine which is running - the frame needs the value the call produces in
/// order to return - so it is driven by a machine of its own, which costs a
/// native frame for every level a script nests one.
///
/// Each entry recurses through the native function it names, so that nesting
/// them is reported rather than aborting the process.
const REENTRANT: [(&str, &str); 8] = [
    ("option-map", "Some(n).map(|x| f(x - 1));"),
    ("option-and-then", "Some(n).and_then(|x| Some(f(x - 1)));"),
    ("option-unwrap-or-else", "None.unwrap_or_else(|| f(n - 1));"),
    ("result-map", "Ok(n).map(|x| f(x - 1));"),
    ("result-and-then", "Ok(n).and_then(|x| Ok(f(x - 1)));"),
    (
        "sort-by",
        "let v = [1, 2]; v.sort_by(|a, b| { f(n - 1); a.cmp(b) });",
    ),
    (
        "iter-map",
        "let v = [1]; v.iter().map(|x| f(n - 1)).collect::<Vec>();",
    ),
    (
        "iter-fold",
        "let v = [1]; v.iter().fold(0, |a, x| f(n - 1));",
    ),
];

/// How deeply the re-entrant chains below nest.
///
/// A level cost a little under 18 KiB unoptimised when this was measured, so
/// the 2 MiB stack a test runs on was exhausted at around 120 levels. This is
/// well past that.
const DEEP_REENTRY: usize = 10000;

/// A call which re-enters the machine through a native frame is bounded, so a
/// script which nests them gets an error it can catch rather than an abort it
/// cannot.
#[test]
fn native_reentry_is_bounded() {
    for (kind, step) in REENTRANT {
        let source = format!("fn f(n) {{ if n == 0 {{ return 0; }} {step} 0 }} f({DEEP_REENTRY})");

        let error = run_deep(&source).expect_err("Should exceed the limit");
        assert!(
            error.to_string().contains("nested too deeply"),
            "{kind}: {error}"
        );
    }
}

/// Nesting within the limit still runs, so the bound only rejects what would
/// otherwise overflow.
#[test]
fn shallow_native_reentry_runs() {
    for (kind, step) in REENTRANT {
        let source = format!("fn f(n) {{ if n == 0 {{ return 0; }} {step} 0 }} f(8)");

        let value = run_deep(&source).unwrap_or_else(|e| panic!("{kind}: {e}"));
        let value: i64 = crate::from_value(value).expect("Failed to convert");
        assert_eq!(value, 0, "{kind}");
    }
}

/// A generator is driven by a machine of its own too, so resuming one from
/// inside another nests the same way a call through a native frame does.
#[test]
fn nested_generators_are_bounded() {
    let source = format!(
        "fn g(n) {{ if n > 0 {{ let inner = g(n - 1); inner.next(); }} yield 0; }} \
         let outer = g({DEEP_REENTRY}); outer.next(); 0"
    );

    let error = run_deep(&source).expect_err("Should exceed the limit");
    assert!(error.to_string().contains("nested too deeply"), "{error}");

    let source = "fn g(n) { if n > 0 { let inner = g(n - 1); inner.next(); } yield 0; } \
                  let outer = g(8); outer.next(); 0";

    let value = run_deep(source).expect("Shallow nesting should run");
    let value: i64 = crate::from_value(value).expect("Failed to convert");
    assert_eq!(value, 0);
}

/// Serializing a value descends into it by recursing, and a value graph is
/// built by the machine, so a script decides how deeply the walk nests.
///
/// The bound is the one every other walk over a value is subject to, so a graph
/// which is too deep to compare is too deep to serialize as well, and it is
/// reported rather than overflowing the stack.
#[cfg(feature = "serde")]
#[test]
fn deep_value_serialization_is_bounded() {
    let source = format!("{} a", nested_value(2000, "vec"));
    let value = run_deep(&source).expect("Deeply nested value should be returned");

    let error = serde_json::to_string(&value).expect_err("Should exceed the limit");
    assert!(error.to_string().contains("nested too deeply"), "{error}");

    // Nesting within the limit still serializes.
    let source = format!("{} a", nested_value(8, "vec"));
    let value = run_deep(&source).expect("Shallow value should be returned");

    let json = serde_json::to_string(&value).expect("Shallow value should serialize");
    assert_eq!(json, "[[[[[[[[[]]]]]]]]]");
}

/// A value graph can also be cyclic, which no depth a script writes down bounds
/// at all.
#[cfg(feature = "serde")]
#[test]
fn cyclic_value_serialization_is_bounded() {
    let value = run_deep("let a = []; a.push(a); a").expect("Cyclic value should be returned");

    let error = serde_json::to_string(&value).expect_err("Should exceed the limit");
    assert!(error.to_string().contains("nested too deeply"), "{error}");
}

/// Deserializing a value descends into the document by recursing, and how
/// deeply that nests is decided by whoever wrote it.
#[cfg(feature = "serde")]
#[test]
fn deep_value_deserialization_is_bounded() {
    // Deeper than the walk is allowed to go, but shallow enough that the format
    // itself still hands the nesting over.
    let json = format!("{}{}", "[".repeat(100), "]".repeat(100));

    let error = serde_json::from_str::<Value>(&json).expect_err("Should exceed the limit");
    assert!(error.to_string().contains("nested too deeply"), "{error}");

    // Nesting within the limit still deserializes.
    let json = format!("{}{}", "[".repeat(8), "]".repeat(8));
    serde_json::from_str::<Value>(&json).expect("Shallow document should deserialize");
}

/// An adapter delegates to the iterator it wraps by walking into it, so a chain
/// of them is a value nested as deeply as the chain is long, and a script
/// decides how long that is.
///
/// The walk is bounded the same way every other walk over a value is, so a
/// chain which is too long to iterate is reported rather than overflowing.
#[test]
fn deep_iterator_chains_are_bounded() {
    let steps = [
        "a = a.skip(0);",
        "a = a.take(1);",
        "a = a.map(|x| x);",
        "a = a.filter(|x| true);",
        "a = a.filter_map(|x| Some(x));",
        "a = a.flat_map(|x| [x]);",
        "a = a.enumerate();",
        "a = a.peekable();",
        "a = a.chain([1]);",
        "a = a.rev();",
    ];

    for step in steps {
        let source = format!(
            "let a = [1].iter(); let i = 0; while i < 2000 {{ {step} i += 1; }} a.next(); 1"
        );

        let error = run_deep(&source).expect_err("Should exceed the limit");
        assert!(
            error.to_string().contains("nested too deeply"),
            "{step}: {error}"
        );

        // A chain within the limit still iterates.
        let source =
            format!("let a = [1].iter(); let i = 0; while i < 8 {{ {step} i += 1; }} a.next(); 1");

        let value = run_deep(&source).unwrap_or_else(|e| panic!("{step}: {e}"));
        let value: i64 = crate::from_value(value).expect("Failed to convert");
        assert_eq!(value, 1, "{step}");
    }
}

/// Paths nest through generic arguments, which are lowered from a stream of
/// their own rather than from the tree being walked, so lowering them recurses.
///
/// That is bounded by a limit of its own, which is small: the stack was
/// exhausted somewhere between 300 and 1000 levels unoptimised when this was
/// measured. Exceeding it is a diagnostic rather than an abort.
#[test]
fn nested_generics_are_bounded() {
    let source = format!(
        "fn c() {{}} let a = c{}{};",
        "::<c".repeat(5000),
        " >".repeat(5000)
    );

    let error = build(&source, &options(usize::MAX)).expect_err("Should exceed the limit");
    assert!(
        matches!(error, ErrorKind::MaxGenericsDepth { .. }),
        "{error:?}"
    );

    // `max-depth` lowers the bound but does not raise it.
    let error = build(&source, &options(4)).expect_err("Should exceed the limit");
    assert!(
        matches!(
            error,
            ErrorKind::MaxGenericsDepth { max: 4 } | ErrorKind::MaxNesting { max: 4 }
        ),
        "{error:?}"
    );

    // Nesting within the limit is still lowered, and reaches the point where
    // what it refers to is looked up.
    let source = format!(
        "fn c() {{}} let a = c{}{};",
        "::<c".repeat(4),
        " >".repeat(4)
    );
    let error = build(&source, &options(usize::MAX)).expect_err("Should not resolve");
    assert!(
        matches!(error, ErrorKind::MissingItemParameters { .. }),
        "{error:?}"
    );
}

/// Building a constant needs the value of every constant it mentions, so those
/// are built here and now rather than queued, and the recursion goes through
/// the whole of lowering once per level.
///
/// A level costs far more stack than any other nesting the compiler walks - a
/// chain of around twenty aborted the process - so how deeply items may depend
/// on each other is bounded, and exceeding it is a diagnostic.
#[test]
fn item_recursion_is_bounded() {
    // A constant which is defined in terms of the one before it.
    let mut source = String::from("const A0 = 1;");

    for i in 1..3000 {
        source.push_str(&format!("const A{i} = A{}; ", i - 1));
    }

    source.push_str("let a = A2999;");

    let error = build(&source, &options(usize::MAX)).expect_err("Should exceed the limit");
    assert!(
        matches!(error, ErrorKind::ItemRecursionLimit { .. }),
        "{error:?}"
    );

    // The same through constant functions, which are assembled once each and
    // therefore have to be built before whatever calls them can be.
    let mut source = String::from("const fn f0() { 1 }");

    for i in 1..3000 {
        source.push_str(&format!("const fn f{i}() {{ f{}() }} ", i - 1));
    }

    source.push_str("const A = f2999(); let a = A;");

    let error = build(&source, &options(usize::MAX)).expect_err("Should exceed the limit");
    assert!(
        matches!(error, ErrorKind::ItemRecursionLimit { .. }),
        "{error:?}"
    );

    // A chain within the limit is still built.
    let mut source = String::from("const A0 = 1;");

    for i in 1..4 {
        source.push_str(&format!("const A{i} = A{}; ", i - 1));
    }

    source.push_str("A3");

    let value: i64 = eval_deep(&source);
    assert_eq!(value, 1);
}

#[test]
fn probe_format_specs() {
    let context = Context::with_default_modules().expect("context");
    let mut bad = 0usize;
    let mut ran = 0usize;

    macro_rules! check {
        ($spec:literal, $value:expr, $lit:literal) => {{
            ran += 1;
            let expected = rust_alloc::format!(concat!("{:", $spec, "}"), $value);
            let source = rust_alloc::format!("format!(\"{{:{}}}\", {})", $spec, $lit);

            let actual = match crate::tests::run::<String>(&context, &source, (), true) {
                Ok(value) => value,
                Err(e) => {
                    let e = rust_alloc::format!("{e}");
                    rust_alloc::format!("ERR: {}", e.lines().next().unwrap_or(""))
                }
            };

            if actual != expected {
                bad += 1;
                std::eprintln!("SPEC | {source}\n  rust: {expected:?}\n  rune: {actual:?}");
            }
        }};
    }

    check!("", 42i64, "42");
    check!("5", 42i64, "42");
    check!("<5", 42i64, "42");
    check!(">5", 42i64, "42");
    check!("^5", 42i64, "42");
    check!("05", 42i64, "42");
    check!("+", 42i64, "42");
    check!("+05", 42i64, "42");
    check!("x", 255i64, "255");
    check!("X", 255i64, "255");
    check!("o", 8i64, "8");
    check!("b", 5i64, "5");
    check!("#x", 255i64, "255");
    check!("#b", 5i64, "5");
    check!("#o", 8i64, "8");
    check!("#010x", 255i64, "255");
    check!("", -42i64, "-42");
    check!("05", -42i64, "-42");
    check!("+", -42i64, "-42");
    check!("<8", -42i64, "-42");
    check!("", 1.5f64, "1.5");
    check!(".2", 1.5f64, "1.5");
    check!(".0", 1.5f64, "1.5");
    check!("8.2", 1.5f64, "1.5");
    check!("<8.2", 1.5f64, "1.5");
    check!("e", 1500.0f64, "1500.0");
    check!("E", 1500.0f64, "1500.0");
    check!("+.3", 1.5f64, "1.5");
    check!("", "ab", "\"ab\"");
    check!("5", "ab", "\"ab\"");
    check!("<5", "ab", "\"ab\"");
    check!(">5", "ab", "\"ab\"");
    check!("^5", "ab", "\"ab\"");
    check!(".1", "ab", "\"ab\"");
    check!("", 'c', "'c'");
    check!("5", 'c', "'c'");
    check!("", true, "true");
    check!("5", true, "true");
    check!("?", 42i64, "42");
    check!("?", "ab", "\"ab\"");
    check!("?", 'c', "'c'");
    check!("?", 1.5f64, "1.5");

    std::eprintln!("FORMAT-SPECS: {bad} bad out of {ran}");
}
