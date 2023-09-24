use rune::alloc::prelude::*;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Diagnostics, Vm};

use std::sync::Arc;

#[tokio::main]
async fn main() -> rune::support::Result<()> {
    let context = rune_modules::default_context()?;
    let runtime = Arc::new(context.runtime()?);

    let mut sources = rune::sources! {
        entry => {
            async fn main(timeout) {
                time::delay_for(time::Duration::from_secs(timeout)).await
            }
        }
    };

    let mut diagnostics = Diagnostics::new();

    let result = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        diagnostics.emit(&mut writer, &sources)?;
    }

    let unit = result?;

    let vm = Vm::new(runtime, Arc::new(unit));

    let execution = vm.try_clone()?.send_execute(["main"], (5u32,))?;
    let t1 = tokio::spawn(async move {
        execution.async_complete().await.unwrap();
        println!("timer ticked");
    });

    let execution = vm.try_clone()?.send_execute(["main"], (2u32,))?;
    let t2 = tokio::spawn(async move {
        execution.async_complete().await.unwrap();
        println!("timer ticked");
    });

    tokio::try_join!(t1, t2).unwrap();
    Ok(())
}
