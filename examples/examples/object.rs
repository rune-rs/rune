use rune::runtime::Object;
use rune::Value;
use rune_tests::run;
use std::sync::Arc;

fn main() -> rune::Result<()> {
    let context = Arc::new(rune_modules::default_context()?);

    let mut object = Object::new();
    object.insert(String::from("Hello"), Value::from(42i64));

    let object: Object = run(
        &context,
        r#"
        pub fn calc(input) {
            dbg(input["Hello"]);
            input["Hello"] = "World";
            input
        }
        "#,
        &["calc"],
        (object,),
    )?;

    println!("{:?}", object.get("Hello"));
    Ok(())
}
