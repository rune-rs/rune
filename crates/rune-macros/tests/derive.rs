use std::fmt::Debug;

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
    struct MyStruct(#[rune(get)] usize);

    #[crate::item_impl(exporter = export_rune_api)]
    impl MyStruct {
        #[rune(export)]
        pub fn foo(&self) -> usize {
            self.0
        }
    }

    #[crate::item_impl(list = rune_api_extension, exporter = export_rune_api_extension)]
    impl MyStruct {
        #[rune(export)]
        pub fn bar(&self) -> usize {
            self.0 + 1
        }

        #[rune(export)]
        pub fn baz() -> usize {
            42
        }

        pub fn rune_export(
            mut module: rune::Module,
        ) -> rune::alloc::Result<Result<rune::Module, rune::ContextError>> {
            for func in Self::rune_api()?
                .into_iter()
                .chain(Self::rune_api_extension()?.into_iter())
            {
                if let Err(e) = module.function_from_meta(func) {
                    return Ok(Err(e));
                }
            }

            Ok(Ok(module))
        }
    }

    let a = MyStruct(2);
    assert_eq!(a.foo() + 1, a.bar());

    fn test_fn<F, T, E>(f: F)
    where
        E: Debug,
        F: Fn(rune::Module) -> Result<T, E>,
    {
        let mut m = rune::Module::new();
        m.ty::<MyStruct>().unwrap();
        f(m).unwrap();
    }

    test_fn(MyStruct::rune_export);
    test_fn(MyStruct::export_rune_api);
    test_fn(MyStruct::export_rune_api_extension);

    assert_eq!(MyStruct::baz(), 42);
}
