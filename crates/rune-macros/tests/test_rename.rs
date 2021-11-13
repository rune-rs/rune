#![allow(unused)]

use rune::compile::Item;
use rune::runtime::{Mut, Object, Ref, Shared, Tuple, Value};
use rune::{Any, Context, ContextError, FromValue, Module, ToValue};

#[derive(Any)]
#[rune(name = "Bar")]
struct Foo {}

#[derive(Any)]
struct Bar {}

#[test]
fn test_rename() {
    let mut module = Module::new();
    module.ty::<Foo>().unwrap();
    module.ty::<Bar>().unwrap();

    let mut context = Context::new();
    let e = context.install(&module).unwrap_err();

    match e {
        ContextError::ConflictingType { item, .. } => {
            assert_eq!(item, Item::with_item(&["Bar"]));
        }
        actual => {
            panic!("expected conflicting type but got: {:?}", actual);
        }
    }
}
