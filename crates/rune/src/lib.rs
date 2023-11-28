//! <img alt="rune logo" src="https://raw.githubusercontent.com/rune-rs/rune/main/assets/icon.png" />
//! <br>
//! <a href="https://github.com/rune-rs/rune"><img alt="github" src="https://img.shields.io/badge/github-rune--rs/rune-8da0cb?style=for-the-badge&logo=github" height="20"></a>
//! <a href="https://crates.io/crates/rune"><img alt="crates.io" src="https://img.shields.io/crates/v/rune.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20"></a>
//! <a href="https://docs.rs/rune"><img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-rune-66c2a5?style=for-the-badge&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20"></a>
//! <a href="https://discord.gg/v5AeNkT"><img alt="chat on discord" src="https://img.shields.io/discord/558644981137670144.svg?logo=discord&style=flat-square" height="20"></a>
//! <br>
//! Minimum support: Rust <b>1.70+</b>.
//! <br>
//! <br>
//! <a href="https://rune-rs.github.io"><b>Visit the site 🌐</b></a>
//! &mdash;
//! <a href="https://rune-rs.github.io/book/"><b>Read the book 📖</b></a>
//! <br>
//! <br>
//!
//! The Rune Language, an embeddable dynamic programming language for Rust.
//!
//! <br>
//!
//! ## Contributing
//!
//! If you want to help out, please have a look at [Open Issues].
//!
//! <br>
//!
//! ## Highlights of Rune
//!
//! * Runs a compact representation of the language on top of an efficient
//!   [stack-based virtual machine][support-virtual-machine].
//! * Clean [Rust integration 💻][support-rust-integration].
//! * Memory safe through [reference counting 📖][support-reference-counted].
//! * [Awesome macros 📖][support-macros].
//! * [Template literals 📖][support-templates].
//! * [Try operators 📖][support-try].
//! * [Pattern matching 📖][support-patterns].
//! * [Structs and enums 📖][support-structs] with associated data and
//!   functions.
//! * Dynamic [vectors 📖][support-dynamic-vectors], [objects
//!   📖][support-anon-objects], and [tuples 📖][support-anon-tuples] with
//!   out-of-the-box [serde support 💻][support-serde].
//! * First-class [async support 📖][support-async].
//! * [Generators 📖][support-generators].
//! * Dynamic [instance functions 📖][support-instance-functions].
//! * [Stack isolation 📖][support-stack-isolation] between function calls.
//!
//! <br>
//!
//! ## Rune scripts
//!
//! You can run Rune programs with the bundled CLI:
//!
//! ```text
//! cargo run --bin rune -- run scripts/hello_world.rn
//! ```
//!
//! If you want to see detailed diagnostics of your program while it's running,
//! you can use:
//!
//! ```text
//! cargo run --bin rune -- run scripts/hello_world.rn --dump-unit --trace --dump-vm
//! ```
//!
//! See `--help` for more information.
//!
//! <br>
//!
//! ## Running scripts from Rust
//!
//! > You can find more examples [in the `examples` folder].
//!
//! The following is a complete example, including rich diagnostics using
//! [`termcolor`]. It can be made much simpler if this is not needed.
//!
//! [`termcolor`]: https://docs.rs/termcolor
//!
//! ```rust
//! use rune::{Context, Diagnostics, Source, Sources, Vm};
//! use rune::termcolor::{ColorChoice, StandardStream};
//! use std::sync::Arc;
//!
//! let context = Context::with_default_modules()?;
//! let runtime = Arc::new(context.runtime()?);
//!
//! let mut sources = Sources::new();
//! sources.insert(Source::memory("pub fn add(a, b) { a + b }")?);
//!
//! let mut diagnostics = Diagnostics::new();
//!
//! let result = rune::prepare(&mut sources)
//!     .with_context(&context)
//!     .with_diagnostics(&mut diagnostics)
//!     .build();
//!
//! if !diagnostics.is_empty() {
//!     let mut writer = StandardStream::stderr(ColorChoice::Always);
//!     diagnostics.emit(&mut writer, &sources)?;
//! }
//!
//! let unit = result?;
//! let mut vm = Vm::new(runtime, Arc::new(unit));
//!
//! let output = vm.call(["add"], (10i64, 20i64))?;
//! let output: i64 = rune::from_value(output)?;
//!
//! println!("{}", output);
//! # Ok::<_, rune::support::Error>(())
//! ```
//!
//! [in the `examples` folder]: https://github.com/rune-rs/rune/tree/main/examples/examples
//! [Open Issues]: https://github.com/rune-rs/rune/issues
//! [support-anon-objects]: https://rune-rs.github.io/book/objects.html
//! [support-anon-tuples]: https://rune-rs.github.io/book/tuples.html
//! [support-async]: https://rune-rs.github.io/book/async.html
//! [support-dynamic-vectors]: https://rune-rs.github.io/book/vectors.html
//! [support-generators]: https://rune-rs.github.io/book/generators.html
//! [support-instance-functions]: https://rune-rs.github.io/book/instance_functions.html
//! [support-macros]: https://rune-rs.github.io/book/macros.html
//! [support-patterns]: https://rune-rs.github.io/book/pattern_matching.html
//! [support-reference-counted]: https://rune-rs.github.io/book/variables.html
//! [support-rust-integration]: https://github.com/rune-rs/rune/tree/main/crates/rune-modules
//! [support-serde]: https://github.com/rune-rs/rune/blob/main/crates/rune-modules/src/json.rs
//! [support-stack-isolation]: https://rune-rs.github.io/book/call_frames.html
//! [support-structs]: https://rune-rs.github.io/book/structs.html
//! [support-templates]: https://rune-rs.github.io/book/template_literals.html
//! [support-try]: https://rune-rs.github.io/book/try_operator.html
//! [support-virtual-machine]: https://rune-rs.github.io/book/the_stack.html

#![no_std]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_doc_tests)]
#![cfg_attr(rune_nightly, feature(rustdoc_missing_doc_code_examples))]
#![cfg_attr(rune_nightly, deny(rustdoc::missing_doc_code_examples))]
#![allow(clippy::enum_variant_names)]
#![allow(clippy::needless_doctest_main)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::branches_sharing_code)]
#![allow(clippy::match_like_matches_macro)]
#![allow(clippy::type_complexity)]
#![allow(clippy::module_inception)]
#![allow(clippy::self_named_constructors)]
#![cfg_attr(rune_docsrs, feature(doc_cfg))]

#[cfg(feature = "std")]
#[macro_use]
extern crate std;

// This is here for forward compatibility when we can support allocation-free
// execution.
#[cfg(not(feature = "alloc"))]
compile_error!("The `alloc` feature is currently required to build rune, but will change for parts of rune in the future.");

#[macro_use]
extern crate alloc as rust_alloc;

/// A macro that can be used to construct a [Span][crate::ast::Span] that can be
/// pattern matched over.
///
/// # Examples
///
/// ```
/// use rune::ast::Span;
/// use rune::span;
///
/// let span = Span::new(42, 84);
/// assert!(matches!(span, span!(42, 84)));
/// ```
#[macro_export]
#[doc(hidden)]
macro_rules! span {
    ($start:expr, $end:expr) => {
        $crate::ast::Span {
            start: $crate::ast::ByteIndex($start),
            end: $crate::ast::ByteIndex($end),
        }
    };
}

pub mod alloc;

#[macro_use]
#[cfg(test)]
pub(crate) mod testing;

/// Helper prelude for #[no_std] support.
pub mod no_std;

#[macro_use]
mod internal_macros;

#[macro_use]
mod exported_macros;

#[macro_use]
pub mod ast;

#[cfg(feature = "fmt")]
pub(crate) mod fmt;

cfg_emit! {
    pub use ::codespan_reporting::term::termcolor;
}

mod any;
pub use self::any::Any;

mod build;
pub use self::build::{prepare, Build, BuildError};

pub mod compile;
#[doc(inline)]
pub use self::compile::{Context, ContextError, Options};

pub mod module;
#[doc(inline)]
pub use self::module::module::Module;

pub mod diagnostics;
#[doc(inline)]
pub use self::diagnostics::Diagnostics;

mod hash;
pub use self::hash::{Hash, ToTypeHash};

#[cfg(feature = "alloc")]
mod hashbrown;

mod params;
pub use self::params::Params;

mod hir;

mod indexing;

pub mod macros;

pub mod modules;

pub mod parse;

pub mod query;

pub mod runtime;
pub use self::runtime::{from_value, to_value, FromValue, ToValue, Unit, Value, Vm};

mod shared;

pub mod source;
pub use self::source::Source;

#[macro_use]
mod sources;
pub use self::sources::{SourceId, Sources};

mod worker;

#[doc(hidden)]
pub mod support;

cfg_workspace! {
    pub mod workspace;
}

// Macros used internally and re-exported.
pub(crate) use rune_macros::__internal_impl_any;

/// Macro used to annotate native functions which can be loaded into rune.
///
/// This macro automatically performs the following things:
/// * Rust documentation comments are captured so that it can be used in
///   generated Rune documentation.
/// * The name of arguments is captured to improve documentation generation.
/// * If an instance function is annotated this is detected (if the function
///   receives `self`). This behavior can be forced using
///   `#[rune::function(instance)]` if the function doesn't take `self`.
/// * The name of the function can be set using the `#[rune::function(path =
///   name)]` argument.
/// * An associated function can be specified with the `#[rune::function(path =
///   Type::name)]` argument. If `instance` is specified it is an associated
///   instance function that can be defined externally.
/// * Instance functions can be made a protocol function
///   `#[rune::function(protocol = STRING_DISPLAY)]`.
///
/// # Instance and associated functions
///
/// Instance and associated functions are a bit tricky to declare using
/// `#[rune::function]`, and care must be taken that you understand what needs
/// to be done. So this section is dedicated to documenting the ins and outs of
/// the process.
///
/// Briefly we should mention that instance functions are functions which are
/// associated with a type at runtime. Calling a value like `value.hello()`
/// invokes the `hello` associated function through the instance of `value`. The
/// exact type of `value` will then be used to look up which function to call.
/// They must take some kind of `self` parameter. Meanwhile associated functions
/// are just functions which are associated with a static type. Like
/// `String::new()`. The type `String` must then be in scope, and the function
/// does not take a `self` parameter.
///
/// This is how you declare an instance function which takes `&self` or `&mut
/// self`:
///
/// ```rust
/// # use rune::Any;
/// #[derive(Any)]
/// struct Struct {
///     /* .. */
/// }
///
/// impl Struct {
///     /// Get the length of the `Struct`.
///     #[rune::function]
///     fn len(&self) -> usize {
///         /* .. */
///         # todo!()
///     }
/// }
/// ```
///
/// If a function does not take `&self` or `&mut self`, you must specify that
/// it's an instance function using `#[rune::function(instance)]`. The first
/// argument is then considered the instance the function gets associated with:
///
/// ```rust
/// # use rune::Any;
/// #[derive(Any)]
/// struct Struct {
///     /* .. */
/// }
///
/// /// Get the length of the `Struct`.
/// #[rune::function(instance)]
/// fn len(this: &Struct) -> usize {
///     /* .. */
///     # todo!()
/// }
/// ```
///
/// To declare an associated function which does not receive the type we
/// must specify the path to the function using `#[rune::function(path =
/// Self::<name>)]`:
///
/// ```rust
/// # use rune::Any;
/// #[derive(Any)]
/// struct Struct {
///     /* .. */
/// }
///
/// impl Struct {
///     /// Construct a new [`Struct`].
///     #[rune::function(path = Self::new)]
///     fn new() -> Struct {
///         Struct {
///            /* .. */
///         }
///     }
/// }
/// ```
///
/// Or externally like this:
///
/// ```rust
/// # use rune::Any;
/// #[derive(Any)]
/// struct Struct {
///     /* .. */
/// }
///
/// /// Construct a new [`Struct`].
/// #[rune::function(free, path = Struct::new)]
/// fn new() -> Struct {
///     Struct {
///        /* .. */
///     }
/// }
/// ```
///
/// The first part `Struct` in `Struct::new` is used to determine the type
/// the function is associated with.
///
/// Protocol functions can either be defined in an impl block or externally. To
/// define a protocol externally, you can simply do this:
///
/// ```rust
/// # use rune::Any;
/// # use rune::runtime::{Formatter, VmResult};
/// #[derive(Any)]
/// struct Struct {
///     /* .. */
/// }
///
/// #[rune::function(instance, protocol = STRING_DISPLAY)]
/// fn string_display(this: &Struct, f: &mut Formatter) -> VmResult<()> {
///     /* .. */
///     # todo!()
/// }
/// ```
///
/// # Examples
///
/// Defining and using a simple free function:
///
/// ```
/// use rune::{Module, ContextError};
///
/// /// This is a pretty neat function which is called `std::str::to_uppercase("hello")`.
/// #[rune::function]
/// fn to_uppercase(string: &str) -> String {
///     string.to_uppercase()
/// }
///
/// fn module() -> Result<Module, ContextError> {
///     let mut m = Module::new();
///     m.function_meta(to_uppercase)?;
///     Ok(m)
/// }
/// ```
///
/// A free instance function:
///
/// ```
/// use rune::{Module, ContextError};
///
/// /// This is a pretty neat function, which is called like `"hello".to_uppercase()`.
/// #[rune::function(instance)]
/// fn to_uppercase(string: &str) -> String {
///     string.to_uppercase()
/// }
///
/// /// This is a pretty neat function, which is called like `string::to_uppercase2("hello")`.
/// #[rune::function(path = string)]
/// fn to_uppercase2(string: &str) -> String {
///     string.to_uppercase()
/// }
///
/// fn module() -> Result<Module, ContextError> {
///     let mut m = Module::new();
///     m.function_meta(to_uppercase)?;
///     m.function_meta(to_uppercase2)?;
///     Ok(m)
/// }
/// ```
///
/// Regular instance and protocol functions:
///
/// ```
/// use rune::{Any, Module, ContextError};
/// use rune::vm_write;
/// use rune::runtime::{Formatter, VmResult};
/// use rune::alloc::fmt::TryWrite;
///
/// #[derive(Any)]
/// struct String {
///     inner: std::string::String
/// }
///
/// impl String {
///     /// Construct a new string wrapper.
///     #[rune::function(path = Self::new)]
///     fn new(string: &str) -> Self {
///         Self {
///             inner: string.into()
///         }
///     }
///
///     /// Uppercase the string inside of the string wrapper.
///     ///
///     /// # Examples
///     ///
///     /// ```rune
///     /// let string = String::new("hello");
///     /// assert_eq!(string.to_uppercase(), "HELLO");
///     /// ```
///     #[rune::function]
///     fn to_uppercase(&self) -> String {
///         String {
///             inner: self.inner.to_uppercase()
///         }
///     }
///
///     /// Display the string using the [`STRING_DISPLAY`] protocol.
///     ///
///     /// # Examples
///     ///
///     /// ```rune
///     /// let string = String::new("hello");
///     /// assert_eq!(format!("{}", string), "hello");
///     /// ```
///     #[rune::function(protocol = STRING_DISPLAY)]
///     fn display(&self, f: &mut Formatter) -> VmResult<()> {
///         vm_write!(f, "{}", self.inner);
///         VmResult::Ok(())
///     }
/// }
///
/// /// Construct a new empty string.
/// ///
/// /// # Examples
/// ///
/// /// ```rune
/// /// let string = String::empty();
/// /// assert_eq!(string, "hello");
/// /// ```
/// #[rune::function(free, path = String::empty)]
/// fn empty() -> String {
///     String {
///         inner: std::string::String::new()
///     }
/// }
///
/// /// Lowercase the string inside of the string wrapper.
/// ///
/// /// # Examples
/// ///
/// /// ```rune
/// /// let string = String::new("Hello");
/// /// assert_eq!(string.to_lowercase(), "hello");
/// /// ```
/// #[rune::function(instance)]
/// fn to_lowercase(this: &String) -> String {
///     String {
///         inner: this.inner.to_lowercase()
///     }
/// }
///
/// fn module() -> Result<Module, ContextError> {
///     let mut m = Module::new();
///     m.ty::<String>()?;
///     m.function_meta(String::new)?;
///     m.function_meta(empty)?;
///     m.function_meta(String::to_uppercase)?;
///     m.function_meta(to_lowercase)?;
///     m.function_meta(String::display)?;
///     Ok(m)
/// }
/// ```
///
/// # Using `vm_result` and `<expr>.vm?`.
///
/// In order to conveniently deal with virtual machine errors which require use
/// [`VmResult`] this attribute macro supports the `vm_result` option.
///
/// This changes the return value of the function to be [`VmResult`], and
/// ensures that any try operator use is wrapped as appropriate. The special
/// operator `<expr>.vm?` is also supported in this context, which is a
/// shorthand for the [`vm_try!`] macro.
///
/// ```
/// use rune::alloc::String;
/// use rune::alloc::prelude::*;
///
/// #[rune::function(vm_result)]
/// fn trim(string: &str) -> String {
///     string.trim().try_to_owned().vm?
/// }
/// ```
///
/// This can be combined with regular uses of the try operator `?`:
///
/// ```
/// use core::str::Utf8Error;
///
/// use rune::alloc::String;
/// use rune::alloc::prelude::*;
///
/// #[rune::function(vm_result)]
/// fn trim_bytes(bytes: &[u8]) -> Result<String, Utf8Error> {
///     Ok(core::str::from_utf8(bytes)?.trim().try_to_owned().vm?)
/// }
/// ```
///
/// [`VmResult`]: crate::runtime::VmResult
/// [`vm_try!`]: crate::vm_try!
pub use rune_macros::function;

/// Macro used to annotate native functions which can be loaded as macros in
/// rune.
///
/// See [`Module::macro_meta`].
#[doc(hidden)]
pub use rune_macros::macro_;

/// Macro used to annotate native functions which can be loaded as attribute
/// macros in rune.
///
/// See [`Module::macro_meta`].
#[doc(hidden)]
pub use rune_macros::attribute_macro;

/// Macro used to annotate a module with metadata.
pub use rune_macros::module;

cfg_cli! {
    pub mod cli;
}

#[cfg(feature = "languageserver")]
pub mod languageserver;

cfg_doc! {
    pub mod doc;
}

/// Privately exported details.
#[doc(hidden)]
pub mod __private {
    pub use crate::compile::ItemBuf;
    pub use crate::module::module::Module;
    pub use crate::module::{
        FunctionMetaData, FunctionMetaKind, InstallWith, MacroMetaData, MacroMetaKind,
        ModuleMetaData,
    };
    pub use crate::params::Params;
    pub use crate::runtime::TypeOf;
    pub use rust_alloc::boxed::Box;
}

#[cfg(test)]
mod tests;
