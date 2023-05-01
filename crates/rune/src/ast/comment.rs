use crate::ast::prelude::*;

/// A declaration.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub enum Commment {
    Line {
        /// The comment text.
        #[rune(iter)]
        text: ast::LitStr,
    },
    MultiLine {
        /// The comment text.
        #[rune(iter)]
        text: ast::LitStr,
    },
}
