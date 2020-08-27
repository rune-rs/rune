//! HTTP module for runestick based on reqwest.
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! runestick = "0.2"
//! runestick-http = "0.2"
//! # not necessary, but useful
//! runestick-json = "0.2"
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> runestick::Result<()> {
//! let mut context = runestick::Context::with_default_packages()?;
//! context.install(runestick_http::module()?)?;
//! context.install(runestick_json::module()?)?;
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

use runestick::Bytes;

#[derive(Debug)]
struct Client {
    client: reqwest::Client,
}

#[derive(Debug)]
pub struct Response {
    response: reqwest::Response,
}

impl Response {
    async fn text(self) -> runestick::Result<String> {
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
    async fn send(self) -> runestick::Result<Response> {
        let response = self.request.send().await?;
        Ok(Response { response })
    }

    async fn body_bytes(self, bytes: Bytes) -> runestick::Result<Self> {
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
    async fn get(&self, url: &str) -> runestick::Result<RequestBuilder> {
        let request = self.client.get(url);
        Ok(RequestBuilder { request })
    }

    /// Construct a builder to POST to the given URL.
    async fn post(&self, url: &str) -> runestick::Result<RequestBuilder> {
        let request = self.client.post(url);
        Ok(RequestBuilder { request })
    }
}

/// Shorthand for generating a get request.
async fn get(url: &str) -> runestick::Result<Response> {
    Ok(Response {
        response: reqwest::get(url).await?,
    })
}

runestick::decl_external!(Client);
runestick::decl_external!(Response);
runestick::decl_external!(RequestBuilder);

/// Construct the http library.
pub fn module() -> Result<runestick::Module, runestick::ContextError> {
    let mut module = runestick::Module::new(&["http"]);

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
