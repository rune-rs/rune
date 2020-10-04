#![allow(unused)]

use rune::testing::*;
use runestick::{Any, FromValue, Mut, Ref};
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

fn main() -> runestick::Result<()> {
    let context = Arc::new(rune_modules::default_context()?);

    let my_bytes = MyBytes::default();

    let mut proxy: Proxy = run(
        &context,
        &["passthrough"],
        (my_bytes,),
        r#"
        fn passthrough(my_bytes) {
            #{ field: String::from_str("hello world"), my_bytes }
        }
        "#,
    )?;

    println!("field: {}", &*proxy.field);
    proxy.field.clear();
    Ok(())
}
