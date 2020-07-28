use anyhow::Result;

async fn run_main<T>(functions: &st::Functions, source: &str) -> Result<T>
where
    T: st::FromValue,
{
    let unit = rune::compile(source)?;
    let mut vm = st::Vm::new();
    let task: st::Task<T> = vm.call_function(functions, &unit, "main", ())?;
    let output = task.run_to_completion().await?;
    Ok(output)
}

#[tokio::test]
async fn test_custom_functions() {
    let mut functions = st::Functions::new();

    functions.global_fn("test", || Ok(42)).unwrap();

    assert_eq! {
        run_main::<i64>(
            &functions,
            r#"
                fn main() {
                    test()
                }
            "#
        ).await.unwrap(),
        42,
    };
}
