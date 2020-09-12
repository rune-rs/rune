#![allow(unused)]

use rune_testing::*;
use runestick::{Any, FromValue, Mut, Ref};

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
    let my_bytes = MyBytes::default();

    let mut proxy: Proxy = run(
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
