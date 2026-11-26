//! Links in documentation resolve the way a name would where the documented
//! item is.

prelude!();

use crate::compile::Prelude;
use crate::doc::{resolve_link, Artifacts, ItemKind, Link, Visitor};

/// Where a link resolved to, as the item of the page, the kind of page and
/// the anchor on it.
type Resolved = (
    std::string::String,
    std::string::String,
    Option<std::string::String>,
);

fn context() -> Context {
    Context::with_default_modules().expect("Failed to build context")
}

/// Resolve `link` written in the documentation of the standard library item
/// at `scope`.
fn resolve_in(context: &Context, scope: &[&str], link: &str) -> Option<Resolved> {
    let cx = crate::doc::Context::new(Some(context), &[]);
    let prelude = Prelude::with_default_prelude().expect("Failed to build prelude");

    let item =
        ItemBuf::with_crate_item("std", scope.iter().copied()).expect("Failed to build item");

    let scope = *cx
        .meta(&item)
        .expect("Failed to look up scope")
        .first()
        .unwrap_or_else(|| panic!("Missing scope {item}"));

    match resolve_link(&cx, &prelude, scope, link).expect("Failed to resolve") {
        Link::Found(target) => Some((
            target.item.to_string(),
            kind(target.kind),
            target.fragment.map(|f| f.to_string()),
        )),
        Link::Unresolved | Link::NotAPath => None,
    }
}

fn kind(kind: ItemKind) -> std::string::String {
    kind.to_string()
}

fn page(item: &str, kind: &str) -> Option<Resolved> {
    Some((item.into(), kind.into(), None))
}

fn anchor(item: &str, kind: &str, fragment: &str) -> Option<Resolved> {
    Some((item.into(), kind.into(), Some(fragment.into())))
}

/// A bare name resolves to what is in the module the item is documented in.
/// Before, only names directly in that module resolved at all, and only to a
/// struct, enum, macro or function.
#[test]
fn a_bare_name_resolves_in_the_module_of_the_item() {
    let c = context();

    assert_eq!(
        resolve_in(&c, &["vec"], "Vec"),
        page("::std::vec::Vec", "struct")
    );

    assert_eq!(
        resolve_in(&c, &["iter"], "`Iterator`"),
        page("::std::iter::Iterator", "trait")
    );

    // What is in a module above is in scope too, the way it is for a name in
    // a script.
    assert_eq!(
        resolve_in(&c, &["collections", "hash_map"], "hash_set"),
        page("::std::collections::hash_set", "module")
    );

    assert_eq!(
        resolve_in(&c, &["collections", "hash_map"], "collections"),
        page("::std::collections", "module")
    );
}

/// Names which are in scope everywhere through the prelude resolve, which is
/// what makes `[u64]` and `[Option]` work from any module. Without it, these
/// were most of the warnings `rune doc` printed over the standard library.
#[test]
fn names_in_the_prelude_resolve_everywhere() {
    let c = context();

    assert_eq!(resolve_in(&c, &["iter"], "u64"), page("::std::u64", "type"));

    assert_eq!(
        resolve_in(&c, &["iter"], "String"),
        page("::std::string::String", "type")
    );

    assert_eq!(
        resolve_in(&c, &["iter"], "Option"),
        page("::std::option::Option", "enum")
    );

    // A variant is documented on the page of its enum.
    assert_eq!(
        resolve_in(&c, &["vec"], "`None`"),
        anchor("::std::option::Option", "enum", "variant.None")
    );
}

/// A path with more than one segment resolves through its first segment,
/// and an associated function links to its entry on the page of its type,
/// since it has no page of its own.
#[test]
fn a_path_resolves_through_its_first_segment() {
    let c = context();

    assert_eq!(
        resolve_in(&c, &["vec"], "std::option::Option"),
        page("::std::option::Option", "enum")
    );

    assert_eq!(
        resolve_in(&c, &["vec"], "Option::map"),
        anchor("::std::option::Option", "enum", "method.map")
    );

    assert_eq!(
        resolve_in(&c, &["vec"], "Vec::push"),
        anchor("::std::vec::Vec", "struct", "method.push")
    );

    assert_eq!(
        resolve_in(&c, &["vec"], "std::iter::Iterator::next"),
        anchor("::std::iter::Iterator", "trait", "method.next")
    );

    assert_eq!(
        resolve_in(&c, &["vec"], "std::iter::once"),
        page("::std::iter::once", "function")
    );
}

/// `crate::`, `self::`, `super::` and a leading `::` mean what they do in a
/// path written in a script.
#[test]
fn crate_self_super_and_absolute_paths_resolve() {
    let c = context();
    let scope = &["collections", "hash_map"];

    assert_eq!(
        resolve_in(&c, scope, "crate::option::Option"),
        page("::std::option::Option", "enum")
    );

    assert_eq!(
        resolve_in(&c, scope, "self::HashMap"),
        page("::std::collections::hash_map::HashMap", "struct")
    );

    assert_eq!(
        resolve_in(&c, scope, "super::hash_set::HashSet"),
        page("::std::collections::hash_set::HashSet", "struct")
    );

    assert_eq!(
        resolve_in(&c, scope, "super::super::vec::Vec"),
        page("::std::vec::Vec", "struct")
    );

    assert_eq!(
        resolve_in(&c, scope, "::std::iter::Iterator"),
        page("::std::iter::Iterator", "trait")
    );

    // Going above the crate is not anywhere.
    assert_eq!(resolve_in(&c, &["vec"], "super::super::Vec"), None);
}

/// What is documented under a type, like its methods, can name the other
/// items of that type, with or without `Self::`.
#[test]
fn the_items_of_a_type_are_in_scope_in_its_documentation() {
    let c = context();
    let scope = &["iter", "Peekable"];

    assert_eq!(
        resolve_in(&c, scope, "peek"),
        anchor("::std::iter::Peekable", "struct", "method.peek")
    );

    assert_eq!(
        resolve_in(&c, scope, "Self::peek"),
        anchor("::std::iter::Peekable", "struct", "method.peek")
    );
}

/// Every kind of item which has a page, or an entry on one, is linkable. A
/// constant has neither, so it links to what holds it.
#[test]
fn every_kind_of_item_is_linkable() {
    let c = context();

    assert_eq!(
        resolve_in(&c, &["vec"], "println!"),
        page("::std::io::println", "macro")
    );

    assert_eq!(
        resolve_in(&c, &["vec"], "std::f64::consts"),
        page("::std::f64::consts", "module")
    );

    assert_eq!(
        resolve_in(&c, &["vec"], "std::cmp::Ord"),
        page("::std::cmp::Ord", "trait")
    );

    assert_eq!(
        resolve_in(&c, &["vec"], "i64::MAX"),
        page("::std::i64", "type")
    );
}

/// The suffixes and prefixes which say what kind of item is meant restrict
/// what a link resolves to, and the parts of a link which are not a path -
/// generic parameters, arguments and a fragment - are set aside.
#[test]
fn a_link_says_what_it_means() {
    let c = context();

    assert_eq!(
        resolve_in(&c, &["vec"], "`std::iter::once()`"),
        page("::std::iter::once", "function")
    );

    // There is no macro named `once`.
    assert_eq!(resolve_in(&c, &["vec"], "std::iter::once!"), None);
    // ... nor is `Option` something which can be called.
    assert_eq!(resolve_in(&c, &["vec"], "Option()"), None);

    assert_eq!(
        resolve_in(&c, &["vec"], "struct@Vec"),
        page("::std::vec::Vec", "struct")
    );

    assert_eq!(resolve_in(&c, &["vec"], "enum@Vec"), None);

    assert_eq!(
        resolve_in(&c, &["vec"], "Result<T, E>"),
        page("::std::result::Result", "enum")
    );

    assert_eq!(
        resolve_in(&c, &["vec"], "`Some(v)`"),
        anchor("::std::option::Option", "enum", "variant.Some")
    );

    assert_eq!(
        resolve_in(&c, &["vec"], "Vec#examples"),
        anchor("::std::vec::Vec", "struct", "examples")
    );
}

/// A name which is not in scope does not resolve, and brackets around text
/// which is not a path at all - like an interval written in prose - are not
/// a link which failed to resolve.
#[test]
fn what_is_not_in_scope_does_not_resolve() {
    let c = context();
    let cx = crate::doc::Context::new(Some(&c), &[]);
    let prelude = Prelude::with_default_prelude().expect("Failed to build prelude");

    let item = ItemBuf::with_crate_item("std", ["vec"]).expect("Failed to build item");
    let scope = cx.meta(&item).expect("Failed to look up scope")[0];

    let resolve = |link| resolve_link(&cx, &prelude, scope, link).expect("Failed to resolve");

    assert!(matches!(resolve("DoesNotExist"), Link::Unresolved));
    assert!(matches!(resolve("Vec::does_not_exist"), Link::Unresolved));
    // `HashMap` is not in scope in `std::vec` without its path.
    assert!(matches!(resolve("HashMap"), Link::Unresolved));

    assert!(matches!(resolve("-1, 1"), Link::NotAPath));
    assert!(matches!(resolve("hash map"), Link::NotAPath));
    assert!(matches!(resolve("$ty::from_rng"), Link::NotAPath));
    assert!(matches!(resolve("https://example.com"), Link::NotAPath));
}

/// Links in the documentation of a script resolve to the items of that
/// script, and one which does not resolve is left as the text it was written
/// as rather than becoming a link to nowhere.
#[test]
fn links_in_a_script_are_rendered() {
    let context = context();

    let mut sources = crate::tests::sources(
        r#"
        /// A helper.
        pub struct Helper;

        /// Uses [Helper], [`crate::Helper`], [`Vec`], [link][self::Helper],
        /// [inline](Helper) and [Missing].
        pub fn f() {}
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

    let html = artifacts
        .content("script/f.fn.html")
        .expect("Missing page for function");

    let html = core::str::from_utf8(html).expect("Page is not UTF-8");

    assert!(
        html.contains(r#"<a href="Helper.struct.html" title="struct Helper">Helper</a>"#),
        "{html}"
    );

    assert!(
        html.contains(
            r#"<a href="Helper.struct.html" title="struct crate::Helper"><code>crate::Helper</code></a>"#
        ),
        "{html}"
    );

    assert!(
        html.contains(
            r#"<a href="../std/vec/Vec.struct.html" title="struct Vec"><code>Vec</code></a>"#
        ),
        "{html}"
    );

    assert!(
        html.contains(r#"<a href="Helper.struct.html" title="struct self::Helper">link</a>"#),
        "{html}"
    );

    assert!(
        html.contains(r#"<a href="Helper.struct.html" title="struct Helper">inline</a>"#),
        "{html}"
    );

    assert!(html.contains("and [Missing]."), "{html}");
}
