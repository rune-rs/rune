//! HTTP module for stk based on reqwest.
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! stk = "0.2"
//! stk-http = "0.2"
//! # not necessary, but useful
//! stk-json = "0.2"
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> stk::Result<()> {
//! let mut context = stk::Context::with_default_packages()?;
//! context.install(stk_http::module()?)?;
//! context.install(stk_json::module()?)?;
//! # Ok(())
//! # }
//! ```
//!
//! Use it in Rune:
//!
//! ```rust,ignore
//! use http;
//! use json;
//!
//! fn main() {
//!     let client = http::Client::new();
//!     let response = client.get("http://worldtimeapi.org/api/ip");
//!     let text = response.text();
//!     let json = json::from_string(text);
//!
//!     let timezone = json["timezone"];
//!
//!     if timezone is String {
//!         dbg(timezone);
//!     }
//!
//!     let body = json::to_bytes(#{"hello": "world"});
//!
//!     let response = client.post("https://postman-echo.com/post")
//!         .body_bytes(body)
//!         .send();
//!
//!     let response = json::from_string(response.text());
//!     dbg(response);
//! }
//! ```

use stk::packages::bytes::Bytes;

#[derive(Debug)]
struct Client {
    client: reqwest::Client,
}

#[derive(Debug)]
pub struct Response {
    response: reqwest::Response,
}

impl Response {
    async fn text(self) -> stk::Result<String> {
        let text = self.response.text().await?;
        Ok(text)
    }
}

#[derive(Debug)]
pub struct RequestBuilder {
    request: reqwest::RequestBuilder,
}

impl RequestBuilder {
    /// Send the request being built.
    async fn send(self) -> stk::Result<Response> {
        let response = self.request.send().await?;
        Ok(Response { response })
    }

    async fn body_bytes(self, bytes: Bytes) -> stk::Result<Self> {
        let bytes = bytes.into_inner();

        Ok(Self {
            request: self.request.body(bytes),
        })
    }
}

impl Client {
    fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Construct a builder to GET the given URL.
    async fn get(&self, url: &str) -> stk::Result<RequestBuilder> {
        let request = self.client.get(url);
        Ok(RequestBuilder { request })
    }

    /// Construct a builder to POST to the given URL.
    async fn post(&self, url: &str) -> stk::Result<RequestBuilder> {
        let request = self.client.post(url);
        Ok(RequestBuilder { request })
    }
}

/// Shorthand for generating a get request.
async fn get(url: &str) -> stk::Result<Response> {
    Ok(Response {
        response: reqwest::get(url).await?,
    })
}

stk::decl_external!(Client);
stk::decl_external!(Response);
stk::decl_external!(RequestBuilder);

/// Construct the http library.
pub fn module() -> Result<stk::Module, stk::ContextError> {
    let mut module = stk::Module::new(&["http"]);

    module.ty(&["Client"]).build::<Client>()?;
    module.ty(&["Response"]).build::<Response>()?;
    module.ty(&["RequestBuilder"]).build::<RequestBuilder>()?;

    module.function(&["Client", "new"], Client::new)?;
    module.async_function(&["get"], get)?;

    module.async_inst_fn("get", Client::get)?;
    module.async_inst_fn("post", Client::post)?;

    module.async_inst_fn("text", Response::text)?;

    module.async_inst_fn("send", RequestBuilder::send)?;
    module.async_inst_fn("body_bytes", RequestBuilder::body_bytes)?;
    Ok(module)
}
