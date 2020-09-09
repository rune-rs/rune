#![allow(unused)]

use rune_testing::*;
use runestick::{Any, FromValue, OwnedMut, OwnedRef};

#[derive(Any, Debug, Default)]
struct MyBytes {
    bytes: Vec<u8>,
}

#[derive(FromValue)]
struct Proxy {
    field: OwnedMut<String>,
    #[rune(any)]
    my_bytes: OwnedRef<MyBytes>,
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
