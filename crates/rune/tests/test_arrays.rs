use anyhow::Result;

async fn run_main<T>(source: &str) -> Result<T>
where
    T: st::FromValue,
{
    let unit = rune::compile(source)?;
    let mut vm = st::Vm::new();
    let context = st::Context::new();
    let task: st::Task<T> = vm.call_function(&context, &unit, &["main"], ())?;
    let output = task.run_to_completion().await?;
    Ok(output)
}

#[tokio::test]
async fn test_arrays() {
    assert_eq! {
        run_main::<()>(r#"fn main() { let v = [1, 2, 3, 4, 5]; }"#).await.unwrap(),
        (),
    };
}
