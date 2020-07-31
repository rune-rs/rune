use anyhow::Result;

async fn run_main<T>(functions: &st::Context, source: &str) -> Result<T>
where
    T: st::FromValue,
{
    let unit = rune::compile(source)?;
    let mut vm = st::Vm::new();
    let task: st::Task<T> = vm.call_function(functions, &unit, &["main"], ())?;
    let output = task.run_to_completion().await?;
    Ok(output)
}

#[tokio::test]
async fn test_custom_functions() -> anyhow::Result<()> {
    let mut module = st::Module::default();
    module.free_fn("test", || 42).unwrap();

    let mut context = st::Context::new();
    context.install(module)?;

    assert_eq! {
        run_main::<i64>(
            &context,
            r#"
                fn main() {
                    test()
                }
            "#
        ).await.unwrap(),
        42,
    };

    Ok(())
}
