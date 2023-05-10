//! The native `http` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = { version = "0.12.3", features = ["http", "json"] }
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! let mut context = rune::Context::with_default_modules()?;
//! context.install(rune_modules::http::module(true)?)?;
//! context.install(rune_modules::json::module(true)?)?;
//! # Ok::<_, rune::Error>(())
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

use rune::{Any, Module, Value, ContextError};
use rune::runtime::{Bytes, Protocol};
use std::fmt;
use std::fmt::Write;

/// Construct the `http` module.
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::with_crate("http");

    module.ty::<Client>()?;
    module.ty::<Response>()?;
    module.ty::<RequestBuilder>()?;
    module.ty::<StatusCode>()?;
    module.ty::<Error>()?;

    module.function_meta(Client::new)?;
    module.function_meta(get)?;

    module.function_meta(Client::get)?;
    module.function_meta(Client::post)?;

    module.function_meta(Response::text)?;
    module.function_meta(Response::json)?;
    module.function_meta(Response::status)?;

    module.function_meta(RequestBuilder::send)?;
    module.function_meta(RequestBuilder::header)?;
    module.function_meta(RequestBuilder::body_bytes)?;

    module.inst_fn(Protocol::STRING_DISPLAY, Error::display)?;
    module.inst_fn(Protocol::STRING_DISPLAY, StatusCode::display)?;
    Ok(module)
}

#[derive(Debug, Any)]
#[rune(item = ::http)]
pub struct Error {
    inner: reqwest::Error,
}

impl From<reqwest::Error> for Error {
    fn from(inner: reqwest::Error) -> Self {
        Self { inner }
    }
}

impl Error {
    fn display(&self, buf: &mut String) -> fmt::Result {
        write!(buf, "{}", self.inner)
    }
}

/// An asynchronous Client to make Requests with.
#[derive(Debug, Any)]
#[rune(item = ::http)]
struct Client {
    client: reqwest::Client,
}

/// A Response to a submitted [`Request`].
#[derive(Debug, Any)]
#[rune(item = ::http)]
pub struct Response {
    response: reqwest::Response,
}

impl Response {
    /// Get the response as text.
    #[rune::function]
    async fn text(self) -> Result<String, Error> {
        let text = self.response.text().await?;
        Ok(text)
    }

    /// Get the response as a Rune value decoded from JSON.
    #[rune::function]
    async fn json(self) -> Result<Value, Error> {
        let text = self.response.json().await?;
        Ok(text)
    }

    /// Get the status code of the response.
    #[rune::function]
    fn status(&self) -> StatusCode {
        let inner = self.response.status();
        StatusCode { inner }
    }
}

#[derive(Debug, Any)]
#[rune(item = ::http)]
pub struct StatusCode {
    inner: reqwest::StatusCode,
}

impl StatusCode {
    fn display(&self, buf: &mut String) -> fmt::Result {
        write!(buf, "{}", self.inner)
    }
}

/// A builder to construct the properties of a Request.
///
/// To construct a RequestBuilder, refer to the [`Client`] documentation.
#[derive(Debug, Any)]
#[rune(item = ::http)]
pub struct RequestBuilder {
    request: reqwest::RequestBuilder,
}

impl RequestBuilder {
    /// Send the request being built.
    #[rune::function]
    async fn send(self) -> Result<Response, Error> {
        let response = self.request.send().await?;
        Ok(Response { response })
    }

    /// Modify a header in the request.
    #[rune::function]
    fn header(self, key: &str, value: &str) -> Self {
        Self {
            request: self.request.header(key, value),
        }
    }

    /// Set the request body from bytes.
    ///
    /// ```rune
    /// let client = http::Client::new();
    ///
    /// let response = client.get("http://example.com")
    ///     .body_bytes(body)
    ///     .send()
    ///     .await?;
    ///
    /// let response = response.text().await?;
    /// ```
    #[rune::function]
    fn body_bytes(self, bytes: Bytes) -> Self {
        let bytes = bytes.into_vec();

        Self {
            request: self.request.body(bytes),
        }
    }
}

impl Client {
    /// Construct a new http client.
    ///
    /// # Examples
    ///
    /// ```rune
    /// let client = http::Client::new();
    /// ```
    #[rune::function(path = Self::new)]
    fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Construct a builder to GET the given `url`.
    ///
    /// # Examples
    ///
    /// ```rune
    /// let client = http::Client::new();
    ///
    /// let response = client.get("http://example.com")
    ///     .send()
    ///     .await?;
    ///
    /// let response = response.text().await?;
    /// ```
    #[rune::function]
    fn get(&self, url: &str) -> RequestBuilder {
        RequestBuilder { request: self.client.get(url) }
    }

    /// Construct a builder to POST to the given `url`.
    ///
    /// # Examples
    ///
    /// ```rune
    /// let client = http::Client::new();
    ///
    /// let response = client.post("https://postman-echo.com/post")
    ///     .body_bytes(b"Hello World")
    ///     .send()
    ///     .await?;
    ///
    /// let response = response.json().await?;
    /// ```
    #[rune::function]
    fn post(&self, url: &str) -> RequestBuilder {
        let request = self.client.post(url);
        RequestBuilder { request }
    }
}

/// Shorthand for generating a get request.
///
/// # Examples
///
/// ```rune
/// let response = http::get("http://worldtimeapi.org/api/ip").await?;
/// let json = response.json().await?;
/// 
/// let timezone = json["timezone"];
/// ```
#[rune::function]
async fn get(url: &str) -> Result<Response, Error> {
    Ok(Response {
        response: reqwest::get(url).await?,
    })
}
