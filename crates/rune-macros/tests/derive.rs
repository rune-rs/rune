use rune::T;
use rune_macros::*;

#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Parse, Spanned)]
struct SomeThing {
    eq: T![=],
}

#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Parse, Spanned)]
struct EqValue<T> {
    eq: rune::ast::Eq,
    value: T,
}
