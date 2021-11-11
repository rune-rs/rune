use std::sync::Arc;

#[tokio::main]
async fn main() -> rune::Result<()> {
    let context = rune_modules::default_context()?;

    let unit = rune_tests::build(
        &context,
        r#"
        async fn main(timeout) {
            time::delay_for(time::Duration::from_secs(timeout)).await
        }
        "#,
    )?;

    let main = rune::Hash::type_hash(&["main"]);
    let vm = rune::Vm::new(Arc::new(context.runtime()), unit.clone());

    let execution = vm.clone().send_execute(main, (5,))?;
    let t1 = tokio::spawn(async move {
        execution.async_complete().await.unwrap();
        println!("timer ticked");
    });

    let execution = vm.clone().send_execute(main, (2,))?;
    let t2 = tokio::spawn(async move {
        execution.async_complete().await.unwrap();
        println!("timer ticked");
    });

    tokio::try_join!(t1, t2).unwrap();
    Ok(())
}
