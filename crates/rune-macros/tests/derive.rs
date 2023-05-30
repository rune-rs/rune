use rune::T;
use rune_macros::*;

#[test]
fn derive_outside_rune() {
    #[derive(Debug, Clone, PartialEq, Eq, ToTokens, Parse, Spanned)]
    struct SomeThing {
        eq: T![=],
    }
}

#[test]
fn generic_derive() {
    #[derive(Debug, Clone, PartialEq, Eq, ToTokens, Parse, Spanned)]
    struct EqValue<T> {
        eq: rune::ast::Eq,
        value: T,
    }
}
