use anyhow::Result;
use futures_executor::block_on;

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

#[test]
fn test_custom_functions() -> anyhow::Result<()> {
    let mut module = stk::Module::default();
    module.function(&["test"], || 42).unwrap();

    let mut context = stk::Context::new();
    context.install(module)?;

    assert_eq! {
        block_on(run_main::<i64, _>(
            &context,
            r#"
                fn main() {
                    test()
                }
            "#,
            ()
        )).unwrap(),
        42,
    };

    Ok(())
}

#[derive(Debug)]
struct Thing(usize);

stk::decl_external!(Thing);

#[test]
fn test_passed_in_reference() -> anyhow::Result<()> {
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

    let a = block_on(run_main::<Thing, _>(
        &context,
        r#"fn main(a, b) { test(a, b) }"#,
        (a, &mut b),
    ))
    .unwrap();

    assert_eq!(a.0, 29);
    assert_eq!(b.0, 11);
    Ok(())
}
