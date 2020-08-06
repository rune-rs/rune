use anyhow::Result;

async fn run_main<T, A>(context: &stk::Context, source: &str, args: A) -> Result<T>
where
    T: stk::FromValue,
    A: stk::IntoArgs,
{
    let (unit, _) = rune::compile(source)?;
    let mut vm = stk::Vm::new();
    let task: stk::Task<T> = vm.call_function(context, &unit, &["main"], args)?;
    let output = task.run_to_completion().await?;
    Ok(output)
}

#[tokio::test]
async fn test_custom_functions() -> anyhow::Result<()> {
    let mut module = stk::Module::default();
    module.function(&["test"], || 42).unwrap();

    let mut context = stk::Context::new();
    context.install(module)?;

    assert_eq! {
        run_main::<i64, _>(
            &context,
            r#"
                fn main() {
                    test()
                }
            "#,
            ()
        ).await.unwrap(),
        42,
    };

    Ok(())
}

#[derive(Debug)]
struct Thing(usize);

stk::decl_external!(Thing);

#[tokio::test]
async fn test_passed_in_reference() -> anyhow::Result<()> {
    let mut module = stk::Module::default();
    module
        .function(&["test"], |mut a: Thing, b: &mut Thing| {
            a.0 += 10;
            b.0 -= 10;
            a
        })
        .unwrap();

    let mut context = stk::Context::new();
    context.install(module)?;

    let a = Thing(19);
    let mut b = Thing(21);

    let a = run_main::<Thing, _>(&context, r#"fn main(a, b) { test(a, b) }"#, (a, &mut b))
        .await
        .unwrap();

    assert_eq!(a.0, 29);
    assert_eq!(b.0, 11);
    Ok(())
}
