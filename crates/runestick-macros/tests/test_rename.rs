#![allow(unused)]

use runestick::{ContextError, Item, Mut, Object, Ref, Shared, Tuple, Value};
use runestick_macros::{Any, FromValue, ToValue};

#[derive(Any)]
#[rune(name = "Bar")]
struct Foo {}

#[derive(Any)]
struct Bar {}

#[test]
fn test_rename() {
    let mut module = runestick::Module::empty();
    module.ty::<Foo>().unwrap();
    module.ty::<Bar>().unwrap();

    let mut context = runestick::Context::new();
    let e = context.install(&module).unwrap_err();

    match e {
        ContextError::ConflictingType { name, .. } => {
            assert_eq!(name, Item::of(&["Bar"]));
        }
        actual => {
            panic!("expected conflicting type but got: {:?}", actual);
        }
    }
}
