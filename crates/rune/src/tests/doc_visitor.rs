//! What the documentation builder is handed for a source it was pointed at.

prelude!();

use crate::compile::meta;
use crate::doc::Visitor;

/// The top level of a script is compiled as a function of its own, and that
/// function has no item, so it lands on the item of the module which holds it.
///
/// That module is what the documentation is built out of, so it has to stay a
/// module. When it did not, `rune doc` on a script - which is what the CLI does
/// for every path given to it as an argument - failed with "Missing meta for"
/// and documented nothing at all.
#[test]
fn a_script_leaves_its_module_a_module() {
    let context = Context::with_default_modules().expect("Failed to build context");

    let mut sources = crate::tests::sources("/// Documented.\npub fn f() { 1 }\n1");
    let mut diagnostics = Diagnostics::new();

    let mut visitor = Visitor::new(["script"]).expect("Failed to build visitor");

    let mut options = Options::default();
    options.script(true);

    crate::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .with_options(&options)
        .with_visitor(&mut visitor)
        .expect("Failed to install visitor")
        .build()
        .expect("Source should compile");

    let data = visitor
        .get_by_hash(Hash::type_hash(["script"]))
        .expect("The module should have been visited");

    assert!(
        matches!(data.kind, Some(meta::Kind::Module)),
        "{:?}",
        data.kind
    );

    // What the script declared is still there to be documented.
    let data = visitor
        .get_by_hash(Hash::type_hash(["script", "f"]))
        .expect("The function should have been visited");

    assert!(
        matches!(data.kind, Some(meta::Kind::Function { .. })),
        "{:?}",
        data.kind
    );
}

/// What an `impl` block declares is documented on the page of its type when
/// the visitor has a base, which every documented script has. The compiler
/// names the container of an associated item without the base, so the methods
/// were left off the page.
#[test]
fn methods_are_documented_on_their_type() {
    use crate::doc::Artifacts;

    let context = Context::with_default_modules().expect("Failed to build context");

    let mut sources = crate::tests::sources(
        r#"
        pub struct Point {}

        impl Point {
            /// The length.
            fn len(self) {}
        }

        pub enum Shape {
            /// A circle.
            Circle,
        }
        "#,
    );

    let mut diagnostics = Diagnostics::new();
    let base = ItemBuf::with_crate("script").expect("Failed to build item");
    let mut visitor = Visitor::new(&base).expect("Failed to build visitor");

    crate::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .with_visitor(&mut visitor)
        .expect("Failed to install visitor")
        .build()
        .expect("Source should compile");

    let mut artifacts = Artifacts::new();

    crate::doc::build("root", &mut artifacts, Some(&context), &[visitor])
        .expect("Failed to build documentation");

    let page = |path: &str| {
        let html = artifacts.content(path).expect("Missing page");
        core::str::from_utf8(html).expect("Page is not UTF-8")
    };

    let html = page("script/Point.struct.html");
    assert!(html.contains("method.len"), "{html}");
    assert!(html.contains("The length."), "{html}");

    let html = page("script/Shape.enum.html");
    assert!(html.contains("variant.Circle"), "{html}");
    assert!(html.contains("A circle."), "{html}");
}
