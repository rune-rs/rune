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
