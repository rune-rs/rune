use crate::ast::Kind::*;

use super::Node;

#[derive(Debug, Clone, Copy)]
pub(crate) enum NodeClass {
    Const,
    Local,
    Item,
    Expr,
}

/// Classify the kind of a node.
pub(crate) fn classify(node: &Node<'_>) -> (bool, NodeClass) {
    match node.kind() {
        Local => return (true, NodeClass::Local),
        Item => {
            for node in node.children() {
                let needs_semi = match node.kind() {
                    ItemConst => return (true, NodeClass::Const),
                    ItemStruct => node
                        .children()
                        .rev()
                        .any(|n| matches!(n.kind(), TupleBody | EmptyBody)),
                    ItemEnum | ItemFn | ItemImpl | ItemMod => false,
                    ItemFileMod => true,
                    _ => continue,
                };

                return (needs_semi, NodeClass::Item);
            }
        }
        Expr => {
            if node.children().rev().map(|n| n.kind()).any(|k| {
                matches!(
                    k,
                    ExprIf | ExprFor | ExprWhile | ExprLoop | ExprMatch | ExprSelect | Block
                )
            }) {
                return (false, NodeClass::Item);
            }
        }
        _ => {}
    }

    (false, NodeClass::Expr)
}
