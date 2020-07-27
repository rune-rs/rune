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
mod hash;
pub mod packages;
mod reflection;
mod unit;

pub use crate::functions::{Functions, Register, RegisterAsync};
pub use crate::hash::{FnDynamicHash, FnHash, Hash};
pub use crate::reflection::{
    Allocate, AllocateError, FromValue, IntoArgs, ReflectValueType, ToValue,
};
pub use crate::unit::{Unit, UnitError};
pub use crate::value::{Managed, Slot, TypeHash, Value, ValueError, ValueRef, ValueType};
pub use crate::vm::{Inst, Task, Vm};

mod collections {
    pub use std::collections::{hash_map, HashMap};
}
