//! # runestick
//!
//! A stack-based virtual machine for the Rust programming language.
//!
//! This drives the [Rune language].
//!
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
mod error;
mod fn_ptr;
mod future;
mod hash;
mod inst;
mod item;
mod meta;
pub(crate) mod module;
pub mod packages;
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
mod value_type;
mod value_type_info;

pub use self::meta::{Meta, MetaClosureCapture, MetaStruct, MetaTuple};
pub use self::module::{AsyncFunction, AsyncInstFn, Function, InstFn, Module};
pub use self::static_string::StaticString;
pub use self::static_type::{
    StaticType, BOOL_TYPE, BYTES_TYPE, BYTE_TYPE, CHAR_TYPE, FLOAT_TYPE, FN_PTR_TYPE, FUTURE_TYPE,
    INTEGER_TYPE, OBJECT_TYPE, OPTION_TYPE, RESULT_TYPE, STRING_TYPE, TUPLE_TYPE, UNIT_TYPE,
    VEC_TYPE,
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
pub use crate::error::{Error, Result};
pub use crate::fn_ptr::FnPtr;
pub use crate::future::Future;
pub use crate::hash::{Hash, IntoTypeHash};
pub use crate::inst::{Inst, OptionVariant, PanicReason, ResultVariant, TypeCheck};
pub use crate::item::{Component, Item};
pub use crate::panic::Panic;
pub use crate::protocol::{
    Protocol, ADD, ADD_ASSIGN, DIV, DIV_ASSIGN, INDEX_GET, INDEX_SET, INTO_ITER, MUL, MUL_ASSIGN,
    NEXT, REM, STRING_DISPLAY, SUB, SUB_ASSIGN,
};
pub use crate::reflection::{FromValue, IntoArgs, ReflectValueType, ToValue, UnsafeFromValue};
pub use crate::shared::{OwnedMut, OwnedRef, RawOwnedMut, RawOwnedRef, Shared};
pub use crate::stack::{Stack, StackError};
pub use crate::unit::{Span, Unit, UnitError};
pub use crate::value::{
    Integer, Object, TypedObject, TypedTuple, Value, ValueError, VariantObject, VariantTuple,
    VecTuple,
};
pub use crate::vm::{Task, Vm, VmError};

mod collections {
    pub use hashbrown::HashMap;
    pub use hashbrown::HashSet;
}
