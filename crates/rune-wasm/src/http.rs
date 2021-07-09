use runestick::{Any, ContextError, Module};
use wasm_bindgen::JsCast as _;
use wasm_bindgen_futures::JsFuture;

/// The wasm `http` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("http");
    module.ty::<Response>()?;
    module.ty::<Error>()?;
    module.async_function(&["get"], get)?;
    module.async_inst_fn("text", Response::text)?;
    Ok(module)
}

#[derive(Any)]
struct Response {
    inner: web_sys::Response,
}

#[derive(Any)]
struct Error;

/// Perform a `get` request.
async fn get(url: &str) -> Result<Response, Error> {
    let mut opts = web_sys::RequestInit::new();
    opts.method("GET");
    opts.mode(web_sys::RequestMode::Cors);

    let window = web_sys::window().ok_or(Error)?;
    let request = web_sys::Request::new_with_str(url).map_err(|_| Error)?;
    let inner = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|_| Error)?;
    let inner: web_sys::Response = inner.dyn_into().map_err(|_| Error)?;

    Ok(Response { inner })
}

impl Response {
    /// Try to get the text of the respponse.
    async fn text(&self) -> Result<String, Error> {
        let text = self.inner.text().map_err(|_| Error)?;
        let text = JsFuture::from(text).await.map_err(|_| Error)?;
        text.as_string().ok_or(Error)
    }
}
