use js_sys::Promise;
use rune::runtime::VmResult;
use rune::{Any, ContextError, Module};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen(module = "/module.js")]
extern "C" {
    fn sleep(ms: i32) -> Promise;
}

/// The wasm 'time' module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("time")?;
    module.ty::<Duration>()?;
    module
        .function("from_secs", Duration::from_secs)
        .build_associated::<Duration>()?;
    module.function("delay_for", delay_for).build()?;
    Ok(module)
}

#[derive(Any)]
#[rune(item = ::time)]
struct Duration(i32);

impl Duration {
    fn from_secs(value: i64) -> Self {
        Self(value as i32 * 1000)
    }
}

async fn delay_for(duration: Duration) -> VmResult<()> {
    let promise = sleep(duration.0);
    let js_fut = JsFuture::from(promise);

    if js_fut.await.is_err() {
        return VmResult::panic("future errored");
    }

    VmResult::Ok(())
}
