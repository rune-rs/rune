//! <div align="center">
//! <a href="https://rune-rs.github.io/rune/">
//!     <b>Read the Book 📖</b>
//! </a>
//! </div>
//!
//! <br>
//!
//! <div align="center">
//! <a href="https://github.com/rune-rs/rune/actions">
//!     <img alt="Build Status" src="https://github.com/rune-rs/rune/workflows/Build/badge.svg">
//! </a>
//!
//! <a href="https://github.com/rune-rs/rune/actions">
//!     <img alt="Book Status" src="https://github.com/rune-rs/rune/workflows/Book/badge.svg">
//! </a>
//!
//! <a href="https://discord.gg/v5AeNkT">
//!     <img alt="Chat on Discord" src="https://img.shields.io/discord/558644981137670144.svg?logo=discord&style=flat-square">
//! </a>
//! </div>
//!
//! <br>
//!
//! A stack-based virtual machine for the Rust programming language.
//!
//! This is the driver for the [Rune language].
//! [Rune Language]: https://github.com/rune-rs/rune

#![deny(missing_docs)]

mod any;
mod context;
mod value;
mod vm;
#[macro_use]
mod macros;
mod access;
mod bytes;
mod fn_ptr;
mod future;
mod generator;
mod generator_state;
mod hash;
mod inst;
mod item;
mod meta;
pub(crate) mod module;
pub mod modules;
mod names;
mod panic;
mod protocol;
mod reflection;
mod serde;
mod shared;
mod stack;
mod static_string;
mod static_type;
mod tuple;
pub mod unit;
mod value_error;
mod value_type;
mod value_type_info;
mod vec_tuple;
mod vm_error;

decl_external!(anyhow::Error);

/// Exported result type for convenience.
pub type Result<T, E = anyhow::Error> = std::result::Result<T, E>;

/// Exported boxed error type for convenience.
pub type Error = anyhow::Error;

pub use self::generator::Generator;
pub use self::generator_state::GeneratorState;
pub use self::meta::{Meta, MetaClosureCapture, MetaStruct, MetaTuple};
pub use self::module::{AsyncFunction, AsyncInstFn, Function, InstFn, Module};
pub use self::static_string::StaticString;
pub use self::static_type::{
    StaticType, BOOL_TYPE, BYTES_TYPE, BYTE_TYPE, CHAR_TYPE, FLOAT_TYPE, FN_PTR_TYPE, FUTURE_TYPE,
    GENERATOR_STATE_TYPE, GENERATOR_TYPE, INTEGER_TYPE, OBJECT_TYPE, OPTION_TYPE, RESULT_TYPE,
    STRING_TYPE, TUPLE_TYPE, UNIT_TYPE, VEC_TYPE,
};
pub use self::tuple::Tuple;
pub use self::value_type::ValueType;
pub use self::value_type_info::ValueTypeInfo;
pub use crate::access::{
    AccessError, BorrowMut, BorrowRef, NotAccessibleMut, NotAccessibleRef, RawBorrowedMut,
    RawBorrowedRef,
};
pub use crate::any::Any;
pub use crate::bytes::Bytes;
pub use crate::context::{Context, ContextError, IntoInstFnHash};
pub use crate::fn_ptr::FnPtr;
pub use crate::future::Future;
pub use crate::hash::{Hash, IntoTypeHash};
pub use crate::inst::{Inst, PanicReason, TypeCheck};
pub use crate::item::{Component, Item};
pub use crate::names::Names;
pub use crate::panic::Panic;
pub use crate::protocol::{
    Protocol, ADD, ADD_ASSIGN, DIV, DIV_ASSIGN, INDEX_GET, INDEX_SET, INTO_FUTURE, INTO_ITER, MUL,
    MUL_ASSIGN, NEXT, REM, STRING_DISPLAY, SUB, SUB_ASSIGN,
};
pub use crate::reflection::{FromValue, IntoArgs, ReflectValueType, ToValue, UnsafeFromValue};
pub use crate::shared::{OwnedMut, OwnedRef, RawOwnedMut, RawOwnedRef, Shared};
pub use crate::stack::{Stack, StackError};
pub use crate::unit::{ImportEntry, ImportKey, Span, Unit, UnitError};
pub use crate::value::{
    Integer, Object, TypedObject, TypedTuple, Value, VariantObject, VariantTuple,
};
pub use crate::value_error::{ValueError, ValueErrorKind};
pub use crate::vec_tuple::VecTuple;
pub use crate::vm::{StopReason, Task, Vm};
pub use crate::vm_error::{VmError, VmErrorKind};

mod collections {
    pub use hashbrown::HashMap;
    pub use hashbrown::HashSet;
}
