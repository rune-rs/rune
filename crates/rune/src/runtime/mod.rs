//! Runtime module for Rune.

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
pub use self::from_value::{from_value, FromValue, UnsafeFromValue};

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
    Inst, InstAddress, InstAssignOp, InstOp, InstRangeLimits, InstTarget, InstValue, InstVariant,
    PanicReason, TypeCheck,
};

mod iterator;
pub use self::iterator::{Iterator, IteratorTrait};

mod key;
pub use self::key::Key;

mod label;
pub use self::label::{DebugLabel, Label};

mod object;
pub use self::object::Object;

mod panic;
pub(crate) use self::panic::{BoxedPanic, Panic};

mod protocol;
pub use self::protocol::Protocol;

mod protocol_caller;
pub(crate) use self::protocol_caller::{EnvProtocolCaller, ProtocolCaller};

mod range;
pub use self::range::{Range, RangeLimits};

mod raw_str;
pub use self::raw_str::RawStr;

mod runtime_context;
pub use self::runtime_context::RuntimeContext;
pub(crate) use self::runtime_context::{FunctionHandler, MacroHandler};

mod select;
pub(crate) use self::select::Select;

mod shared;
pub use self::shared::{Mut, RawMut, RawRef, Ref, Shared, SharedPointerGuard};

mod stack;
pub use self::stack::{Stack, StackError};

mod static_string;
pub use self::static_string::StaticString;

mod static_type;
pub use self::static_type::{
    StaticType, BOOL_TYPE, BYTES_TYPE, BYTE_TYPE, CHAR_TYPE, FLOAT_TYPE, FORMAT_TYPE,
    FUNCTION_TYPE, FUTURE_TYPE, GENERATOR_STATE_TYPE, GENERATOR_TYPE, INTEGER_TYPE, ITERATOR_TYPE,
    OBJECT_TYPE, OPTION_TYPE, RANGE_TYPE, RESULT_TYPE, STREAM_TYPE, STRING_TYPE, TUPLE_TYPE, TYPE,
    UNIT_TYPE, VEC_TYPE,
};

mod stream;
pub use self::stream::Stream;

mod to_value;
pub use self::to_value::{to_value, ToValue, UnsafeToValue};

mod tuple;
pub use self::tuple::Tuple;

mod type_info;
pub use self::type_info::{AnyTypeInfo, TypeInfo};

mod type_of;
pub use self::type_of::{FullTypeOf, MaybeTypeOf, TypeOf};

mod unit;
pub use self::unit::{Unit, UnitFn};

mod value;
pub use self::value::{Rtti, Struct, TupleStruct, UnitStruct, Value, VariantRtti};

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
pub use self::vm_error::VmErrorKind;
pub use self::vm_error::{try_result, TryFromResult, VmError, VmIntegerRepr, VmResult};

mod vm_execution;
pub use self::vm_execution::{ExecutionState, VmExecution, VmSendExecution};

mod vm_halt;
pub(crate) use self::vm_halt::VmHalt;
pub use self::vm_halt::VmHaltInfo;
