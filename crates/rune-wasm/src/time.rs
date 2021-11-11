use js_sys::Promise;
use rune::{Any, ContextError, Module, VmError};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen(module = "/module.js")]
extern "C" {
    fn sleep(ms: i32) -> Promise;
}

/// The wasm 'time' module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("time");
    module.ty::<Duration>()?;
    module.function(&["Duration", "from_secs"], Duration::from_secs)?;
    module.async_function(&["delay_for"], delay_for)?;
    Ok(module)
}

#[derive(Any)]
struct Duration(i32);

impl Duration {
    fn from_secs(value: i64) -> Self {
        Self(value as i32 * 1000)
    }
}

async fn delay_for(duration: Duration) -> Result<(), VmError> {
    let promise = sleep(duration.0);
    let js_fut = JsFuture::from(promise);
    js_fut.await.map_err(|_| VmError::panic("future errored"))?;
    Ok(())
}
