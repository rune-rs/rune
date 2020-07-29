//! ST, a really simple stack-based virtual machine.
//!
//! # Example Scripts
//!
//! ST comes with a simple scripting language called rune.
//!
//! You can run example scripts through rune-cli:
//!
//! ```bash
//! cargo rune-cli ./scripts/fib.rn
//! ```

#![deny(missing_docs)]

mod external;
mod functions;
mod value;
mod vm;
#[macro_use]
mod macros;
mod error;
mod hash;
pub mod packages;
mod reflection;
mod unit;

pub use crate::error::{Error, Result};
pub use crate::functions::{CallError, Functions, GlobalModule, Module, RegisterError};
pub use crate::hash::Hash;
pub use crate::reflection::{FromValue, IntoArgs, ReflectValueType, ToValue, UnsafeFromValue};
pub use crate::unit::{Unit, UnitError};
pub use crate::value::{Managed, Slot, Value, ValueError, ValuePtr, ValueRef, ValueType};
pub use crate::vm::{Inst, Task, Vm};

mod collections {
    pub use hashbrown::{hash_map, HashMap};
}
