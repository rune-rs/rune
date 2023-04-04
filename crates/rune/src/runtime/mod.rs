//! Runtime module for Rune.

mod access;
mod any_obj;
mod args;
mod awaited;
pub mod budget;
mod bytes;
mod call;
mod const_value;
pub mod debug;
mod env;
pub mod format;
mod from_value;
mod function;
pub(crate) mod future;
mod generator;
mod generator_state;
mod guarded_args;
mod inst;
mod iterator;
mod key;
mod label;
mod object;
mod panic;
mod protocol;
mod protocol_caller;
mod range;
mod raw_str;
mod runtime_context;
mod select;
mod shared;
mod stack;
mod static_string;
mod static_type;
mod stream;
mod to_value;
mod tuple;
mod type_info;
mod type_of;
mod unit;
mod value;
mod variant;
mod vec;
mod vec_tuple;
mod vm;
mod vm_call;
mod vm_error;
mod vm_execution;
mod vm_halt;

pub(crate) use self::access::{Access, AccessKind};
pub use self::access::{
    AccessError, BorrowMut, BorrowRef, NotAccessibleMut, NotAccessibleRef, RawAccessGuard,
};
pub use self::any_obj::{AnyObj, AnyObjError, AnyObjVtable};
pub use self::args::Args;
pub(crate) use self::awaited::Awaited;
pub use self::bytes::Bytes;
pub use self::call::Call;
pub use self::const_value::ConstValue;
pub use self::debug::{DebugInfo, DebugInst};
pub use self::format::{Format, FormatSpec};
pub use self::from_value::{FromValue, UnsafeFromValue};
pub use self::function::{Function, SyncFunction};
pub use self::future::Future;
pub use self::generator::Generator;
pub use self::generator_state::GeneratorState;
pub use self::guarded_args::GuardedArgs;
pub use self::inst::{
    Inst, InstAddress, InstAssignOp, InstOp, InstRangeLimits, InstTarget, InstValue, InstVariant,
    PanicReason, TypeCheck,
};
pub use self::iterator::{Iterator, IteratorTrait};
pub use self::key::Key;
pub use self::label::{DebugLabel, Label};
pub use self::object::Object;
pub use self::panic::Panic;
pub use self::protocol::Protocol;
pub(crate) use self::protocol_caller::{EnvProtocolCaller, ProtocolCaller};
pub use self::range::{Range, RangeLimits};
pub use self::raw_str::RawStr;
pub use self::runtime_context::RuntimeContext;
pub(crate) use self::runtime_context::{FunctionHandler, MacroHandler};
pub use self::select::Select;
pub use self::shared::{Mut, RawMut, RawRef, Ref, Shared, SharedPointerGuard};
pub use self::stack::{Stack, StackError};
pub use self::static_string::StaticString;
pub use self::static_type::{
    StaticType, BOOL_TYPE, BYTES_TYPE, BYTE_TYPE, CHAR_TYPE, FLOAT_TYPE, FORMAT_TYPE,
    FUNCTION_TYPE, FUTURE_TYPE, GENERATOR_STATE_TYPE, GENERATOR_TYPE, INTEGER_TYPE, ITERATOR_TYPE,
    OBJECT_TYPE, OPTION_TYPE, RANGE_TYPE, RESULT_TYPE, STREAM_TYPE, STRING_TYPE, TUPLE_TYPE, TYPE,
    UNIT_TYPE, VEC_TYPE,
};
pub use self::stream::Stream;
pub use self::to_value::{ToValue, UnsafeToValue};
pub use self::tuple::Tuple;
pub use self::type_info::{AnyTypeInfo, TypeInfo};
pub use self::type_of::{FullTypeOf, MaybeTypeOf, TypeOf};
pub use self::unit::{Unit, UnitFn};
pub use self::value::{Rtti, Struct, TupleStruct, UnitStruct, Value, VariantRtti};
pub use self::variant::{Variant, VariantData};
pub use self::vec::Vec;
pub use self::vec_tuple::VecTuple;
pub use self::vm::{CallFrame, Vm};
pub(crate) use self::vm_call::VmCall;
pub use self::vm_error::{
    try_result, TryFromResult, VmError, VmErrorKind, VmErrorWithTrace, VmIntegerRepr, VmResult,
};
pub use self::vm_execution::{ExecutionState, VmExecution, VmSendExecution};
pub(crate) use self::vm_halt::VmHalt;
pub use self::vm_halt::VmHaltInfo;
