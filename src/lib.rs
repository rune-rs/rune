//! ST, a really simple stack-based virtual machine.
//!
//! # Example Scripts
//!
//! ST comes with a simple scripting language called rune.
//!
//! You can run example scripts through rune-cli:
//!
//! ```bash
//! cargo run --manifest-path=rune-cli/Cargo.toml --release -- ./scripts/fib.rn
//! ```

#![deny(missing_docs)]

mod external;
mod functions;
mod value;
mod vm;
#[macro_use]
mod macros;
mod reflection;
mod unit;

pub use crate::functions::{Functions, Register, RegisterAsync};
pub use crate::reflection::{
    Allocate, AllocateError, FromValue, IntoArgs, ReflectValueType, ToValue,
};
pub use crate::unit::{Unit, UnitError};
pub use crate::value::{TypeHash, Value, ValueType};
pub use crate::vm::{FnDynamicHash, FnHash, Inst, Task, Vm};

mod collections {
    pub use std::collections::{hash_map, HashMap};
}
