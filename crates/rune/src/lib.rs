//! <img alt="rune logo" src="https://raw.githubusercontent.com/rune-rs/rune/main/assets/icon.png" />
//! <br>
//! <a href="https://github.com/rune-rs/rune"><img alt="github" src="https://img.shields.io/badge/github-rune--rs/rune-8da0cb?style=for-the-badge&logo=github" height="20"></a>
//! <a href="https://crates.io/crates/rune"><img alt="crates.io" src="https://img.shields.io/crates/v/rune.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20"></a>
//! <a href="https://docs.rs/rune"><img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-rune-66c2a5?style=for-the-badge&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20"></a>
//! <a href="https://discord.gg/v5AeNkT"><img alt="chat on discord" src="https://img.shields.io/discord/558644981137670144.svg?logo=discord&style=flat-square" height="20"></a>
//! <br>
//! Minimum support: Rust <b>1.87+</b>.
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
//! * [Multithreaded 📖][support-multithreading] execution.
//! * [Hot reloading 📖][support-hot-reloading].
//! * Memory safe through [reference counting 📖][support-reference-counted].
//! * [Awesome macros 📖][support-macros] and [Template literals 📖][support-templates].
//! * [Try operators 📖][support-try] and [Pattern matching 📖][support-patterns].
//! * [Structs and enums 📖][support-structs] with associated data and
//!   functions.
//! * Dynamic containers like [vectors 📖][support-dynamic-vectors], [objects
//!   📖][support-anon-objects], and [tuples 📖][support-anon-tuples] all with
//!   out-of-the-box [serde support 💻][support-serde].
//! * First-class [async support 📖][support-async] with [Generators 📖][support-generators].
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
//! cargo run --bin rune -- run scripts/hello_world.rn --dump --trace
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
//! ```no_run
//! use rune::{Context, Diagnostics, Source, Sources, Vm};
//! use rune::termcolor::{ColorChoice, StandardStream};
//! use rune::sync::Arc;
//!
//! let context = Context::with_default_modules()?;
//!
//! let mut sources = Sources::new();
//! sources.insert(Source::memory("pub fn add(a, b) { a + b }")?);
//!
//! let mut diagnostics = Diagnostics::new();
//!
//! let result = rune::prepare(&mut sources)
//!     .with_context(&context)
//!     .with_diagnostics(&mut diagnostics)
//!     .build_vm();
//!
//! if !diagnostics.is_empty() {
//!     let mut writer = StandardStream::stderr(ColorChoice::Always);
//!     diagnostics.emit(&mut writer, &sources)?;
//! }
//!
//! let mut vm = result?;
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
//! [support-hot-reloading]: https://rune-rs.github.io/book/hot_reloading.html
//! [support-instance-functions]: https://rune-rs.github.io/book/instance_functions.html
//! [support-macros]: https://rune-rs.github.io/book/macros.html
//! [support-multithreading]: https://rune-rs.github.io/book/multithreading.html
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

#[cfg(any(feature = "serde", feature = "musli"))]
mod source_info;

#[cfg(any(feature = "serde", feature = "musli"))]
pub(crate) use source_info::SourceInfo;

pub mod alloc;
#[doc(inline)]
pub use rune_alloc::sync;

/// Helper prelude for `#[no_std]` support.
pub mod no_std;

#[macro_use]
mod internal_macros;
pub(crate) use self::internal_macros::{async_vm_try, declare_dyn_fn, declare_dyn_trait, vm_error};

mod exported_macros;
#[doc(inline)]
#[allow(deprecated)]
pub use self::exported_macros::{docstring, nested_try, vm_panic, vm_try, vm_write};

#[macro_use]
pub mod ast;

#[cfg(feature = "fmt")]
#[cfg_attr(rune_docsrs, doc(cfg(feature = "fmt")))]
pub mod fmt;

#[cfg(feature = "emit")]
#[cfg_attr(rune_docsrs, doc(cfg(feature = "emit")))]
#[doc(inline)]
pub use ::codespan_reporting::term::termcolor;

pub(crate) mod any;
#[doc(inline)]
pub use self::any::Any;

mod build;
pub use self::build::{prepare, Build, BuildError};

pub mod compile;
#[doc(inline)]
pub use self::compile::{Context, ContextError, Options};

pub mod item;
#[doc(inline)]
pub use self::item::{Item, ItemBuf};

#[doc(hidden)]
mod function_meta;

mod function;

pub mod module;
#[doc(inline)]
pub use self::module::module::Module;

pub mod diagnostics;
#[doc(inline)]
pub use self::diagnostics::Diagnostics;

pub mod hash;
#[doc(inline)]
pub use self::hash::{Hash, NonZeroHash, ToTypeHash};

mod hashbrown;

mod params;
pub use self::params::Params;

mod hir;

mod indexing;

pub mod macros;

pub mod modules;

pub mod parse;

pub(crate) mod grammar;

pub mod query;

pub mod runtime;
#[doc(inline)]
pub use self::runtime::{
    from_const_value, from_value, to_const_value, to_value, FromConstValue, FromValue, Mut, Ref,
    ToConstValue, ToValue, TypeHash, Unit, Value, Vm, VmError,
};

mod shared;

pub mod source;
#[doc(inline)]
pub use self::source::Source;

#[macro_use]
mod sources;
#[doc(inline)]
pub use self::sources::{SourceId, Sources};

mod worker;

#[doc(hidden)]
pub mod support;

#[cfg(feature = "workspace")]
#[cfg_attr(rune_docsrs, doc(cfg(feature = "workspace")))]
pub mod workspace;

/// Macro used to annotate native functions which can be loaded as attribute
/// macros in rune.
///
/// See [`Module::macro_meta`][crate::Module::macro_meta].
///
/// # Examples
///
/// ```
/// use rune::Module;
/// use rune::ast;
/// use rune::compile;
/// use rune::macros::{quote, MacroContext, TokenStream};
/// use rune::parse::Parser;
/// use rune::alloc::prelude::*;
///
/// /// Takes an identifier and converts it into a string.
/// ///
/// /// # Examples
/// ///
/// /// ```rune
/// /// assert_eq!(ident_to_string!(Hello), "Hello");
/// /// ```
/// #[rune::macro_]
/// fn ident_to_string(cx: &mut MacroContext<'_, '_, '_>, stream: &TokenStream) -> compile::Result<TokenStream> {
///     let mut p = Parser::from_token_stream(stream, cx.input_span());
///     let ident = p.parse_all::<ast::Ident>()?;
///     let ident = cx.resolve(ident)?.try_to_owned()?;
///     let string = cx.lit(&ident)?;
///     Ok(quote!(#string).into_token_stream(cx)?)
/// }
///
/// let mut m = Module::new();
/// m.macro_meta(ident_to_string)?;
/// # Ok::<_, rune::support::Error>(())
/// ```
#[doc(inline)]
pub use rune_macros::attribute_macro;

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
///   `#[rune::function(protocol = DISPLAY_FMT)]`.
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
/// use rune::Any;
/// use rune::runtime::Formatter;
/// use rune::alloc::fmt::TryWrite;
/// use rune::alloc;
///
/// #[derive(Any)]
/// struct Struct {
///     /* .. */
/// }
///
/// #[rune::function(instance, protocol = DISPLAY_FMT)]
/// fn display_fmt(this: &Struct, f: &mut Formatter) -> alloc::Result<()> {
///     write!(f, "Struct {{ /* .. */ }}")
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
/// use rune::runtime::Formatter;
/// use rune::alloc::fmt::TryWrite;
/// use rune::alloc;
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
///     /// Display the string using the [`DISPLAY_FMT`] protocol.
///     ///
///     /// # Examples
///     ///
///     /// ```rune
///     /// let string = String::new("hello");
///     /// assert_eq!(format!("{}", string), "hello");
///     /// ```
///     #[rune::function(protocol = DISPLAY_FMT)]
///     fn display(&self, f: &mut Formatter) -> alloc::Result<()> {
///         write!(f, "{}", self.inner)
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
/// > **Deprecated:** This feature will be removed in a future version of Rune.
/// > It is not recommended that you use a `Result<T, VmError>` return type
/// > directly and make use of helpers like [`nested_try!`] for propagating
/// > inner errors.
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
/// # Using `keep` to keep the name
///
/// By default, the name of the function is mangled and the metadata is given
/// the original name. This means you can't easily call the function from both
/// Rune and Rust. This behaviour can be changed by using the `keep` attribute, in
/// which case you must refer to the meta object by a mangled name
/// (specifically the function name with `__meta` appended):
///
/// ```
/// use rune::{Module, ContextError};
///
/// /// Don't mangle the name of the function
/// #[rune::function(keep)]
/// fn to_uppercase(string: &str) -> String {
///     string.to_uppercase()
/// }
///
/// fn module() -> Result<Module, ContextError> {
///     let mut m = Module::new();
///     m.function_meta(to_uppercase__meta)?;
///     Ok(m)
/// }
///
/// fn call_from_rust() {
///    assert_eq!(to_uppercase("hello"), "HELLO");
/// }
/// ```
///
/// [`vm_try!`]: crate::vm_try
/// [`VmResult`]: crate::runtime::VmResult
/// [`nested_try!`]: crate::nested_try
#[doc(inline)]
pub use rune_macros::function;

/// Calculate a type hash at compile time.
///
/// By default this uses the `rune` crate.
///
/// # Examples
///
/// ```
/// use rune::Hash;
///
/// let hash: Hash = rune::hash!(::std::option::Option::Some);
/// ```
#[doc(inline)]
pub use rune_macros::hash;

/// Calculate a type hash at compile time using a custom crate.
///
/// By default the [`hash!`] macro uses the `rune` crate.
///
/// # Examples
///
/// ```
/// use rune_core::hash::Hash;
///
/// let hash: Hash = rune::hash_in!(rune_core::hash, ::std::option::Option::Some);
/// ```
#[doc(inline)]
pub use rune_macros::hash_in;

/// Construct an [`Item`] reference at compile time.
///
/// # Examples
///
/// ```
/// use rune::{Item, ItemBuf};
///
/// static ITEM: &Item = rune::item!(::std::ops::generator::Generator);
///
/// let mut item = ItemBuf::with_crate("std")?;
/// item.push("ops")?;
/// item.push("generator")?;
/// item.push("Generator")?;
///
/// assert_eq!(item, ITEM);
/// # Ok::<_, rune::alloc::Error>(())
/// ```
#[doc(inline)]
pub use rune_macros::item;

/// Construct an [`Item`] reference at compile time.
///
/// This variant of the [`item!`] macro allows the module that's used to be
/// specified as the first argument. By default this is `rune`.
///
/// # Examples
///
/// ```
/// use rune_core::item::{Item, ItemBuf};
/// use rune_macros::item_in;
///
/// static ITEM: &Item = rune::item_in!(rune_core::item, ::std::ops::generator::Generator);
///
/// let mut item = ItemBuf::with_crate("std")?;
/// item.push("ops")?;
/// item.push("generator")?;
/// item.push("Generator")?;
///
/// assert_eq!(item, ITEM);
/// # Ok::<_, rune_core::alloc::Error>(())
/// ```
#[doc(inline)]
pub use rune_macros::item_in;

/// Macro used to annotate native functions which can be loaded as macros in
/// rune.
///
/// See [`Module::macro_meta`][crate::Module::macro_meta].
///
/// # Examples
///
/// ```
/// use rune::Module;
/// use rune::ast;
/// use rune::compile;
/// use rune::macros::{quote, MacroContext, TokenStream};
/// use rune::parse::Parser;
/// use rune::alloc::prelude::*;
///
/// /// Takes an identifier and converts it into a string.
/// ///
/// /// # Examples
/// ///
/// /// ```rune
/// /// assert_eq!(ident_to_string!(Hello), "Hello");
/// /// ```
/// #[rune::macro_]
/// fn ident_to_string(cx: &mut MacroContext<'_, '_, '_>, stream: &TokenStream) -> compile::Result<TokenStream> {
///     let mut p = Parser::from_token_stream(stream, cx.input_span());
///     let ident = p.parse_all::<ast::Ident>()?;
///     let ident = cx.resolve(ident)?.try_to_owned()?;
///     let string = cx.lit(&ident)?;
///     Ok(quote!(#string).into_token_stream(cx)?)
/// }
///
/// let mut m = Module::new();
/// m.macro_meta(ident_to_string)?;
/// # Ok::<_, rune::support::Error>(())
/// ```
#[doc(inline)]
pub use rune_macros::macro_;

/// Macro used to annotate a module with metadata.
///
/// ThIs defines a local function `module_meta` which can be used in conjunction
/// with [`Module::from_meta`] to construct a module with a given item and
/// captured documentation.
///
/// [`Module::from_meta`]: crate::module::Module::from_meta
///
/// # Examples
///
/// ```
/// use rune::{ContextError, Module};
///
/// /// Utilities for working with colors.
/// #[rune::module(::color)]
/// pub fn module() -> Result<Module, ContextError> {
///     let mut m = Module::from_meta(module__meta)?;
///
///     // Populate module.
///
///     Ok(m)
/// }
/// ```
#[doc(inline)]
pub use rune_macros::module;

#[cfg(feature = "cli")]
mod ace;

#[cfg(feature = "cli")]
#[cfg_attr(rune_docsrs, doc(cfg(feature = "cli")))]
pub mod cli;

#[cfg(feature = "languageserver")]
#[cfg_attr(rune_docsrs, doc(cfg(feature = "languageserver")))]
pub mod languageserver;

#[cfg(feature = "doc")]
#[cfg_attr(rune_docsrs, doc(cfg(feature = "doc")))]
pub(crate) mod doc;

/// Privately exported details.
#[doc(hidden)]
pub mod __priv {
    pub use crate::any::AnyMarker;
    pub use crate::function_meta::{
        FunctionMetaData, FunctionMetaKind, FunctionMetaStatics, MacroMetaData, MacroMetaKind,
    };
    pub use crate::item::{Item, ItemBuf};
    pub use crate::module::{InstallWith, Module, ModuleMetaData};
    pub use crate::params::Params;
    pub use crate::runtime::{
        AnyTypeInfo, ConstConstruct, ConstConstructImpl, ConstValue, FromConstValue, FromValue,
        MaybeTypeOf, Object, OwnedTuple, Protocol, RawValueGuard, RuntimeError, ToConstValue,
        ToValue, Tuple, TypeHash, TypeOf, TypeValue, UnsafeToMut, UnsafeToRef, UnsafeToValue,
        Value, ValueMutGuard, ValueRefGuard, VmError,
    };
    pub use core::clone::Clone;

    pub mod e {
        use crate::alloc::borrow::TryToOwned;
        use crate::runtime::{AnyTypeInfo, RuntimeError, TypeInfo, VmErrorKind};

        #[doc(hidden)]
        #[inline]
        pub fn missing_struct_field(target: &'static str, name: &'static str) -> RuntimeError {
            RuntimeError::new(VmErrorKind::MissingStructField { target, name })
        }

        #[doc(hidden)]
        #[inline]
        pub fn missing_variant(name: &str) -> RuntimeError {
            match name.try_to_owned() {
                Ok(name) => RuntimeError::new(VmErrorKind::MissingVariant { name }),
                Err(error) => RuntimeError::from(error),
            }
        }

        #[doc(hidden)]
        #[inline]
        pub fn expected_variant(actual: TypeInfo) -> RuntimeError {
            RuntimeError::new(VmErrorKind::ExpectedVariant { actual })
        }

        #[doc(hidden)]
        #[inline]
        pub fn missing_variant_name() -> RuntimeError {
            RuntimeError::new(VmErrorKind::MissingVariantName)
        }

        #[doc(hidden)]
        #[inline]
        pub fn missing_tuple_index(target: &'static str, index: usize) -> RuntimeError {
            RuntimeError::new(VmErrorKind::MissingTupleIndex { target, index })
        }

        #[doc(hidden)]
        #[inline]
        pub fn unsupported_object_field_get(target: AnyTypeInfo) -> RuntimeError {
            RuntimeError::new(VmErrorKind::UnsupportedObjectFieldGet {
                target: TypeInfo::from(target),
            })
        }

        #[doc(hidden)]
        #[inline]
        pub fn unsupported_tuple_index_get(target: AnyTypeInfo, index: usize) -> RuntimeError {
            RuntimeError::new(VmErrorKind::UnsupportedTupleIndexGet {
                target: TypeInfo::from(target),
                index,
            })
        }
    }
}

#[cfg(feature = "musli")]
mod musli;
#[cfg(feature = "serde")]
mod serde;

#[cfg(test)]
mod tests;

rune_macros::binding! {
    impl ::std::string::String for crate::alloc::String;
    #[cfg(feature = "std")]
    #[cfg_attr(rune_docsrs, doc(cfg(feature = "std")))]
    impl ::std::io::Error for std::io::Error;
    impl ::std::string::FromUtf8Error for crate::alloc::string::FromUtf8Error;
    #[cfg(feature = "anyhow")]
    impl ::std::error::Error for anyhow::Error;
    impl ::std::fmt::Error for core::fmt::Error;
    impl ::std::char::ParseCharError for core::char::ParseCharError;
    impl ::std::num::ParseFloatError for core::num::ParseFloatError;
    impl ::std::num::ParseIntError for core::num::ParseIntError;
    impl ::std::string::Utf8Error for core::str::Utf8Error;
    #[any]
    impl ::std::option::Option for Option<Value>;
    #[type_of]
    impl<T> ::std::option::Option for Option<T>;
    #[any]
    impl ::std::result::Result for Result<Value, Value>;
    #[type_of]
    impl<T, E> ::std::result::Result for Result<T, E>;
    #[type_of]
    impl ::std::bool for bool;
    #[type_of]
    impl ::std::char for char;
    #[type_of]
    impl ::std::i64 for i8;
    #[type_of]
    impl ::std::i64 for i16;
    #[type_of]
    impl ::std::i64 for i32;
    #[type_of]
    impl ::std::i64 for i64;
    #[type_of]
    impl ::std::i64 for i128;
    #[type_of]
    impl ::std::i64 for isize;
    #[type_of]
    impl ::std::u64 for u8;
    #[type_of]
    impl ::std::u64 for u16;
    #[type_of]
    impl ::std::u64 for u32;
    #[type_of]
    impl ::std::u64 for u64;
    #[type_of]
    impl ::std::u64 for u128;
    #[type_of]
    impl ::std::u64 for usize;
    #[type_of]
    impl ::std::f64 for f32;
    #[type_of]
    impl ::std::f64 for f64;
    #[type_of]
    impl<C, B> ::std::ops::ControlFlow for core::ops::ControlFlow<C, B>;
    #[type_of]
    impl ::std::bytes::Bytes for [u8];
    #[type_of]
    impl ::std::cmp::Ordering for core::cmp::Ordering;
    #[type_of]
    impl ::std::string::String for rust_alloc::string::String;
    #[type_of]
    impl ::std::string::String for crate::alloc::Box<str>;
    #[type_of]
    impl ::std::string::String for str;
    #[type_of]
    impl ::std::vec::Vec for [Value];
    #[type_of]
    impl<T> ::std::vec::Vec for rust_alloc::vec::Vec<T>;
    #[type_of]
    impl<T> ::std::vec::Vec for crate::alloc::Vec<T>;
    #[type_of]
    impl<T, const N: usize> ::std::vec::Vec for [T; N];
    #[type_of]
    impl<T> ::std::vec::Vec for crate::runtime::VecTuple<T>;
    #[type_of]
    impl ::std::tuple::Tuple for crate::runtime::Tuple;
    #[type_of]
    impl<T> ::std::object::Object for crate::alloc::HashMap<rust_alloc::string::String, T>;
    #[type_of]
    impl<T> ::std::object::Object for crate::alloc::HashMap<alloc::String, T>;
    #[type_of]
    #[cfg(feature = "std")]
    #[cfg_attr(rune_docsrs, doc(cfg(feature = "std")))]
    impl<T> ::std::object::Object for std::collections::HashMap<rust_alloc::string::String, T>;
    #[type_of]
    #[cfg(feature = "std")]
    #[cfg_attr(rune_docsrs, doc(cfg(feature = "std")))]
    impl<T> ::std::object::Object for std::collections::HashMap<alloc::String, T>;
    #[type_of]
    impl ::std::any::Type for crate::runtime::Type;
    #[type_of]
    impl ::std::any::Hash for crate::hash::Hash;
}

vm_error!(crate::alloc::Error);
