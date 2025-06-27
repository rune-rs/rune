#[path = "hot_reloading/path_reloader.rs"]
mod path_reloader;

use std::path::PathBuf;
use std::pin::pin;

use anyhow::{Context as _, Result};

use rune::sync::Arc;
use rune::{Context, Vm};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let root =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").context("missing CARGO_MANIFEST_DIR")?);

    let context = Context::with_default_modules()?;

    let mut exit = pin!(tokio::signal::ctrl_c());
    let mut reloader = pin!(path_reloader::PathReloader::new(
        root.join("scripts"),
        &context
    )?);

    let context = Arc::try_new(context.runtime()?)?;

    let mut events = Vec::new();

    loop {
        tokio::select! {
            _ = exit.as_mut() => {
                break;
            }
            result = reloader.as_mut().watch(&mut events) => {
                result?;
            }
        }

        for event in events.drain(..) {
            let mut vm = Vm::new(context.clone(), event.unit);

            match event.kind {
                path_reloader::EventKind::Added => {
                    if let Err(error) = vm.call(["hello"], ()) {
                        println!("Error: {error}");
                    }
                }
                path_reloader::EventKind::Removed => {
                    if let Err(error) = vm.call(["goodbye"], ()) {
                        println!("Error: {error}");
                    }
                }
            }
        }
    }

    Ok(())
}
