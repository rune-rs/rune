#[tokio::main]
async fn main() -> runestick::Result<()> {
    let context = std::sync::Arc::new(rune_modules::default_context()?);

    let unit = rune::testing::build(
        &*context,
        r#"
        pub async fn main() {
            time::delay_for(time::Duration::from_secs(5)).await
        }
        "#,
    )?;

    let vm = runestick::Vm::new(context.clone(), unit.clone());
    vm.execute(&["main"], ())?.async_complete().await?;
    println!("Done sleeping");
    Ok(())
}
