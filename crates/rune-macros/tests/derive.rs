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

#[test]
fn export_impl() {
    #[derive(crate::Any)]
    struct MyStruct(usize);

    #[crate::impl_item]
    impl MyStruct {
        #[export]
        pub fn foo(&self) -> usize {
            self.0
        }
    }

    #[crate::impl_item(export_rune_api_extension)]
    impl MyStruct {
        #[export]
        pub fn bar(&self) -> usize {
            self.0 + 1
        }
    }

    assert!(MyStruct(2).foo() + 1 == MyStruct(2).bar());
}
