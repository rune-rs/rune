#![allow(unused)]

use rune::compile::ItemBuf;
use rune::runtime::{Mut, Object, Ref, Shared, Tuple, Value};
use rune::{Any, Context, ContextError, FromValue, Module, ToValue};

#[derive(Any)]
#[rune(name = Bar)]
struct Foo {}

#[derive(Any)]
struct Bar {}

#[test]
fn test_rename() {
    let mut module = Module::new();
    module.ty::<Foo>().unwrap();
    let e = module.ty::<Bar>().unwrap_err();

    match e {
        ContextError::ConflictingType { item, .. } => {
            assert_eq!(item, ItemBuf::with_item(["Bar"]));
        }
        actual => {
            panic!("Expected conflicting type but got: {:?}", actual);
        }
    }
}
