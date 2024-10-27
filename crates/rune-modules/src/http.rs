//! The native `http` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = { version = "0.14.0", features = ["http", "json"] }
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! let mut context = rune::Context::with_default_modules()?;
//! context.install(rune_modules::http::module(true)?)?;
//! context.install(rune_modules::json::module(true)?)?;
//! # Ok::<_, rune::support::Error>(())
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

use rune::alloc::fmt::TryWrite;
use rune::runtime::{Bytes, Formatter, Ref, VmResult};
use rune::{Any, ContextError, Module, Value};
use rune::alloc::prelude::*;

/// A simple HTTP module for Rune.
///
/// # Examples
///
/// ```rune,no_run
/// let res = http::get("https://httpstat.us/200?sleep=100").await;
///
/// dbg!(res.text().await?);
/// ```
#[rune::module(::http)]
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::from_meta(self::module_meta)?;

    module.ty::<Client>()?;
    module.ty::<Response>()?;
    module.ty::<RequestBuilder>()?;
    module.ty::<StatusCode>()?;
    module.ty::<Version>()?;
    module.ty::<Error>()?;

    module.function_meta(get)?;

    module.function_meta(Client::new)?;
    module.function_meta(Client::get)?;
    module.function_meta(Client::post)?;
    module.function_meta(Client::put)?;
    module.function_meta(Client::delete)?;
    module.function_meta(Client::head)?;

    module.function_meta(Response::text)?;
    module.function_meta(Response::json)?;
    module.function_meta(Response::bytes)?;
    module.function_meta(Response::status)?;
    module.function_meta(Response::version)?;
    module.function_meta(Response::content_length)?;

    module.function_meta(RequestBuilder::send)?;
    module.function_meta(RequestBuilder::header)?;
    module.function_meta(RequestBuilder::basic_auth)?;
    module.function_meta(RequestBuilder::bearer_auth)?;
    module.function_meta(RequestBuilder::fetch_mode_no_cors)?;
    module.function_meta(RequestBuilder::body_bytes)?;

    module.function_meta(StatusCode::string_display)?;
    module.function_meta(StatusCode::as_u16)?;
    module.function_meta(StatusCode::as_str)?;
    module.function_meta(StatusCode::canonical_reason)?;
    module.function_meta(StatusCode::is_informational)?;
    module.function_meta(StatusCode::is_success)?;
    module.function_meta(StatusCode::is_redirection)?;
    module.function_meta(StatusCode::is_client_error)?;
    module.function_meta(StatusCode::is_server_error)?;

    module.implement_trait::<StatusCode>(rune::item!(::std::cmp::PartialEq))?;
    module.implement_trait::<StatusCode>(rune::item!(::std::cmp::Eq))?;
    module.implement_trait::<StatusCode>(rune::item!(::std::cmp::PartialOrd))?;
    module.implement_trait::<StatusCode>(rune::item!(::std::cmp::Ord))?;

    module.function_meta(Version::string_debug)?;

    module.implement_trait::<Version>(rune::item!(::std::cmp::PartialEq))?;
    module.implement_trait::<Version>(rune::item!(::std::cmp::Eq))?;
    module.implement_trait::<Version>(rune::item!(::std::cmp::PartialOrd))?;
    module.implement_trait::<Version>(rune::item!(::std::cmp::Ord))?;

    module.function_meta(Error::string_display)?;

    Ok(module)
}

/// An error returned by methods in the `http` module.
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
    #[rune::function(instance, protocol = STRING_DISPLAY)]
    fn string_display(&self, f: &mut Formatter) -> VmResult<()> {
        rune::vm_write!(f, "{}", self.inner);
        VmResult::Ok(())
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
    ///
    /// ```rune,no_run
    /// let client = http::Client::new();
    ///
    /// let response = client.get("http://example.com")
    ///     .body_bytes(b"Hello World")
    ///     .send()
    ///     .await?;
    ///
    /// let response = response.text().await?;
    /// ```
    #[rune::function(vm_result)]
    async fn text(self) -> Result<String, Error> {
        let text = self.response.text().await?;
        // NB: We simply take ownership of the string here, raising an error in
        // case we reach a memory limit.
        Ok(String::try_from(text).vm?)
    }

    /// Get the response as a Rune value decoded from JSON.
    ///
    /// ```rune,no_run
    /// let client = http::Client::new();
    ///
    /// let response = client.get("http://example.com")
    ///     .send()
    ///     .await?;
    ///
    /// let response = response.json().await?;
    /// ```
    #[rune::function]
    async fn json(self) -> Result<Value, Error> {
        let text = self.response.json().await?;
        Ok(text)
    }

    /// Get the response as bytes.
    ///
    /// ```rune,no_run
    /// let client = http::Client::new();
    ///
    /// let response = client.get("http://example.com")
    ///     .send()
    ///     .await?;
    ///
    /// let response = response.bytes().await?;
    /// ```
    #[rune::function(vm_result)]
    async fn bytes(mut self) -> Result<Bytes, Error> {
        let len = self.response.content_length().unwrap_or(0) as usize;
        let mut bytes = Vec::try_with_capacity(len).vm?;

        while let Some(chunk) = self.response.chunk().await? {
            bytes.try_extend_from_slice(chunk.as_ref()).vm?;
        }

        Ok(Bytes::from_vec(bytes))
    }

    /// Get the status code of the response.
    #[rune::function(instance)]
    fn status(&self) -> StatusCode {
        StatusCode { inner: self.response.status() }
    }

    /// Get the version of the response.
    #[rune::function(instance)]
    fn version(&self) -> Version {
        Version { inner: self.response.version() }
    }

    /// Get the content-length of this response, if known.
    ///
    /// Reasons it may not be known:
    ///
    /// - The server didn't send a `content-length` header.
    /// - The response is compressed and automatically decoded (thus changing
    ///   the actual decoded length).
    #[rune::function(instance)]
    fn content_length(&self) -> Option<u64> {
        self.response.content_length()
    }
}

/// An HTTP status code.
#[derive(Debug, Any, PartialEq, Eq, PartialOrd, Ord)]
#[rune(item = ::http)]
pub struct StatusCode {
    inner: reqwest::StatusCode,
}

impl StatusCode {
    /// Returns the `u16` corresponding to this `StatusCode`.
    ///
    /// # Note
    ///
    /// This is the same as the `From<StatusCode>` implementation, but included
    /// as an inherent method because that implementation doesn't appear in
    /// rustdocs, as well as a way to force the type instead of relying on
    /// inference.
    ///
    /// # Example
    ///
    /// ```rune
    /// let status = http::StatusCode::OK;
    /// assert_eq!(status.as_u16(), 200);
    /// ```
    #[rune::function(instance)]
    #[inline]
    fn as_u16(&self) -> u16 {
        self.inner.as_u16()
    }

    /// Returns a &str representation of the `StatusCode`
    ///
    /// The return value only includes a numerical representation of the status
    /// code. The canonical reason is not included.
    ///
    /// # Example
    ///
    /// ```rune
    /// let status = http::StatusCode::OK;
    /// assert_eq!(status.as_str(), "200");
    /// ```
    #[rune::function(instance, vm_result)]
    #[inline]
    fn as_str(&self) -> String {
        self.inner.as_str().try_to_owned().vm?
    }

    /// Get the standardised `reason-phrase` for this status code.
    ///
    /// This is mostly here for servers writing responses, but could potentially
    /// have application at other times.
    ///
    /// The reason phrase is defined as being exclusively for human readers. You
    /// should avoid deriving any meaning from it at all costs.
    ///
    /// Bear in mind also that in HTTP/2.0 and HTTP/3.0 the reason phrase is
    /// abolished from transmission, and so this canonical reason phrase really
    /// is the only reason phrase you’ll find.
    ///
    /// # Example
    ///
    /// ```rune
    /// let status = http::StatusCode::OK;
    /// assert_eq!(status.canonical_reason(), Some("OK"));
    /// ```
    #[inline]
    #[rune::function(instance)]
    fn canonical_reason(&self) -> Option<&'static str> {
        self.inner.canonical_reason()
    }

    /// Check if status is within 100-199.
    #[inline]
    #[rune::function(instance)]
    fn is_informational(&self) -> bool {
        self.inner.is_informational()
    }

    /// Check if status is within 200-299.
    #[inline]
    #[rune::function(instance)]
    fn is_success(&self) -> bool {
        self.inner.is_success()
    }

    /// Check if status is within 300-399.
    #[inline]
    #[rune::function(instance)]
    fn is_redirection(&self) -> bool {
        self.inner.is_redirection()
    }

    /// Check if status is within 400-499.
    #[inline]
    #[rune::function(instance)]
    fn is_client_error(&self) -> bool {
        self.inner.is_client_error()
    }

    /// Check if status is within 500-599.
    #[inline]
    #[rune::function(instance)]
    fn is_server_error(&self) -> bool {
        self.inner.is_server_error()
    }

    #[rune::function(instance, protocol = STRING_DISPLAY)]
    fn string_display(&self, f: &mut Formatter) -> VmResult<()> {
        rune::vm_write!(f, "{}", self.inner);
        VmResult::Ok(())
    }
}

/// Represents a version of the HTTP spec.
#[derive(Debug, Any, PartialEq, Eq, PartialOrd, Ord)]
#[rune(item = ::http)]
pub struct Version {
    inner: reqwest::Version,
}

impl Version {
    #[rune::function(instance, protocol = STRING_DEBUG)]
    fn string_debug(&self, f: &mut Formatter) -> VmResult<()> {
        rune::vm_write!(f, "{:?}", self.inner);
        VmResult::Ok(())
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
    /// Send the request and receive an answer from the server.
    ///
    /// ```rune,no_run
    /// let client = http::Client::new();
    ///
    /// let response = client.get("http://example.com")
    ///     .header("Accept", "text/html")
    ///     .send()
    ///     .await?;
    ///
    /// let response = response.text().await?;
    /// ```
    #[rune::function]
    async fn send(self) -> Result<Response, Error> {
        let response = self.request.send().await?;
        Ok(Response { response })
    }

    /// Modify a header in the request.
    ///
    /// ```rune,no_run
    /// let client = http::Client::new();
    ///
    /// let response = client.get("http://example.com")
    ///     .header("Accept", "text/html")
    ///     .send()
    ///     .await?;
    ///
    /// let response = response.text().await?;
    /// ```
    #[rune::function]
    fn header(self, key: &str, value: &str) -> Self {
        Self {
            request: self.request.header(key, value),
        }
    }

    /// Enable basic authentication in the request.
    ///
    /// ```rune,no_run
    /// let client = http::Client::new();
    ///
    /// let response = client.get("http://example.com")
    ///     .basic_auth("admin", Some("good password"))
    ///     .send()
    ///     .await?;
    ///
    /// let response = response.text().await?;
    /// ```
    #[rune::function]
    fn basic_auth(self, username: &str, password: Option<Ref<str>>) -> Self {
        Self {
            request: self.request.basic_auth(username, password.as_deref()),
        }
    }

    /// Enable bearer authentication in the request.
    ///
    /// ```rune,no_run
    /// let client = http::Client::new();
    ///
    /// let response = client.get("http://example.com")
    ///     .bearer_auth("A1B2C3D4E5")
    ///     .send()
    ///     .await?;
    ///
    /// let response = response.text().await?;
    /// ```
    #[rune::function]
    fn bearer_auth(self, token: &str) -> Self {
        Self {
            request: self.request.bearer_auth(token),
        }
    }

    /// Set version in the request.
    ///
    /// ```rune,no_run
    /// let client = http::Client::new();
    ///
    /// let response = client.get("http://example.com")
    ///     .version(Version::HTTP_2)
    ///     .send()
    ///     .await?;
    ///
    /// let response = response.text().await?;
    /// ```
    #[rune::function]
    fn version(self, version: Version) -> Self {
        Self {
            request: self.request.version(version.inner),
        }
    }

    /// Disable CORS on fetching the request.
    ///
    /// This option is only effective with WebAssembly target.
    /// The [request mode][mdn] will be set to 'no-cors'.
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/Request/mode
    ///
    /// ```rune,no_run
    /// let client = http::Client::new();
    ///
    /// let response = client.get("http://example.com")
    ///     .fetch_mode_no_cors()
    ///     .send()
    ///     .await?;
    ///
    /// let response = response.text().await?;
    /// ```
    #[rune::function]
    fn fetch_mode_no_cors(self) -> Self {
        Self {
            request: self.request.fetch_mode_no_cors(),
        }
    }

    /// Set the request body from bytes.
    ///
    /// ```rune,no_run
    /// let client = http::Client::new();
    ///
    /// let response = client.get("http://example.com")
    ///     .body_bytes(b"Hello World")
    ///     .send()
    ///     .await?;
    ///
    /// let response = response.text().await?;
    /// ```
    #[rune::function]
    fn body_bytes(self, bytes: Bytes) -> Self {
        let bytes = bytes.into_vec();

        Self {
            request: self.request.body(bytes.into_std()),
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
    /// ```rune,no_run
    /// let client = http::Client::new();
    ///
    /// let response = client.get("http://example.com")
    ///     .send()
    ///     .await?;
    ///
    /// let response = response.text().await?;
    /// ```
    #[rune::function(instance)]
    fn get(&self, url: &str) -> RequestBuilder {
        RequestBuilder {
            request: self.client.get(url),
        }
    }

    /// Construct a builder to POST to the given `url`.
    ///
    /// # Examples
    ///
    /// ```rune,no_run
    /// let client = http::Client::new();
    ///
    /// let response = client.post("https://postman-echo.com/post")
    ///     .body_bytes(b"My post data...")
    ///     .send()
    ///     .await?;
    ///
    /// let response = response.json().await?;
    /// ```
    #[rune::function(instance)]
    fn post(&self, url: &str) -> RequestBuilder {
        let request = self.client.post(url);
        RequestBuilder { request }
    }

    /// Construct a builder to PUT to the given `url`.
    ///
    /// # Examples
    ///
    /// ```rune,no_run
    /// let client = http::Client::new();
    ///
    /// let response = client.put("https://postman-echo.com/put")
    ///     .body_bytes(b"My put data...")
    ///     .send()
    ///     .await?;
    ///
    /// let response = response.json().await?;
    /// ```
    #[rune::function(instance)]
    fn put(&self, url: &str) -> RequestBuilder {
        let request = self.client.put(url);
        RequestBuilder { request }
    }

    /// Construct a builder to PATCH to the given `url`.
    ///
    /// # Examples
    ///
    /// ```rune,no_run
    /// let client = http::Client::new();
    ///
    /// let response = client.patch("https://postman-echo.com/patch")
    ///     .body_bytes(b"My patch data...")
    ///     .send()
    ///     .await?;
    ///
    /// let response = response.json().await?;
    /// ```
    #[rune::function(instance)]
    fn patch(&self, url: &str) -> RequestBuilder {
        let request = self.client.patch(url);
        RequestBuilder { request }
    }

    /// Construct a builder to DELETE to the given `url`.
    ///
    /// # Examples
    ///
    /// ```rune,no_run
    /// let client = http::Client::new();
    ///
    /// let response = client.delete("https://postman-echo.com/delete")
    ///     .body_bytes(b"My delete data...")
    ///     .send()
    ///     .await?;
    ///
    /// let response = response.json().await?;
    /// ```
    #[rune::function(instance)]
    fn delete(&self, url: &str) -> RequestBuilder {
        let request = self.client.delete(url);
        RequestBuilder { request }
    }

    /// Construct a builder to HEAD to the given `url`.
    ///
    /// # Examples
    ///
    /// ```rune,no_run
    /// let client = http::Client::new();
    ///
    /// let response = client.head("https://postman-echo.com/head")
    ///     .body_bytes(b"My head data...")
    ///     .send()
    ///     .await?;
    ///
    /// let response = response.json().await?;
    /// ```
    #[rune::function(instance)]
    fn head(&self, url: &str) -> RequestBuilder {
        let request = self.client.head(url);
        RequestBuilder { request }
    }
}

/// Shorthand for generating a get request.
///
/// # Examples
///
/// ```rune,no_run
/// let response = http::get("http://worldtimeapi.org/api/ip").await?;
/// let json = response.json().await?;
///
/// let timezone = json["timezone"];
/// ```
#[rune::function]
async fn get(url: Ref<str>) -> Result<Response, Error> {
    Ok(Response {
        response: reqwest::get(url.as_ref()).await?,
    })
}
