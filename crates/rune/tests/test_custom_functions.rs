use futures_executor::block_on;
use std::sync::Arc;

async fn run_main<T, A>(context: Arc<runestick::Context>, source: &str, args: A) -> rune::Result<T>
where
    T: runestick::FromValue,
    A: runestick::IntoArgs,
{
    let (unit, _) = rune::compile(&*context, source)?;
    let vm = runestick::Vm::new(Arc::new(unit));
    let mut task: runestick::Task<T> = vm.call_function(context, &["main"], args)?;
    let output = task.run_to_completion().await?;
    Ok(output)
}

#[test]
fn test_custom_functions() {
    let mut module = runestick::Module::default();
    module.function(&["test"], || 42).unwrap();

    let mut context = runestick::Context::new();
    context.install(module).unwrap();
    let context = Arc::new(context);

    assert_eq! {
        block_on(run_main::<i64, _>(
            context,
            r#"
                fn main() {
                    test()
                }
            "#,
            ()
        )).unwrap(),
        42,
    };
}

#[derive(Debug)]
struct Thing(usize);

runestick::decl_external!(Thing);

#[test]
fn test_passed_in_reference() {
    let mut module = runestick::Module::default();
    module
        .function(&["test"], |mut a: Thing, b: &mut Thing| {
            a.0 += 10;
            b.0 -= 10;
            a
        })
        .unwrap();

    let mut context = runestick::Context::new();
    context.install(module).unwrap();
    let context = Arc::new(context);

    let a = Thing(19);
    let mut b = Thing(21);

    let a = block_on(run_main::<Thing, _>(
        context,
        r#"fn main(a, b) { test(a, b) }"#,
        (a, &mut b),
    ))
    .unwrap();

    assert_eq!(a.0, 29);
    assert_eq!(b.0, 11);
}
