#[tokio::main]
async fn main() -> runestick::Result<()> {
    let (context, unit) = rune::testing::build(
        r#"
        async fn main() {
            time::delay_for(time::Duration::from_secs(5)).await
        }
        "#,
    )?;

    let vm = runestick::Vm::new(context.clone(), unit.clone());
    vm.execute(&["main"], ())?.async_complete().await?;
    println!("Done sleeping");
    Ok(())
}
