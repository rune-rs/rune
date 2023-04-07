//! Tests for derive(Any) generates the necessary metadata to match over fields.

use rune_tests::prelude::*;

#[derive(Any, Clone, Copy)]
struct External {
    #[rune(get)]
    a: u32,
    #[rune(get)]
    b: u32,
}

fn make_module() -> Result<Module, ContextError> {
    let mut module = Module::new();
    module.ty::<External>()?;
    Ok(module)
}

#[test]
fn test_external_field_match() {
    let m = make_module().expect("failed make module");

    let e = External { a: 40, b: 41 };

    assert_eq!(
        rune_n! {
            &m,
            (e,),
            i64 => pub fn main(v) { match v { External { .. } => 2, _ => 0 } }
        },
        2
    );

    assert_eq!(
        rune_n! {
            &m,
            (e,),
            i64 => pub fn main(v) { match v { External { a, .. } => a, _ => 0 } }
        },
        40
    );

    assert_eq!(
        rune_n! {
            &m,
            (e,),
            i64 => pub fn main(v) { match v { External { b, .. } => b, _ => 0 } }
        },
        41
    );

    assert_eq!(
        rune_n! {
            &m,
            (e,),
            i64 => pub fn main(v) { match v { External { a, b } => a + b, _ => 0 } }
        },
        81
    );
}
