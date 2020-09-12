//! The native `http` module for the [Rune Language].
//!
//! [Rune Language]: https://github.com/rune-rs/rune
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = {version = "0.6.16", features = ["http", "json"]}
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> runestick::Result<()> {
//! let mut context = runestick::Context::with_default_modules()?;
//! context.install(&rune_modules::http::module()?)?;
//! context.install(&rune_modules::json::module()?)?;
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

use runestick::{Any, Bytes, Value};
use std::fmt;
use std::fmt::Write as _;

/// Construct the `http` module.
pub fn module() -> Result<runestick::Module, runestick::ContextError> {
    let mut module = runestick::Module::new(&["http"]);

    module.ty::<Client>()?;
    module.ty::<Response>()?;
    module.ty::<RequestBuilder>()?;
    module.ty::<StatusCode>()?;
    module.ty::<Error>()?;

    module.function(&["Client", "new"], Client::new)?;
    module.async_function(&["get"], get)?;

    module.async_inst_fn("get", Client::get)?;
    module.async_inst_fn("post", Client::post)?;

    module.async_inst_fn("text", Response::text)?;
    module.async_inst_fn("json", Response::json)?;
    module.inst_fn("status", Response::status)?;

    module.async_inst_fn("send", RequestBuilder::send)?;
    module.inst_fn("header", RequestBuilder::header)?;
    module.async_inst_fn("body_bytes", RequestBuilder::body_bytes)?;

    module.inst_fn(runestick::STRING_DISPLAY, StatusCode::display)?;
    Ok(module)
}

#[derive(Debug, Any)]
pub struct Error {
    inner: reqwest::Error,
}

impl From<reqwest::Error> for Error {
    fn from(inner: reqwest::Error) -> Self {
        Self { inner }
    }
}

#[derive(Debug, Any)]
struct Client {
    client: reqwest::Client,
}

#[derive(Debug, Any)]
pub struct Response {
    response: reqwest::Response,
}

#[derive(Debug, Any)]
pub struct StatusCode {
    inner: reqwest::StatusCode,
}

impl StatusCode {
    fn display(&self, buf: &mut String) -> fmt::Result {
        write!(buf, "{}", self.inner)
    }
}

impl Response {
    async fn text(self) -> Result<String, Error> {
        let text = self.response.text().await?;
        Ok(text)
    }

    async fn json(self) -> Result<Value, Error> {
        let text = self.response.json().await?;
        Ok(text)
    }

    /// Get the status code of the response.
    fn status(&self) -> StatusCode {
        let inner = self.response.status();

        StatusCode { inner }
    }
}

#[derive(Debug, Any)]
pub struct RequestBuilder {
    request: reqwest::RequestBuilder,
}

impl RequestBuilder {
    /// Send the request being built.
    async fn send(self) -> Result<Response, Error> {
        let response = self.request.send().await?;
        Ok(Response { response })
    }

    /// Modify a header in the request.
    fn header(self, key: &str, value: &str) -> Self {
        Self {
            request: self.request.header(key, value),
        }
    }

    /// Set the request body from bytes.
    async fn body_bytes(self, bytes: Bytes) -> Result<Self, Error> {
        let bytes = bytes.into_vec();

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
    async fn get(&self, url: &str) -> Result<RequestBuilder, Error> {
        let request = self.client.get(url);
        Ok(RequestBuilder { request })
    }

    /// Construct a builder to POST to the given URL.
    async fn post(&self, url: &str) -> Result<RequestBuilder, Error> {
        let request = self.client.post(url);
        Ok(RequestBuilder { request })
    }
}

/// Shorthand for generating a get request.
async fn get(url: &str) -> Result<Response, Error> {
    Ok(Response {
        response: reqwest::get(url).await?,
    })
}
