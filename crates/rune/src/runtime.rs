//! Runtime module for Rune.

#[cfg(test)]
mod tests;

mod access;
pub(crate) use self::access::{Access, AccessKind};
pub use self::access::{
    AccessError, BorrowMut, BorrowRef, NotAccessibleMut, NotAccessibleRef, RawAccessGuard,
};

mod any_obj;
pub use self::any_obj::{AnyObj, AnyObjError, AnyObjVtable};

mod args;
pub use self::args::Args;

mod awaited;
pub(crate) use self::awaited::Awaited;

pub mod budget;

mod bytes;
pub use self::bytes::Bytes;

mod call;
pub use self::call::Call;

mod const_value;
pub use self::const_value::ConstValue;

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

mod generator;
pub use self::generator::Generator;

mod generator_state;
pub use self::generator_state::GeneratorState;

mod guarded_args;
pub use self::guarded_args::GuardedArgs;

mod inst;
pub use self::inst::{
    Inst, InstAddress, InstAssignOp, InstOp, InstRange, InstTarget, InstValue, InstVariant,
    PanicReason, TypeCheck,
};

mod iterator;
pub use self::iterator::{Iterator, IteratorTrait};

mod type_;
pub use self::type_::Type;

mod label;
pub use self::label::DebugLabel;
pub(crate) use self::label::Label;

mod object;
pub use self::object::Object;

mod panic;
pub(crate) use self::panic::{BoxedPanic, Panic};

mod protocol;
pub use self::protocol::Protocol;

mod protocol_caller;
pub(crate) use self::protocol_caller::{EnvProtocolCaller, ProtocolCaller};

mod range_from;
pub use self::range_from::RangeFrom;

mod range_full;
pub use self::range_full::RangeFull;

mod range_to_inclusive;
pub use self::range_to_inclusive::RangeToInclusive;

mod range_to;
pub use self::range_to::RangeTo;

mod range_inclusive;
pub use self::range_inclusive::RangeInclusive;

mod range;
pub use self::range::Range;

#[doc(inline)]
pub use rune_core::RawStr;

mod runtime_context;
pub use self::runtime_context::RuntimeContext;
pub(crate) use self::runtime_context::{AttributeMacroHandler, FunctionHandler, MacroHandler};

mod select;
pub(crate) use self::select::Select;

mod shared;
pub use self::shared::{Mut, RawMut, RawRef, Ref, Shared, SharedPointerGuard};

mod stack;
pub use self::stack::{Stack, StackError};

mod static_string;
pub use self::static_string::StaticString;

pub(crate) mod static_type;
pub use self::static_type::StaticType;

mod stream;
pub use self::stream::Stream;

mod to_value;
pub use self::to_value::{to_value, ToValue, UnsafeToValue};

mod tuple;
pub use self::tuple::{OwnedTuple, Tuple};

mod type_info;
pub use self::type_info::{AnyTypeInfo, TypeInfo};

mod type_of;
pub use self::type_of::{FullTypeOf, MaybeTypeOf, TypeOf};

pub mod unit;
pub(crate) use self::unit::UnitFn;
pub use self::unit::{Unit, UnitStorage};

mod value;
pub use self::value::{EmptyStruct, Rtti, Struct, TupleStruct, Value, VariantRtti};

mod variant;
pub use self::variant::{Variant, VariantData};

mod vec;
pub use self::vec::Vec;

mod vec_tuple;
pub use self::vec_tuple::VecTuple;

mod vm;
pub use self::vm::{CallFrame, Vm};

mod vm_call;
pub(crate) use self::vm_call::VmCall;

mod vm_error;
#[cfg(feature = "emit")]
pub(crate) use self::vm_error::VmErrorAt;
pub(crate) use self::vm_error::VmErrorKind;
pub use self::vm_error::{try_result, TryFromResult, VmError, VmIntegerRepr, VmResult};

mod vm_execution;
pub use self::vm_execution::{ExecutionState, VmExecution, VmSendExecution};

mod vm_halt;
pub(crate) use self::vm_halt::VmHalt;
pub use self::vm_halt::VmHaltInfo;

mod fmt;
pub use self::fmt::Formatter;

mod control_flow;
pub use self::control_flow::ControlFlow;

#[cfg(feature = "alloc")]
mod hasher;
#[cfg(feature = "alloc")]
pub use self::hasher::Hasher;
