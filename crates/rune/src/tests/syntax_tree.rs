prelude!();

use crate::ast::Kind;
use crate::syntax_tree::parse;

/// The playground shows the tree as an indented list, which only reads right
/// if parents come before their children one level up.
#[test]
fn a_tree_is_flattened_parents_first() {
    let tree = parse("fn main() { 42 }").unwrap();
    assert!(tree.error.is_none());

    let first = tree.nodes.first().unwrap();
    assert_eq!(first.kind, Kind::Root);
    assert_eq!(first.depth, 0);
    assert!(!first.is_token);

    for pair in tree.nodes.windows(2) {
        assert!(pair[1].depth <= pair[0].depth + 1);
        assert!(!pair[0].is_token || pair[1].depth <= pair[0].depth);
    }

    assert!(tree
        .nodes
        .iter()
        .any(|n| n.is_token && matches!(n.kind, Kind::Number(..))));
}

/// A tool showing the tree of source being typed would otherwise show
/// nothing at all for most of the time it is being edited.
#[test]
fn source_which_does_not_parse_keeps_the_tree_so_far() {
    for source in [
        "fn main() { 42 }\nfn broken() { a < b < c }",
        "fn main() { 42 }\nfn broken() { \"unterminated",
    ] {
        let tree = parse(source).unwrap();

        assert!(tree.error.is_some(), "{source:?} parsed");
        assert_eq!(tree.nodes.first().unwrap().kind, Kind::Root);
        assert!(tree
            .nodes
            .iter()
            .any(|n| n.is_token && matches!(n.kind, Kind::Number(..))));
    }
}
