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

use core::hash::Hash;
use core::cmp::Ordering;

use rune::alloc::fmt::TryWrite;
use rune::runtime::{Bytes, Formatter, Hasher, Ref, VmResult};
use rune::{docstring, item, Any, ContextError, Module, Value, ToConstValue};
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

    module.function_meta(get)?;

    module.ty::<Client>()?;
    module.function_meta(Client::new__meta)?;
    module.function_meta(Client::get__meta)?;
    module.function_meta(Client::post__meta)?;
    module.function_meta(Client::put__meta)?;
    module.function_meta(Client::delete__meta)?;
    module.function_meta(Client::head__meta)?;

    module.ty::<Response>()?;
    module.function_meta(Response::text__meta)?;
    module.function_meta(Response::json__meta)?;
    module.function_meta(Response::bytes__meta)?;
    module.function_meta(Response::status__meta)?;
    module.function_meta(Response::version__meta)?;
    module.function_meta(Response::content_length__meta)?;

    module.ty::<RequestBuilder>()?;
    module.function_meta(RequestBuilder::send__meta)?;
    module.function_meta(RequestBuilder::header__meta)?;
    module.function_meta(RequestBuilder::basic_auth__meta)?;
    module.function_meta(RequestBuilder::bearer_auth__meta)?;
    module.function_meta(RequestBuilder::fetch_mode_no_cors__meta)?;
    module.function_meta(RequestBuilder::body_bytes__meta)?;

    module.ty::<StatusCode>()?;
    module.function_meta(StatusCode::as_u16__meta)?;
    module.function_meta(StatusCode::as_str__meta)?;
    module.function_meta(StatusCode::canonical_reason__meta)?;
    module.function_meta(StatusCode::is_informational__meta)?;
    module.function_meta(StatusCode::is_success__meta)?;
    module.function_meta(StatusCode::is_redirection__meta)?;
    module.function_meta(StatusCode::is_client_error__meta)?;
    module.function_meta(StatusCode::is_server_error__meta)?;
    module.function_meta(StatusCode::partial_eq__meta)?;
    module.implement_trait::<StatusCode>(item!(::std::cmp::PartialEq))?;
    module.function_meta(StatusCode::eq__meta)?;
    module.implement_trait::<StatusCode>(item!(::std::cmp::Eq))?;
    module.function_meta(StatusCode::partial_cmp__meta)?;
    module.implement_trait::<StatusCode>(item!(::std::cmp::PartialOrd))?;
    module.function_meta(StatusCode::cmp__meta)?;
    module.implement_trait::<StatusCode>(item!(::std::cmp::Ord))?;
    module.function_meta(StatusCode::hash__meta)?;
    module.function_meta(StatusCode::string_debug__meta)?;
    module.function_meta(StatusCode::string_display__meta)?;



    module
        .constant(
            "CONTINUE",
            StatusCode {
                inner: reqwest::StatusCode::CONTINUE,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Continue
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::CONTINUE;
            /// ```
        })?;

    module
        .constant(
            "SWITCHING_PROTOCOLS",
            StatusCode {
                inner: reqwest::StatusCode::SWITCHING_PROTOCOLS,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Switching Protocols
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::SWITCHING_PROTOCOLS;
            /// ```
        })?;

    module
        .constant(
            "PROCESSING",
            StatusCode {
                inner: reqwest::StatusCode::PROCESSING,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Processing
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::PROCESSING;
            /// ```
        })?;

    module
        .constant(
            "OK",
            StatusCode {
                inner: reqwest::StatusCode::OK,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: OK
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::OK;
            /// ```
        })?;

    module
        .constant(
            "CREATED",
            StatusCode {
                inner: reqwest::StatusCode::CREATED,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Created
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::CREATED;
            /// ```
        })?;

    module
        .constant(
            "ACCEPTED",
            StatusCode {
                inner: reqwest::StatusCode::ACCEPTED,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Accepted
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::ACCEPTED;
            /// ```
        })?;

    module
        .constant(
            "NON_AUTHORITATIVE_INFORMATION",
            StatusCode {
                inner: reqwest::StatusCode::NON_AUTHORITATIVE_INFORMATION,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Non Authoritative Information
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::NON_AUTHORITATIVE_INFORMATION;
            /// ```
        })?;

    module
        .constant(
            "NO_CONTENT",
            StatusCode {
                inner: reqwest::StatusCode::NO_CONTENT,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: No Content
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::NO_CONTENT;
            /// ```
        })?;

    module
        .constant(
            "RESET_CONTENT",
            StatusCode {
                inner: reqwest::StatusCode::RESET_CONTENT,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Reset Content
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::RESET_CONTENT;
            /// ```
        })?;

    module
        .constant(
            "PARTIAL_CONTENT",
            StatusCode {
                inner: reqwest::StatusCode::PARTIAL_CONTENT,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Partial Content
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::PARTIAL_CONTENT;
            /// ```
        })?;

    module
        .constant(
            "MULTI_STATUS",
            StatusCode {
                inner: reqwest::StatusCode::MULTI_STATUS,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Multi-Status
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::MULTI_STATUS;
            /// ```
        })?;

    module
        .constant(
            "ALREADY_REPORTED",
            StatusCode {
                inner: reqwest::StatusCode::ALREADY_REPORTED,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Already Reported
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::ALREADY_REPORTED;
            /// ```
        })?;

    module
        .constant(
            "IM_USED",
            StatusCode {
                inner: reqwest::StatusCode::IM_USED,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: IM Used
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::IM_USED;
            /// ```
        })?;

    module
        .constant(
            "MULTIPLE_CHOICES",
            StatusCode {
                inner: reqwest::StatusCode::MULTIPLE_CHOICES,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Multiple Choices
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::MULTIPLE_CHOICES;
            /// ```
        })?;

    module
        .constant(
            "MOVED_PERMANENTLY",
            StatusCode {
                inner: reqwest::StatusCode::MOVED_PERMANENTLY,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Moved Permanently
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::MOVED_PERMANENTLY;
            /// ```
        })?;

    module
        .constant(
            "FOUND",
            StatusCode {
                inner: reqwest::StatusCode::FOUND,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Found
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::FOUND;
            /// ```
        })?;

    module
        .constant(
            "SEE_OTHER",
            StatusCode {
                inner: reqwest::StatusCode::SEE_OTHER,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: See Other
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::SEE_OTHER;
            /// ```
        })?;

    module
        .constant(
            "NOT_MODIFIED",
            StatusCode {
                inner: reqwest::StatusCode::NOT_MODIFIED,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Not Modified
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::NOT_MODIFIED;
            /// ```
        })?;

    module
        .constant(
            "USE_PROXY",
            StatusCode {
                inner: reqwest::StatusCode::USE_PROXY,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Use Proxy
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::USE_PROXY;
            /// ```
        })?;

    module
        .constant(
            "TEMPORARY_REDIRECT",
            StatusCode {
                inner: reqwest::StatusCode::TEMPORARY_REDIRECT,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Temporary Redirect
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::TEMPORARY_REDIRECT;
            /// ```
        })?;

    module
        .constant(
            "PERMANENT_REDIRECT",
            StatusCode {
                inner: reqwest::StatusCode::PERMANENT_REDIRECT,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Permanent Redirect
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::PERMANENT_REDIRECT;
            /// ```
        })?;

    module
        .constant(
            "BAD_REQUEST",
            StatusCode {
                inner: reqwest::StatusCode::BAD_REQUEST,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Bad Request
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::BAD_REQUEST;
            /// ```
        })?;

    module
        .constant(
            "UNAUTHORIZED",
            StatusCode {
                inner: reqwest::StatusCode::UNAUTHORIZED,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Unauthorized
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::UNAUTHORIZED;
            /// ```
        })?;

    module
        .constant(
            "PAYMENT_REQUIRED",
            StatusCode {
                inner: reqwest::StatusCode::PAYMENT_REQUIRED,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Payment Required
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::PAYMENT_REQUIRED;
            /// ```
        })?;

    module
        .constant(
            "FORBIDDEN",
            StatusCode {
                inner: reqwest::StatusCode::FORBIDDEN,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Forbidden
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::FORBIDDEN;
            /// ```
        })?;

    module
        .constant(
            "NOT_FOUND",
            StatusCode {
                inner: reqwest::StatusCode::NOT_FOUND,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Not Found
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::NOT_FOUND;
            /// ```
        })?;

    module
        .constant(
            "METHOD_NOT_ALLOWED",
            StatusCode {
                inner: reqwest::StatusCode::METHOD_NOT_ALLOWED,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Method Not Allowed
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::METHOD_NOT_ALLOWED;
            /// ```
        })?;

    module
        .constant(
            "NOT_ACCEPTABLE",
            StatusCode {
                inner: reqwest::StatusCode::NOT_ACCEPTABLE,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Not Acceptable
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::NOT_ACCEPTABLE;
            /// ```
        })?;

    module
        .constant(
            "PROXY_AUTHENTICATION_REQUIRED",
            StatusCode {
                inner: reqwest::StatusCode::PROXY_AUTHENTICATION_REQUIRED,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Proxy Authentication Required
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::PROXY_AUTHENTICATION_REQUIRED;
            /// ```
        })?;

    module
        .constant(
            "REQUEST_TIMEOUT",
            StatusCode {
                inner: reqwest::StatusCode::REQUEST_TIMEOUT,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Request Timeout
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::REQUEST_TIMEOUT;
            /// ```
        })?;

    module
        .constant(
            "CONFLICT",
            StatusCode {
                inner: reqwest::StatusCode::CONFLICT,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Conflict
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::CONFLICT;
            /// ```
        })?;

    module
        .constant(
            "GONE",
            StatusCode {
                inner: reqwest::StatusCode::GONE,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Gone
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::GONE;
            /// ```
        })?;

    module
        .constant(
            "LENGTH_REQUIRED",
            StatusCode {
                inner: reqwest::StatusCode::LENGTH_REQUIRED,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Length Required
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::LENGTH_REQUIRED;
            /// ```
        })?;

    module
        .constant(
            "PRECONDITION_FAILED",
            StatusCode {
                inner: reqwest::StatusCode::PRECONDITION_FAILED,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Precondition Failed
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::PRECONDITION_FAILED;
            /// ```
        })?;

    module
        .constant(
            "PAYLOAD_TOO_LARGE",
            StatusCode {
                inner: reqwest::StatusCode::PAYLOAD_TOO_LARGE,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Payload Too Large
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::PAYLOAD_TOO_LARGE;
            /// ```
        })?;

    module
        .constant(
            "URI_TOO_LONG",
            StatusCode {
                inner: reqwest::StatusCode::URI_TOO_LONG,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: URI Too Long
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::URI_TOO_LONG;
            /// ```
        })?;

    module
        .constant(
            "UNSUPPORTED_MEDIA_TYPE",
            StatusCode {
                inner: reqwest::StatusCode::UNSUPPORTED_MEDIA_TYPE,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Unsupported Media Type
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::UNSUPPORTED_MEDIA_TYPE;
            /// ```
        })?;

    module
        .constant(
            "RANGE_NOT_SATISFIABLE",
            StatusCode {
                inner: reqwest::StatusCode::RANGE_NOT_SATISFIABLE,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Range Not Satisfiable
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::RANGE_NOT_SATISFIABLE;
            /// ```
        })?;

    module
        .constant(
            "EXPECTATION_FAILED",
            StatusCode {
                inner: reqwest::StatusCode::EXPECTATION_FAILED,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Expectation Failed
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::EXPECTATION_FAILED;
            /// ```
        })?;

    module
        .constant(
            "IM_A_TEAPOT",
            StatusCode {
                inner: reqwest::StatusCode::IM_A_TEAPOT,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: I'm a teapot
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::IM_A_TEAPOT;
            /// ```
        })?;

    module
        .constant(
            "MISDIRECTED_REQUEST",
            StatusCode {
                inner: reqwest::StatusCode::MISDIRECTED_REQUEST,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Misdirected Request
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::MISDIRECTED_REQUEST;
            /// ```
        })?;

    module
        .constant(
            "UNPROCESSABLE_ENTITY",
            StatusCode {
                inner: reqwest::StatusCode::UNPROCESSABLE_ENTITY,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Unprocessable Entity
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::UNPROCESSABLE_ENTITY;
            /// ```
        })?;

    module
        .constant(
            "LOCKED",
            StatusCode {
                inner: reqwest::StatusCode::LOCKED,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Locked
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::LOCKED;
            /// ```
        })?;

    module
        .constant(
            "FAILED_DEPENDENCY",
            StatusCode {
                inner: reqwest::StatusCode::FAILED_DEPENDENCY,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Failed Dependency
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::FAILED_DEPENDENCY;
            /// ```
        })?;

    module
        .constant(
            "UPGRADE_REQUIRED",
            StatusCode {
                inner: reqwest::StatusCode::UPGRADE_REQUIRED,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Upgrade Required
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::UPGRADE_REQUIRED;
            /// ```
        })?;

    module
        .constant(
            "PRECONDITION_REQUIRED",
            StatusCode {
                inner: reqwest::StatusCode::PRECONDITION_REQUIRED,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Precondition Required
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::PRECONDITION_REQUIRED;
            /// ```
        })?;

    module
        .constant(
            "TOO_MANY_REQUESTS",
            StatusCode {
                inner: reqwest::StatusCode::TOO_MANY_REQUESTS,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Too Many Requests
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::TOO_MANY_REQUESTS;
            /// ```
        })?;

    module
        .constant(
            "REQUEST_HEADER_FIELDS_TOO_LARGE",
            StatusCode {
                inner: reqwest::StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Request Header Fields Too Large
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE;
            /// ```
        })?;

    module
        .constant(
            "UNAVAILABLE_FOR_LEGAL_REASONS",
            StatusCode {
                inner: reqwest::StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Unavailable For Legal Reasons
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS;
            /// ```
        })?;

    module
        .constant(
            "INTERNAL_SERVER_ERROR",
            StatusCode {
                inner: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Internal Server Error
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::INTERNAL_SERVER_ERROR;
            /// ```
        })?;

    module
        .constant(
            "NOT_IMPLEMENTED",
            StatusCode {
                inner: reqwest::StatusCode::NOT_IMPLEMENTED,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Not Implemented
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::NOT_IMPLEMENTED;
            /// ```
        })?;

    module
        .constant(
            "BAD_GATEWAY",
            StatusCode {
                inner: reqwest::StatusCode::BAD_GATEWAY,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Bad Gateway
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::BAD_GATEWAY;
            /// ```
        })?;

    module
        .constant(
            "SERVICE_UNAVAILABLE",
            StatusCode {
                inner: reqwest::StatusCode::SERVICE_UNAVAILABLE,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Service Unavailable
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::SERVICE_UNAVAILABLE;
            /// ```
        })?;

    module
        .constant(
            "GATEWAY_TIMEOUT",
            StatusCode {
                inner: reqwest::StatusCode::GATEWAY_TIMEOUT,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Gateway Timeout
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::GATEWAY_TIMEOUT;
            /// ```
        })?;

    module
        .constant(
            "HTTP_VERSION_NOT_SUPPORTED",
            StatusCode {
                inner: reqwest::StatusCode::HTTP_VERSION_NOT_SUPPORTED,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: HTTP Version Not Supported
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::HTTP_VERSION_NOT_SUPPORTED;
            /// ```
        })?;

    module
        .constant(
            "VARIANT_ALSO_NEGOTIATES",
            StatusCode {
                inner: reqwest::StatusCode::VARIANT_ALSO_NEGOTIATES,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Variant Also Negotiates
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::VARIANT_ALSO_NEGOTIATES;
            /// ```
        })?;

    module
        .constant(
            "INSUFFICIENT_STORAGE",
            StatusCode {
                inner: reqwest::StatusCode::INSUFFICIENT_STORAGE,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Insufficient Storage
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::INSUFFICIENT_STORAGE;
            /// ```
        })?;

    module
        .constant(
            "LOOP_DETECTED",
            StatusCode {
                inner: reqwest::StatusCode::LOOP_DETECTED,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Loop Detected
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::LOOP_DETECTED;
            /// ```
        })?;

    module
        .constant(
            "NOT_EXTENDED",
            StatusCode {
                inner: reqwest::StatusCode::NOT_EXTENDED,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Not Extended
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::NOT_EXTENDED;
            /// ```
        })?;

    module
        .constant(
            "NETWORK_AUTHENTICATION_REQUIRED",
            StatusCode {
                inner: reqwest::StatusCode::NETWORK_AUTHENTICATION_REQUIRED,
            },
        )
        .build_associated::<StatusCode>()?
        .docs(docstring! {
            /// Status Code: Network Authentication Required
            ///
            /// # Examples
            ///
            /// ```rune
            /// use http::StatusCode;
            ///
            /// let status_code = StatusCode::NETWORK_AUTHENTICATION_REQUIRED;
            /// ```
        })?;

    module.ty::<Version>()?;
    module.function_meta(Version::partial_eq__meta)?;
    module.implement_trait::<Version>(item!(::std::cmp::PartialEq))?;
    module.function_meta(Version::eq__meta)?;
    module.implement_trait::<Version>(item!(::std::cmp::Eq))?;
    module.function_meta(Version::partial_cmp__meta)?;
    module.implement_trait::<Version>(item!(::std::cmp::PartialOrd))?;
    module.function_meta(Version::cmp__meta)?;
    module.implement_trait::<Version>(item!(::std::cmp::Ord))?;
    module.function_meta(Version::hash__meta)?;
    module.function_meta(Version::string_debug__meta)?;

    module
        .constant(
            "HTTP_09",
            Version {
                inner: reqwest::Version::HTTP_09,
            },
        )
        .build_associated::<Version>()?
        .docs(docstring! {
            /// The `HTTP/0.9` version.
            ///
            /// # Examples
            ///
            /// ```rune,no_run
            /// use http::Version;
            ///
            /// let version = Version::HTTP_09;
            /// ```
        })?;

    module
        .constant(
            "HTTP_10",
            Version {
                inner: reqwest::Version::HTTP_10,
            },
        )
        .build_associated::<Version>()?
        .docs(docstring! {
            /// The `HTTP/1.0` version.
            ///
            /// # Examples
            ///
            /// ```rune,no_run
            /// use http::Version;
            ///
            /// let version = Version::HTTP_10;
            /// ```
        })?;

    module
        .constant(
            "HTTP_11",
            Version {
                inner: reqwest::Version::HTTP_11,
            },
        )
        .build_associated::<Version>()?
        .docs(docstring! {
            /// The `HTTP/1.1` version.
            ///
            /// # Examples
            ///
            /// ```rune,no_run
            /// use http::Version;
            ///
            /// let version = Version::HTTP_11;
            /// ```
        })?;

    module
        .constant(
            "HTTP_2",
            Version {
                inner: reqwest::Version::HTTP_2,
            },
        )
        .build_associated::<Version>()?
        .docs(docstring! {
            /// The `HTTP/2.0` version.
            ///
            /// # Examples
            ///
            /// ```rune,no_run
            /// use http::Version;
            ///
            /// let version = Version::HTTP_2;
            /// ```
        })?;

    module
        .constant(
            "HTTP_3",
            Version {
                inner: reqwest::Version::HTTP_3,
            },
        )
        .build_associated::<Version>()?
        .docs(docstring! {
            /// The `HTTP/3.0` version.
            ///
            /// # Examples
            ///
            /// ```rune,no_run
            /// use http::Version;
            ///
            /// let version = Version::HTTP_3;
            /// ```
        })?;

    module.ty::<Error>()?;
    module.function_meta(Error::string_display__meta)?;
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
    /// Write a display representation the error.
    #[rune::function(keep, instance, protocol = STRING_DISPLAY)]
    fn string_display(&self, f: &mut Formatter) -> VmResult<()> {
        rune::vm_write!(f, "{}", self.inner)
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
    #[rune::function(keep, vm_result)]
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
    #[rune::function(keep)]
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
    #[rune::function(keep, vm_result)]
    async fn bytes(mut self) -> Result<Bytes, Error> {
        let len = self.response.content_length().unwrap_or(0) as usize;
        let mut bytes = Vec::try_with_capacity(len).vm?;

        while let Some(chunk) = self.response.chunk().await? {
            bytes.try_extend_from_slice(chunk.as_ref()).vm?;
        }

        Ok(Bytes::from_vec(bytes))
    }

    /// Get the status code of the response.
    #[rune::function(keep, instance)]
    fn status(&self) -> StatusCode {
        StatusCode { inner: self.response.status() }
    }

    /// Get the version of the response.
    #[rune::function(keep, instance)]
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
    #[rune::function(keep, instance)]
    fn content_length(&self) -> Option<u64> {
        self.response.content_length()
    }
}

/// An HTTP status code.
#[derive(Debug, Any, PartialEq, Eq, PartialOrd, Ord, ToConstValue)]
#[rune(item = ::http)]
pub struct StatusCode {
    #[const_value(with = self::const_status_code)]
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
    #[rune::function(keep, instance)]
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
    #[rune::function(keep, instance, vm_result)]
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
    #[rune::function(keep, instance)]
    fn canonical_reason(&self) -> Option<&'static str> {
        self.inner.canonical_reason()
    }

    /// Check if status is within 100-199.
    #[inline]
    #[rune::function(keep, instance)]
    fn is_informational(&self) -> bool {
        self.inner.is_informational()
    }

    /// Check if status is within 200-299.
    #[inline]
    #[rune::function(keep, instance)]
    fn is_success(&self) -> bool {
        self.inner.is_success()
    }

    /// Check if status is within 300-399.
    #[inline]
    #[rune::function(keep, instance)]
    fn is_redirection(&self) -> bool {
        self.inner.is_redirection()
    }

    /// Check if status is within 400-499.
    #[inline]
    #[rune::function(keep, instance)]
    fn is_client_error(&self) -> bool {
        self.inner.is_client_error()
    }

    /// Check if status is within 500-599.
    #[inline]
    #[rune::function(keep, instance)]
    fn is_server_error(&self) -> bool {
        self.inner.is_server_error()
    }

    /// Test two byte arrays for partial equality.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::partial_eq;
    ///
    /// use http::StatusCode;
    ///
    /// let ok = StatusCode::OK;
    /// let not_found = StatusCode::NOT_FOUND;
    ///
    /// assert_eq!(partial_eq(ok, ok), true);
    /// assert_eq!(partial_eq(ok, not_found), false);
    /// assert_eq!(partial_eq(not_found, ok), false);
    /// ```
    #[rune::function(keep, instance, protocol = PARTIAL_EQ)]
    #[inline]
    fn partial_eq(&self, rhs: &Self) -> bool {
        PartialEq::eq(&self.inner, &rhs.inner)
    }

    /// Test two byte arrays for total equality.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::eq;
    ///
    /// use http::StatusCode;
    ///
    /// let ok = StatusCode::OK;
    /// let not_found = StatusCode::NOT_FOUND;
    ///
    /// assert_eq!(eq(ok, ok), true);
    /// assert_eq!(eq(ok, not_found), false);
    /// assert_eq!(eq(not_found, ok), false);
    /// ```
    #[rune::function(keep, instance, protocol = EQ)]
    #[inline]
    fn eq(&self, rhs: &Self) -> bool {
        PartialEq::eq(&self.inner, &rhs.inner)
    }

    /// Perform a partial ordered comparison between two byte arrays.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use http::StatusCode;
    ///
    /// let ok = StatusCode::OK;
    /// let not_found = StatusCode::NOT_FOUND;
    ///
    /// assert!(ok < not_found);
    /// assert!(not_found > ok);
    /// assert!(ok == ok);
    /// ```
    ///
    /// Using explicit functions:
    ///
    /// ```rune
    /// use std::cmp::Ordering;
    /// use std::ops::partial_cmp;
    ///
    /// use http::StatusCode;
    ///
    /// let ok = StatusCode::OK;
    /// let not_found = StatusCode::NOT_FOUND;
    ///
    /// assert_eq!(partial_cmp(ok, not_found), Some(Ordering::Less));
    /// assert_eq!(partial_cmp(not_found, ok), Some(Ordering::Greater));
    /// assert_eq!(partial_cmp(ok, ok), Some(Ordering::Equal));
    /// ```
    #[rune::function(keep, instance, protocol = PARTIAL_CMP)]
    #[inline]
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        PartialOrd::partial_cmp(&self.inner, &rhs.inner)
    }

    /// Perform a totally ordered comparison between two byte arrays.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::cmp::Ordering;
    /// use std::ops::cmp;
    ///
    /// use http::StatusCode;
    ///
    /// let ok = StatusCode::OK;
    /// let not_found = StatusCode::NOT_FOUND;
    ///
    /// assert_eq!(cmp(ok, not_found), Ordering::Less);
    /// assert_eq!(cmp(not_found, ok), Ordering::Greater);
    /// assert_eq!(cmp(ok, ok), Ordering::Equal);
    /// ```
    #[rune::function(keep, instance, protocol = CMP)]
    #[inline]
    fn cmp(&self, rhs: &Self) -> Ordering {
        Ord::cmp(&self.inner, &rhs.inner)
    }

    /// Hash the string.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::hash;
    ///
    /// use http::StatusCode;
    ///
    /// let not_found = StatusCode::NOT_FOUND;
    ///
    /// assert_eq!(hash(not_found), hash(not_found));
    /// ```
    #[rune::function(keep, instance, protocol = HASH)]
    fn hash(&self, hasher: &mut Hasher) {
        self.inner.hash(hasher);
    }

    /// Write a debug representation of the status code.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use http::StatusCode;
    ///
    /// let not_found = StatusCode::NOT_FOUND;
    ///
    /// println!("{not_found:?}");
    /// ```
    #[rune::function(keep, instance, protocol = STRING_DEBUG)]
    fn string_debug(&self, f: &mut Formatter) -> VmResult<()> {
        rune::vm_write!(f, "{:?}", self.inner)
    }

    /// Write a display representation of the status code.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use http::StatusCode;
    ///
    /// let not_found = StatusCode::NOT_FOUND;
    ///
    /// println!("{not_found}");
    /// ```
    #[rune::function(keep, instance, protocol = STRING_DISPLAY)]
    fn string_display(&self, f: &mut Formatter) -> VmResult<()> {
        rune::vm_write!(f, "{}", self.inner)
    }
}

/// HTTP version
///
/// This module contains a definition of the `Version` type. The `Version`
/// type is intended to be accessed through the root of the crate
/// (`http::Version`) rather than this module.
///
/// The `Version` type contains constants that represent the various versions
/// of the HTTP protocol.
///
/// # Examples
///
/// ```rune
/// use http::Version;
///
/// let http11 = Version::HTTP_11;
/// let http2 = Version::HTTP_2;
/// assert!(http11 != http2);
///
/// println!("{:?}", http2);
/// ```
#[derive(Debug, Any, PartialEq, Eq, PartialOrd, Ord, ToConstValue)]
#[rune(item = ::http)]
pub struct Version {
    #[const_value(with = self::const_version)]
    inner: reqwest::Version,
}

impl Version {
    /// Test two byte arrays for partial equality.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::partial_eq;
    ///
    /// use http::Version;
    ///
    /// let http11 = Version::HTTP_11;
    /// let http2 = Version::HTTP_2;
    ///
    /// assert_eq!(partial_eq(http11, http11), true);
    /// assert_eq!(partial_eq(http11, http2), false);
    /// assert_eq!(partial_eq(http2, http11), false);
    /// ```
    #[rune::function(keep, instance, protocol = PARTIAL_EQ)]
    #[inline]
    fn partial_eq(&self, rhs: &Self) -> bool {
        PartialEq::eq(&self.inner, &rhs.inner)
    }

    /// Test two byte arrays for total equality.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::eq;
    ///
    /// use http::Version;
    ///
    /// let http11 = Version::HTTP_11;
    /// let http2 = Version::HTTP_2;
    ///
    /// assert_eq!(eq(http11, http11), true);
    /// assert_eq!(eq(http11, http2), false);
    /// assert_eq!(eq(http2, http11), false);
    /// ```
    #[rune::function(keep, instance, protocol = EQ)]
    #[inline]
    fn eq(&self, rhs: &Self) -> bool {
        PartialEq::eq(&self.inner, &rhs.inner)
    }

    /// Perform a partial ordered comparison between two byte arrays.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use http::Version;
    ///
    /// let http11 = Version::HTTP_11;
    /// let http2 = Version::HTTP_2;
    ///
    /// assert!(http11 < http2);
    /// assert!(http2 > http11);
    /// assert!(http11 == http11);
    /// ```
    ///
    /// Using explicit functions:
    ///
    /// ```rune
    /// use std::cmp::Ordering;
    /// use std::ops::partial_cmp;
    ///
    /// use http::Version;
    ///
    /// let http11 = Version::HTTP_11;
    /// let http2 = Version::HTTP_2;
    ///
    /// assert_eq!(partial_cmp(http11, http2), Some(Ordering::Less));
    /// assert_eq!(partial_cmp(http2, http11), Some(Ordering::Greater));
    /// assert_eq!(partial_cmp(http11, http11), Some(Ordering::Equal));
    /// ```
    #[rune::function(keep, instance, protocol = PARTIAL_CMP)]
    #[inline]
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        PartialOrd::partial_cmp(&self.inner, &rhs.inner)
    }

    /// Perform a totally ordered comparison between two byte arrays.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::cmp::Ordering;
    /// use std::ops::cmp;
    ///
    /// use http::Version;
    ///
    /// let http11 = Version::HTTP_11;
    /// let http2 = Version::HTTP_2;
    ///
    /// assert_eq!(cmp(http11, http2), Ordering::Less);
    /// assert_eq!(cmp(http2, http11), Ordering::Greater);
    /// assert_eq!(cmp(http11, http11), Ordering::Equal);
    /// ```
    #[rune::function(keep, instance, protocol = CMP)]
    #[inline]
    fn cmp(&self, rhs: &Self) -> Ordering {
        Ord::cmp(&self.inner, &rhs.inner)
    }

    /// Hash the string.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::hash;
    ///
    /// use http::Version;
    ///
    /// let http2 = Version::HTTP_2;
    ///
    /// assert_eq!(hash(http2), hash(http2));
    /// ```
    #[rune::function(keep, instance, protocol = HASH)]
    fn hash(&self, hasher: &mut Hasher) {
        self.inner.hash(hasher);
    }

    /// Debug print the Version.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use http::Version;
    ///
    /// let http11 = Version::HTTP_11;
    /// let http2 = Version::HTTP_2;
    ///
    /// println!("{:?}", http2);
    /// ```
    #[rune::function(keep, instance, protocol = STRING_DEBUG)]
    fn string_debug(&self, f: &mut Formatter) -> VmResult<()> {
        rune::vm_write!(f, "{:?}", self.inner)
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
    #[rune::function(keep)]
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
    #[rune::function(keep)]
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
    #[rune::function(keep)]
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
    #[rune::function(keep)]
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
    #[rune::function(keep)]
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
    #[rune::function(keep)]
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
    #[rune::function(keep, path = Self::new)]
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
    #[rune::function(keep, instance)]
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
    #[rune::function(keep, instance)]
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
    #[rune::function(keep, instance)]
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
    #[rune::function(keep, instance)]
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
    #[rune::function(keep, instance)]
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

mod const_version {
    use rune::runtime::{RuntimeError, ConstValue, Value};

    #[inline]
    pub(super) fn to_const_value(version: reqwest::Version) -> Result<ConstValue, RuntimeError> {
        match version {
            reqwest::Version::HTTP_09 => Ok(ConstValue::from(1i64)),
            reqwest::Version::HTTP_10 => Ok(ConstValue::from(2i64)),
            reqwest::Version::HTTP_11 => Ok(ConstValue::from(3i64)),
            reqwest::Version::HTTP_2 => Ok(ConstValue::from(4i64)),
            reqwest::Version::HTTP_3 => Ok(ConstValue::from(5i64)),
            version => Err(RuntimeError::panic(format!("Unsupported reqwest version {version:?}"))),
        }
    }

    #[inline]
    pub(super) fn from_const_value(version: &ConstValue) -> Result<reqwest::Version, RuntimeError> {
        let Some(value) = version.as_i64() else {
            return Err(RuntimeError::panic(format!("Unsupported reqwest version {version:?}")));
        };

        from_i64(value)
    }

    #[inline]
    pub(super) fn from_value(version: Value) -> Result<reqwest::Version, RuntimeError> {
        let value = version.as_integer()?;

        from_i64(value)
    }

    #[inline]
    fn from_i64(value: i64) -> Result<reqwest::Version, RuntimeError> {
        match value {
            1i64 => Ok(reqwest::Version::HTTP_09),
            2i64 => Ok(reqwest::Version::HTTP_10),
            3i64 => Ok(reqwest::Version::HTTP_11),
            4i64 => Ok(reqwest::Version::HTTP_2),
            5i64 => Ok(reqwest::Version::HTTP_3),
            value => Err(RuntimeError::panic(format!("unsupported reqwest version {value}"))),
        }
    }
}

mod const_status_code {
    use rune::runtime::{RuntimeError, ConstValue, Value};

    #[inline]
    pub(super) fn to_const_value(status: reqwest::StatusCode) -> Result<ConstValue, RuntimeError> {
        ConstValue::try_from(status.as_u16())
    }

    #[inline]
    pub(super) fn from_const_value(status: &ConstValue) -> Result<reqwest::StatusCode, RuntimeError> {
        let Some(value) = status.as_i64() else {
            return Err(RuntimeError::panic(format!("Unsupported reqwest status {status:?}")));
        };

        match reqwest::StatusCode::from_u16(value as u16) {
            Ok(status) => Ok(status),
            Err(error) => Err(RuntimeError::panic(error)),
        }
    }

    #[inline]
    pub(super) fn from_value(value: Value) -> Result<reqwest::StatusCode, RuntimeError> {
        match reqwest::StatusCode::from_u16(rune::from_value(value)?) {
            Ok(status) => Ok(status),
            Err(error) => Err(RuntimeError::panic(error)),
        }
    }
}
