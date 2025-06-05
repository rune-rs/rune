use js_sys::Promise;
use rune::{Any, ContextError, Module, VmError};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen(module = "/module.js")]
extern "C" {
    fn js_sleep(ms: i32) -> Promise;
}

/// The wasm 'time' module.
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::with_crate("time")?;
    m.ty::<Duration>()?;
    m.function("from_secs", Duration::from_secs)
        .build_associated::<Duration>()?;
    m.function("sleep", sleep).build()?;
    Ok(m)
}

#[derive(Any)]
#[rune(item = ::time)]
struct Duration(i32);

impl Duration {
    fn from_secs(value: i64) -> Self {
        Self(value as i32 * 1000)
    }
}

async fn sleep(duration: Duration) -> Result<(), VmError> {
    let promise = js_sleep(duration.0);
    let js_fut = JsFuture::from(promise);

    if js_fut.await.is_err() {
        return Err(VmError::panic("Sleep errored"));
    }

    Ok(())
}
