#![allow(unused)]

use rune::runtime::{Mut, Ref};
use rune::{Any, FromValue};
use rune_tests::*;
use std::sync::Arc;

#[derive(Any, Debug, Default)]
struct MyBytes {
    bytes: Vec<u8>,
}

#[derive(FromValue)]
struct Proxy {
    field: Mut<String>,
    my_bytes: Ref<MyBytes>,
}

fn main() -> rune::Result<()> {
    let context = Arc::new(rune_modules::default_context()?);

    let my_bytes = MyBytes::default();

    let mut proxy: Proxy = run(
        &context,
        r#"
        pub fn passthrough(my_bytes) {
            #{ field: String::from_str("hello world"), my_bytes }
        }
        "#,
        &["passthrough"],
        (my_bytes,),
    )?;

    println!("field: {}", &*proxy.field);
    proxy.field.clear();
    Ok(())
}
