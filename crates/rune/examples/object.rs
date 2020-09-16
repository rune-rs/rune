use rune::testing::run;
use runestick::{Object, Value};

fn main() -> runestick::Result<()> {
    let mut object = Object::new();
    object.insert(String::from("Hello"), Value::from(42i64));

    let object: Object = run(
        &["calc"],
        (object,),
        r#"
        fn calc(input) {
            dbg(input["Hello"]);
            input["Hello"] = "World";
            input
        }
        "#,
    )?;

    println!("{:?}", object.get("Hello"));
    Ok(())
}
