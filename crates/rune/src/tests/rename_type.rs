prelude!();

#[derive(Any)]
#[rune(name = Bar)]
struct Foo {}

#[derive(Any)]
struct Bar {}

#[test]
fn test_rename() -> Result<()> {
    let mut module = Module::new();
    module.ty::<Foo>().unwrap();
    let e = module.ty::<Bar>().unwrap_err();

    match e {
        ContextError::ConflictingType { item, .. } => {
            assert_eq!(item, ItemBuf::with_item(["Bar"])?);
        }
        actual => {
            panic!("Expected conflicting type but got: {:?}", actual);
        }
    }

    Ok(())
}
