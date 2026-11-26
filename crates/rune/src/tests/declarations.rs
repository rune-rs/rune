//! Declaration sources (`.d.rn`): parsed with the same grammar as any other
//! source, holding only signatures, type definitions and their documentation.

prelude!();

use core::cell::RefCell;

use crate::diagnostics::{Diagnostic, FatalDiagnosticKind};
use crate::indexing::declarations::{self, Declaration, DeclarationKind};
use crate::macros::Storage;
use crate::parse::ResolveContext;

use ErrorKind::*;

/// Extract the declarations of `source` as a declaration source, along with
/// the messages of whatever it reported.
fn extract(source: &str) -> (Vec<Declaration>, Vec<String>) {
    let mut sources = Sources::new();
    let id = sources
        .insert(Source::new("host.d.rn", source).unwrap())
        .unwrap();

    let tree = crate::grammar::text(id, source)
        .declaration(true)
        .root()
        .unwrap();

    let storage = Storage::default();
    let scratch = RefCell::new(alloc::String::new());

    let cx = ResolveContext {
        sources: &sources,
        storage: &storage,
        scratch: &scratch,
    };

    let mut diagnostics = Diagnostics::new();
    let declarations = declarations::file(cx, id, &tree, &mut diagnostics).unwrap();
    let declarations = declarations.into_iter().collect();
    (declarations, messages(diagnostics))
}

/// Build `source` as a declaration source named `name`.
fn build(name: &str, source: &str) -> (bool, Vec<String>) {
    let context = Context::new();
    let mut sources = Sources::new();
    sources.insert(Source::new(name, source).unwrap()).unwrap();

    let mut diagnostics = Diagnostics::new();

    let result = prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build();

    (result.is_ok(), messages(diagnostics))
}

fn messages(diagnostics: Diagnostics) -> Vec<String> {
    let mut out = Vec::new();

    for d in diagnostics.into_diagnostics() {
        if let Diagnostic::Fatal(e) = d {
            if let FatalDiagnosticKind::CompileError(e) = e.into_kind() {
                out.push(e.to_string());
            }
        }
    }

    out
}

fn text(s: &crate::alloc::String) -> &str {
    s.as_str()
}

/// What the language server and `rune doc` are to be given out of a
/// declaration: the name, the arguments with their types, the return type,
/// the documentation and the other attributes. Losing any of these would leave
/// hover and documentation with less than the declaration says.
#[test]
fn a_declaration_source_extracts_signatures() {
    let (decls, errors) = extract(
        r#"
        /// This is a documentation comment that will be rendered.
        #[deprecated = "use hello2"]
        pub fn hello(value: u64, (a, b): (u64, String)) -> u64;

        async fn sleep(ms);

        /// A point.
        struct Point { x: f64, y }
        struct Pair(a, b);

        enum Shape {
            /// A circle.
            Circle { radius: f64 },
            Square(side),
            Empty,
        }

        mod nested {
            mod deeper {
                fn inner() -> !;
            }
        }

        impl Point {
            /// The length.
            fn len(self) -> f64;
        }
        "#,
    );

    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(decls.len(), 7, "{decls:#?}");

    let hello = &decls[0];
    assert!(hello.is_pub);
    assert_eq!(hello.docs.len(), 1);
    assert_eq!(
        text(&hello.docs[0]),
        " This is a documentation comment that will be rendered."
    );
    assert_eq!(hello.attributes.len(), 1);
    assert_eq!(text(&hello.attributes[0]), "#[deprecated = \"use hello2\"]");

    let DeclarationKind::Fn(f) = &hello.kind else {
        panic!("expected function: {hello:?}");
    };

    assert_eq!(text(&f.name), "hello");
    assert!(!f.is_async);
    assert!(f.impl_of.is_none());
    assert_eq!(f.args.len(), 2);
    assert_eq!(text(&f.args[0].pat), "value");
    assert_eq!(f.args[0].ty.as_ref().map(text), Some("u64"));
    assert_eq!(text(&f.args[1].pat), "(a, b)");
    assert_eq!(f.args[1].ty.as_ref().map(text), Some("(u64, String)"));
    assert_eq!(f.output.as_ref().map(text), Some("u64"));

    let DeclarationKind::Fn(f) = &decls[1].kind else {
        panic!("expected function");
    };

    assert_eq!(text(&f.name), "sleep");
    assert!(f.is_async);
    assert_eq!(f.args[0].ty, None);
    assert_eq!(f.output, None);

    let DeclarationKind::Struct(point) = &decls[2].kind else {
        panic!("expected struct");
    };

    assert_eq!(decls[2].docs.len(), 1);
    assert_eq!(text(&point.name), "Point");
    assert_eq!(point.fields.len(), 2);
    assert_eq!(text(&point.fields[0].name), "x");
    assert_eq!(point.fields[0].ty.as_ref().map(text), Some("f64"));
    assert_eq!(text(&point.fields[1].name), "y");
    assert_eq!(point.fields[1].ty, None);

    let DeclarationKind::Struct(pair) = &decls[3].kind else {
        panic!("expected struct");
    };

    assert_eq!(text(&pair.fields[1].name), "1");

    let DeclarationKind::Enum { name, variants } = &decls[4].kind else {
        panic!("expected enum");
    };

    assert_eq!(text(name), "Shape");
    assert_eq!(variants.len(), 3);
    assert_eq!(text(&variants[0].name), "Circle");
    assert_eq!(variants[0].fields[0].ty.as_ref().map(text), Some("f64"));
    assert_eq!(text(&variants[1].fields[0].name), "0");
    assert!(variants[2].fields.is_empty());

    // Bodies are walked last-in first-out, so the `impl` comes before the
    // `mod` which was written ahead of it.
    let DeclarationKind::Fn(len) = &decls[5].kind else {
        panic!("expected function");
    };

    assert_eq!(text(&len.name), "len");
    assert_eq!(len.impl_of.as_ref().map(text), Some("Point"));
    assert_eq!(len.output.as_ref().map(text), Some("f64"));
    assert_eq!(decls[5].docs.len(), 1);

    let inner = &decls[6];
    let DeclarationKind::Fn(f) = &inner.kind else {
        panic!("expected function");
    };

    assert_eq!(text(&f.name), "inner");
    assert_eq!(f.output.as_ref().map(text), Some("!"));
    assert_eq!(
        inner.module.iter().map(text).collect::<Vec<_>>(),
        ["nested", "deeper"]
    );
}

/// A declaration source is not compiled into anything, so building one which
/// only declares succeeds.
#[test]
fn a_declaration_source_builds() {
    let (ok, errors) = build(
        "host.d.rn",
        r#"
        //! The host's API.
        use std::string::String;

        /// Say hello.
        pub fn hello(value: u64) -> u64;
        "#,
    );

    assert!(errors.is_empty(), "{errors:?}");
    assert!(ok);
}

/// Everything which is not a declaration is reported where it was written,
/// rather than silently being left out of what the host is said to provide.
#[test]
fn a_declaration_source_rejects_what_is_not_a_declaration() {
    let cases: &[(&str, &str)] = &[
        (
            "fn hello() { 42 }",
            "a function in a declaration source has no body",
        ),
        (
            "const VALUE = 42;",
            "constants are not allowed in a declaration source",
        ),
        (
            "static VALUE = 42;",
            "constants are not allowed in a declaration source",
        ),
        (
            "let value = 42;",
            "only declarations are allowed in a declaration source",
        ),
        (
            "1 + 2;",
            "only declarations are allowed in a declaration source",
        ),
        (
            "println!(\"hello\");",
            "only declarations are allowed in a declaration source",
        ),
        (
            "mod other;",
            "a declaration source may only contain inline `mod` blocks",
        ),
        (
            "impl Foo { struct Bar; }",
            "an `impl` block in a declaration source may only declare functions",
        ),
        (
            "impl Foo { fn bar() {} }",
            "a function in a declaration source has no body",
        ),
        (
            "mod m { fn bar() {} }",
            "a function in a declaration source has no body",
        ),
        (
            "const fn hello();",
            "only `pub` and `async` are allowed on a declaration",
        ),
    ];

    for (source, expected) in cases {
        let (_, errors) = extract(source);
        assert_eq!(errors, [*expected], "{source}");

        let (ok, errors) = build("host.d.rn", source);
        assert!(!ok, "{source}");
        assert_eq!(errors, [*expected], "{source}");
    }
}

/// Whether a source is a declaration source is decided by its name, so the
/// same text in a `.rn` is an ordinary source, where a signature without a
/// body has no meaning.
#[test]
fn a_function_without_a_body_is_an_error_in_an_ordinary_source() {
    assert_errors! {
        "fn hello();",
        span!(0, 11), Custom { error } => {
            assert_eq!(
                error.to_string(),
                "A function without a body is only allowed in a declaration source (`.d.rn`)"
            );
        }
    };

    let (ok, errors) = build("host.rn", "fn hello();");
    assert!(!ok);
    assert_eq!(
        errors,
        ["A function without a body is only allowed in a declaration source (`.d.rn`)"]
    );
}

/// Type annotations are only for declarations; an ordinary source still
/// reports them as the static typing it does not have.
#[test]
fn type_annotations_are_still_rejected_in_an_ordinary_source() {
    let (ok, errors) = build("host.rn", "fn hello() -> u64 {}");
    assert!(!ok);
    assert_eq!(
        errors,
        ["Adding a return type in functions is not supported"]
    );

    let (ok, errors) = build("host.rn", "struct Point { x: f64 }");
    assert!(!ok);
    assert_eq!(errors, ["Static typing on fields is not supported"]);
}

/// The formatter lays out signatures and type annotations, and leaves what it
/// laid out as it is when run again.
#[test]
fn the_formatter_lays_out_declarations() {
    let options = Options::default();

    let layout = |source: &str| {
        let mut diagnostics = Diagnostics::new();
        crate::fmt::layout_source_as(
            source,
            crate::SourceId::EMPTY,
            true,
            &options,
            &mut diagnostics,
        )
        .unwrap()
        .into_std()
    };

    let source = "/// Docs.\npub   fn hello( value :u64,(a,b):( u64 ,String ) )->u64 ;\nasync fn sleep(ms);\nfn never()->!;\nfn unit()->();\nstruct Point{x:f64,y}\nimpl Point{\n/// Len.\nfn len(self)->f64;\n}\n";

    let expected = "/// Docs.\npub fn hello(value: u64, (a, b): (u64, String)) -> u64;\nasync fn sleep(ms);\nfn never() -> !;\nfn unit() -> ();\nstruct Point {\n    x: f64,\n    y,\n}\nimpl Point {\n    /// Len.\n    fn len(self) -> f64;\n}\n";

    let output = layout(source);
    assert_eq!(output, expected);
    assert_eq!(layout(&output), output);
}

/// Compile `script` as `main.rn` beside `declarations` as `host.d.rn` against
/// `context`, returning a machine running it against `runtime` or the
/// diagnostics of what failed.
fn compile_with(
    context: &Context,
    runtime: &Context,
    declarations: &str,
    script: &str,
) -> Result<crate::Vm, Vec<String>> {
    let mut sources = Sources::new();
    sources
        .insert(Source::new("host.d.rn", declarations).unwrap())
        .unwrap();
    sources
        .insert(Source::new("main.rn", script).unwrap())
        .unwrap();

    let mut diagnostics = Diagnostics::new();

    let result = prepare(&mut sources)
        .with_context(context)
        .with_diagnostics(&mut diagnostics)
        .build();

    if diagnostics.has_error() || result.is_err() {
        let mut buffer = crate::termcolor::Buffer::no_color();
        diagnostics.emit(&mut buffer, &sources).unwrap();
        let output = String::from_utf8(buffer.into_inner()).unwrap();
        return Err(vec![output]);
    }

    let unit = result.unwrap();

    let runtime = Arc::try_new(runtime.runtime().unwrap()).unwrap();
    Ok(crate::Vm::new(runtime, Arc::try_new(unit).unwrap()))
}

const HOST: &str = r#"
/// Say hello.
pub fn hello(value: u64) -> u64;

pub mod net {
    pub fn fetch(url: String) -> String;
}

pub struct Point { x: f64, y: f64 }

impl Point {
    fn new(x: f64, y: f64) -> Point;
    fn len(self) -> f64;
}

pub enum Shape { Circle(f64), Square(f64) }
"#;

/// A script calling what a declaration source declares compiles without the
/// host being present, which is what lets a language server or `rune check`
/// work on scripts written against a host they do not have.
#[test]
fn a_script_calling_declared_functions_compiles() {
    let context = Context::new();

    let script = r#"
    pub fn main(p) {
        hello(1);
        net::fetch("x");
        let p = Point::new(1.0, 2.0);
        Point::len(p);
        p.len();
        p is Point
    }
    "#;

    if let Err(errors) = compile_with(&context, &context, HOST, script) {
        panic!("{errors:?}");
    }
}

/// Declarations only add what they declare: a function nobody declared is
/// still missing.
#[test]
fn an_undeclared_function_is_still_missing() {
    let context = Context::new();

    let Err(errors) = compile_with(&context, &context, HOST, "pub fn main() { goodbye(1) }") else {
        panic!("compiled a call to an undeclared function");
    };

    assert!(errors.iter().any(|e| e.contains("goodbye")), "{errors:?}");
}

/// A declared function the host does not provide fails where it is called,
/// naming the function, rather than with a bare hash or at compile time.
#[test]
fn calling_a_declared_function_without_the_host_is_a_runtime_error() {
    let context = Context::new();

    let mut vm = compile_with(
        &context,
        &context,
        HOST,
        "pub fn main() { net::fetch(\"x\") }",
    )
    .unwrap();

    let error = vm.call(["main"], ()).unwrap_err();

    match error.into_kind() {
        VmErrorKind::MissingDeclaredFunction { item, .. } => {
            assert_eq!(item.try_to_string().unwrap(), "net::fetch");
        }
        kind => panic!("unexpected error: {kind}"),
    }
}

/// A native item of the same name wins over a declaration without a conflict
/// being reported, so a declaration source written for a host keeps working
/// when compiled against that host.
#[test]
fn a_native_item_wins_over_its_declaration() {
    let mut m = Module::new();
    m.function("hello", |value: u64| value + 1).build().unwrap();

    let mut context = Context::new();
    context.install(m).unwrap();

    let mut vm = compile_with(&context, &context, HOST, "pub fn main() { hello(41) }").unwrap();
    let value: u64 = crate::from_value(vm.call(["main"], ()).unwrap()).unwrap();
    assert_eq!(value, 42);
}

/// What a script is compiled against need not be what it runs against: the
/// declarations stand in for the host at compile time, and the host's own
/// function is what is called at run time.
#[test]
fn a_declared_function_calls_what_the_host_provides_at_run_time() {
    let mut m = Module::with_item(["net"]).unwrap();
    m.function("fetch", |url: &str| url.len() as u64)
        .build()
        .unwrap();

    let mut runtime = Context::new();
    runtime.install(m).unwrap();

    let mut vm = compile_with(
        &Context::new(),
        &runtime,
        HOST,
        "pub fn main() { net::fetch(\"four\") }",
    )
    .unwrap();

    let value: u64 = crate::from_value(vm.call(["main"], ()).unwrap()).unwrap();
    assert_eq!(value, 4);
}
