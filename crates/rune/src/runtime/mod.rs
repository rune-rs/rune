//! Runtime module for Rune.

#[cfg(test)]
mod tests;

#[macro_use]
mod macros;

mod steps_between;
use self::steps_between::StepsBetween;

mod access;
pub use self::access::AccessError;
pub(crate) use self::access::{Access, AccessErrorKind, RawAccessGuard, Snapshot};

mod borrow_mut;
pub use self::borrow_mut::BorrowMut;

mod borrow_ref;
pub use self::borrow_ref::BorrowRef;

mod any_obj;
use self::any_obj::AnyObjErrorKind;
pub use self::any_obj::{AnyObj, AnyObjError};
pub(crate) use self::any_obj::{AnyObjDrop, RawAnyObjGuard};

mod args;
pub use self::args::{Args, FixedArgs};
pub(crate) use self::args::{DynArgs, DynArgsUsed, DynGuardedArgs};

mod awaited;
pub(crate) use self::awaited::Awaited;

pub mod budget;

mod bytes;
pub use self::bytes::Bytes;

mod call;
pub use self::call::Call;

mod const_value;
pub use self::const_value::{
    from_const_value, to_const_value, ConstConstruct, ConstValue, FromConstValue, ToConstValue,
};
pub(crate) use self::const_value::{ConstContext, ConstValueKind, EmptyConstContext};

pub mod debug;
pub use self::debug::{DebugInfo, DebugInst};

mod env;

pub mod format;
pub use self::format::{Format, FormatSpec};

mod from_value;
#[allow(deprecated)]
pub use self::from_value::UnsafeFromValue;
pub use self::from_value::{from_value, FromValue, UnsafeToMut, UnsafeToRef};

mod function;
pub use self::function::{Function, SyncFunction};

mod future;
pub use self::future::Future;
pub(crate) use self::future::SelectFuture;

pub(crate) mod generator;
pub use self::generator::Generator;

mod generator_state;
pub use self::generator_state::GeneratorState;

mod guarded_args;
pub use self::guarded_args::GuardedArgs;

mod inst;
pub use self::inst::{
    Inst, InstAddress, InstAssignOp, InstOp, InstRange, InstTarget, InstValue, InstVariant,
    IntoOutput, Output, PanicReason, TypeCheck,
};

mod iterator;
pub use self::iterator::Iterator;

mod type_;
pub use self::type_::Type;

mod label;
pub use self::label::DebugLabel;
pub(crate) use self::label::Label;

pub(crate) mod object;
pub use self::object::Object;

mod panic;
pub(crate) use self::panic::{BoxedPanic, Panic};

mod protocol;
pub use self::protocol::Protocol;

mod protocol_caller;
pub(crate) use self::protocol_caller::{EnvProtocolCaller, ProtocolCaller};

pub(crate) mod range_from;
pub use self::range_from::RangeFrom;

mod range_full;
pub use self::range_full::RangeFull;

mod range_to_inclusive;
pub use self::range_to_inclusive::RangeToInclusive;

mod range_to;
pub use self::range_to::RangeTo;

pub(crate) mod range_inclusive;
pub use self::range_inclusive::RangeInclusive;

pub(crate) mod range;
pub use self::range::Range;

mod runtime_context;
pub(crate) use self::runtime_context::FunctionHandler;
pub use self::runtime_context::RuntimeContext;

mod select;
pub(crate) use self::select::Select;

mod r#ref;
use self::r#ref::RefVtable;
pub use self::r#ref::{Mut, RawAnyGuard, Ref};

mod shared;
pub(crate) use self::shared::Shared;

mod stack;
pub(crate) use self::stack::Pair;
pub use self::stack::{Memory, SliceError, Stack, StackError};

mod static_string;
pub use self::static_string::StaticString;

pub(crate) mod static_type;
pub use self::static_type::StaticType;

mod stream;
pub use self::stream::Stream;

mod to_value;
pub use self::to_value::{to_value, ToReturn, ToValue, UnsafeToValue};

mod tuple;
pub use self::tuple::{OwnedTuple, Tuple};

mod type_info;
pub use self::type_info::{AnyTypeInfo, TypeInfo};

mod type_of;
use self::type_of::StaticTypeInfoKind;
pub use self::type_of::{MaybeTypeOf, StaticTypeInfo, TypeHash, TypeOf};

pub mod unit;
pub(crate) use self::unit::UnitFn;
pub use self::unit::{Unit, UnitStorage};

mod value;
pub use self::value::{
    Accessor, EmptyStruct, Inline, RawValueGuard, Rtti, Struct, TupleStruct, TypeValue, Value,
    ValueMutGuard, ValueRefGuard, VariantRtti,
};
pub(crate) use self::value::{BorrowRefRepr, MutRepr, Mutable, OwnedRepr, RefRepr};

mod variant;
pub use self::variant::{Variant, VariantData};

pub mod slice;

mod vec;
pub use self::vec::Vec;

mod vec_tuple;
pub use self::vec_tuple::VecTuple;

mod vm;
use self::vm::CallResultOnly;
pub use self::vm::{CallFrame, Isolated, Vm};

mod vm_call;
pub(crate) use self::vm_call::VmCall;

pub(crate) mod vm_diagnostics;
pub(crate) use self::vm_diagnostics::{VmDiagnostics, VmDiagnosticsObj};

mod vm_error;
#[cfg(feature = "emit")]
pub(crate) use self::vm_error::VmErrorAt;
pub use self::vm_error::{try_result, RuntimeError, TryFromResult, VmError, VmResult};
pub(crate) use self::vm_error::{VmErrorKind, VmIntegerRepr};

mod vm_execution;
pub(crate) use self::vm_execution::ExecutionState;
pub use self::vm_execution::{VmExecution, VmSendExecution};

mod vm_halt;
pub(crate) use self::vm_halt::{VmHalt, VmHaltInfo};

mod fmt;
pub use self::fmt::Formatter;

mod control_flow;
pub use self::control_flow::ControlFlow;

#[cfg(feature = "alloc")]
mod hasher;
#[cfg(feature = "alloc")]
pub use self::hasher::Hasher;
