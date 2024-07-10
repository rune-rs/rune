use core::fmt;
use core::num::NonZeroUsize;

use musli::{Decode, Encode};
use rune_macros::InstDisplay;
use serde::{Deserialize, Serialize};

use crate as rune;
use crate::alloc;
use crate::alloc::prelude::*;
use crate::runtime::{Call, FormatSpec, Stack, Type, Value, ValueKind, VmErrorKind, VmResult};
use crate::Hash;

/// Pre-canned panic reasons.
///
/// To formulate a custom reason, use
/// [`VmError::panic`][crate::runtime::VmError::panic].
#[derive(Debug, TryClone, Clone, Copy, Serialize, Deserialize, Decode, Encode)]
#[try_clone(copy)]
#[non_exhaustive]
pub enum PanicReason {
    /// Not implemented.
    NotImplemented,
    /// A pattern didn't match where it unconditionally has to.
    UnmatchedPattern,
    /// Tried to poll a future that has already been completed.
    FutureCompleted,
}

impl PanicReason {
    /// The identifier of the panic.
    fn ident(&self) -> &'static str {
        match *self {
            Self::NotImplemented => "not implemented",
            Self::UnmatchedPattern => "unmatched pattern",
            Self::FutureCompleted => "future completed",
        }
    }
}

impl fmt::Display for PanicReason {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NotImplemented => write!(fmt, "functionality has not been implemented yet")?,
            Self::UnmatchedPattern => write!(fmt, "pattern did not match")?,
            Self::FutureCompleted => {
                write!(fmt, "tried to poll future that has already been completed")?
            }
        }

        Ok(())
    }
}

/// Type checks for built-in types.
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Serialize, Deserialize, Decode, Encode)]
#[try_clone(copy)]
#[non_exhaustive]
pub enum TypeCheck {
    /// Matches a unit type.
    EmptyTuple,
    /// Matches an anonymous tuple.
    Tuple,
    /// Matches an anonymous object.
    Object,
    /// Matches a vector.
    Vec,
    /// An option type, and the specified variant index.
    #[musli(packed)]
    Option(usize),
    /// A result type, and the specified variant index.
    #[musli(packed)]
    Result(usize),
    /// A generator state type, and the specified variant index.
    #[musli(packed)]
    GeneratorState(usize),
}

impl fmt::Display for TypeCheck {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTuple => write!(fmt, "Unit"),
            Self::Tuple => write!(fmt, "Tuple"),
            Self::Object => write!(fmt, "Object"),
            Self::Vec => write!(fmt, "Vec"),
            Self::Option(0) => write!(fmt, "Option::Some"),
            Self::Option(..) => write!(fmt, "Option::None"),
            Self::Result(0) => write!(fmt, "Result::Ok"),
            Self::Result(..) => write!(fmt, "Result::Err"),
            Self::GeneratorState(0) => write!(fmt, "GeneratorState::Yielded"),
            Self::GeneratorState(..) => write!(fmt, "GeneratorState::Complete"),
        }
    }
}

/// An operation in the stack-based virtual machine.
#[derive(Debug, TryClone, Clone, Copy, Serialize, Deserialize, Decode, Encode, InstDisplay)]
#[try_clone(copy)]
pub enum Inst {
    /// Initialize the function-local memory region with space for `size` variables.
    Size {
        /// The size of the memory region to allocate.
        size: usize,
    },
    /// Not operator. Takes a boolean from the top of the stack  and inverts its
    /// logical value.
    ///
    /// # Operation
    ///
    /// ```text
    /// <bool>
    /// => <bool>
    /// ```
    Not {
        /// The operand to negate.
        operand: InstAddress,
        /// Whether the produced value from the not should be kept or not.
        out: Output,
    },
    /// Negate the numerical value on the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// <number>
    /// => <number>
    /// ```
    Neg {
        /// Whether the produced value from the negation should be kept or not.
        out: Output,
    },
    /// Construct a closure that takes the given number of arguments and
    /// captures `count` elements from the top of the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value..>
    /// => <fn>
    /// ```
    #[musli(packed)]
    Closure {
        /// The hash of the internally stored closure function.
        hash: Hash,
        /// The number of arguments to store in the environment on the stack.
        count: usize,
    },
    /// Perform a function call within the same unit.
    ///
    /// It will construct a new stack frame which includes the last `args`
    /// number of entries.
    #[musli(packed)]
    CallOffset {
        /// The offset of the function being called in the same unit.
        offset: usize,
        /// The calling convention to use.
        call: Call,
        /// The address where the arguments are stored.
        addr: InstAddress,
        /// The number of arguments expected on the stack for this call.
        args: usize,
        /// Whether the return value should be kept or not.
        out: Output,
    },
    /// Perform a function call.
    ///
    /// It will construct a new stack frame which includes the last `args`
    /// number of entries.
    #[musli(packed)]
    Call {
        /// The hash of the function to call.
        hash: Hash,
        /// The address of the arguments being passed.
        addr: InstAddress,
        /// The number of arguments expected on the stack for this call.
        args: usize,
        /// Whether the return value should be kept or not.
        out: Output,
    },
    /// Perform a instance function call.
    ///
    /// The instance being called on should be on top of the stack, followed by
    /// `args` number of arguments.
    #[musli(packed)]
    CallAssociated {
        /// The hash of the name of the function to call.
        hash: Hash,
        /// The address of arguments being passed.
        addr: InstAddress,
        /// The number of arguments expected on the stack for this call.
        args: usize,
        /// Whether the return value should be kept or not.
        out: Output,
    },
    /// Lookup the specified instance function and put it on the stack.
    /// This might help in cases where a single instance function is called many
    /// times (like in a loop) since it avoids calculating its full hash on
    /// every iteration.
    ///
    /// Note that this does not resolve that the instance function exists, only
    /// that the instance does.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// => <fn>
    /// ```
    #[musli(packed)]
    LoadInstanceFn {
        /// The address of the instance for which the function is being loaded.
        addr: InstAddress,
        /// The name hash of the instance function.
        hash: Hash,
        /// Where to store the loaded instance function.
        out: Output,
    },
    /// Perform a function call on a function pointer stored on the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// <fn>
    /// <args...>
    /// => <ret>
    /// ```
    #[musli(packed)]
    CallFn {
        /// The address of the function being called.
        function: InstAddress,
        /// The address of the arguments being passed.
        addr: InstAddress,
        /// The number of arguments expected on the stack for this call.
        args: usize,
        /// Whether the returned value from calling the function should be kept
        /// or not.
        out: Output,
    },
    /// Perform an index get operation. Pushing the result on the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// <target>
    /// <index>
    /// => <value>
    /// ```
    #[musli(packed)]
    IndexGet {
        /// How the target is addressed.
        target: InstAddress,
        /// How the index is addressed.
        index: InstAddress,
        /// Whether the produced value should be kept or not.
        out: Output,
    },
    /// Set the given index of the tuple on the stack, with the given value.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// <tuple>
    /// => *nothing*
    /// ```
    #[musli(packed)]
    TupleIndexSet {
        /// The index to set.
        index: usize,
    },
    /// Get the given index out of a tuple from the given variable slot.
    /// Errors if the item doesn't exist or the item is not a tuple.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <value>
    /// ```
    #[musli(packed)]
    TupleIndexGetAt {
        /// The address where the tuple we are getting from is stored.
        addr: InstAddress,
        /// The index to fetch.
        index: usize,
        /// Whether the produced value should be kept or not.
        out: Output,
    },
    /// Set the given index out of an object on the top of the stack.
    /// Errors if the item doesn't exist or the item is not an object.
    ///
    /// The index is identifier by a static string slot, which is provided as an
    /// argument.
    ///
    /// # Operation
    ///
    /// ```text
    /// <object>
    /// <value>
    /// =>
    /// ```
    #[musli(packed)]
    ObjectIndexSet {
        /// The static string slot corresponding to the index to set.
        slot: usize,
    },
    /// Get the given index out of an object from the given variable slot.
    /// Errors if the item doesn't exist or the item is not an object.
    ///
    /// The index is identifier by a static string slot, which is provided as an
    /// argument.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <value>
    /// ```
    #[musli(packed)]
    ObjectIndexGetAt {
        /// The address where the object is stored.
        addr: InstAddress,
        /// The static string slot corresponding to the index to fetch.
        slot: usize,
        /// Where to store the fetched value.
        out: Output,
    },
    /// Perform an index set operation.
    ///
    /// # Operation
    ///
    /// ```text
    /// <target>
    /// <index>
    /// <value>
    /// => *noop*
    /// ```
    IndexSet,
    /// Await the future that is on the stack and push the value that it
    /// produces.
    ///
    /// # Operation
    ///
    /// ```text
    /// <future>
    /// => <value>
    /// ```
    Await {
        /// Whether the produced value from the await should be kept or not.
        out: Output,
    },
    /// Select over `len` futures on the stack. Sets the `branch` register to
    /// the index of the branch that completed. And pushes its value on the
    /// stack.
    ///
    /// This operation will block the VM until at least one of the underlying
    /// futures complete.
    ///
    /// # Operation
    ///
    /// ```text
    /// <future...>
    /// => <value>
    /// ```
    #[musli(packed)]
    Select {
        /// The number of futures to poll.
        len: usize,
        /// Where the produced value should be stored.
        out: Output,
    },
    /// Load the given function by hash and push onto the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <value>
    /// ```
    #[musli(packed)]
    LoadFn {
        /// The hash of the function to push.
        hash: Hash,
    },
    /// Push a value onto the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <value>
    /// ```
    #[musli(packed)]
    Push {
        /// The value to push.
        value: InstValue,
    },
    /// Copy a variable from a location `offset` relative to the current call
    /// frame.
    ///
    /// A copy is very cheap. It simply means pushing a reference to the stack.
    #[musli(packed)]
    Copy {
        /// Address of the value being copied.
        addr: InstAddress,
        /// Where the value is being copied to.
        out: Output,
    },
    /// Move a variable from a location `offset` relative to the current call
    /// frame.
    #[musli(packed)]
    Move {
        /// Address of the value being moved.
        addr: InstAddress,
        /// Where the value is being moved to.
        out: Output,
    },
    /// Drop the value in the given frame offset, cleaning out it's slot in
    /// memory.
    ///
    /// # Operation
    ///
    /// ```text
    /// => *noop*
    /// ```
    #[musli(packed)]
    Drop {
        /// Address of the value being dropped.
        addr: InstAddress,
    },
    /// Replace a value at the offset relative from the top of the stack, with
    /// the top of the stack.
    #[musli(packed)]
    Replace {
        /// Offset to swap value from.
        offset: usize,
    },
    /// Swap two values on the stack using their offsets relative to the current
    /// stack frame.
    #[musli(packed)]
    Swap {
        /// Offset to the first value.
        a: usize,
        /// Offset to the second value.
        b: usize,
    },
    /// Pop the current stack frame and restore the instruction pointer from it.
    ///
    /// The stack frame will be cleared, and the value on the top of the stack
    /// will be left on top of it.
    #[musli(packed)]
    Return {
        /// The address of the value to return.
        address: InstAddress,
    },
    /// Pop the current stack frame and restore the instruction pointer from it.
    ///
    /// The stack frame will be cleared, and a unit value will be pushed to the
    /// top of the stack.
    ReturnUnit,
    /// Unconditionally jump to `offset` relative to the current instruction
    /// pointer.
    ///
    /// # Operation
    ///
    /// ```text
    /// *nothing*
    /// => *nothing*
    /// ```
    #[musli(packed)]
    Jump {
        /// Offset to jump to.
        jump: usize,
    },
    /// Jump to `offset` relative to the current instruction pointer if the
    /// condition is `true`.
    ///
    /// # Operation
    ///
    /// ```text
    /// <boolean>
    /// => *nothing*
    /// ```
    #[musli(packed)]
    JumpIf {
        /// Offset to jump to.
        jump: usize,
    },
    /// Jump to the given offset If the top of the stack is false.
    ///
    /// # Operation
    ///
    /// ```text
    /// <bool>
    /// => *noop*
    /// ```
    #[musli(packed)]
    JumpIfNot {
        /// The offset to jump if the condition is true.
        jump: usize,
    },
    /// Compares the `branch` register with the top of the stack, and if they
    /// match pops the top of the stack and performs the jump to offset.
    ///
    /// # Operation
    ///
    /// ```text
    /// <integer>
    /// => *nothing*
    /// ```
    #[musli(packed)]
    JumpIfBranch {
        /// The branch value to compare against.
        branch: i64,
        /// The offset to jump.
        jump: usize,
    },
    /// Construct a push a vector value onto the stack. The number of elements
    /// in the vector are determined by `count` and are popped from the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value..>
    /// => <vec>
    /// ```
    #[musli(packed)]
    Vec {
        /// The size of the vector.
        count: usize,
    },
    /// Construct a push a one-tuple value onto the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <tuple>
    /// ```
    #[musli(packed)]
    Tuple1 {
        /// First element of the tuple.
        #[inst_display(display_with = display_array)]
        args: [InstAddress; 1],
    },
    /// Construct a push a two-tuple value onto the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <tuple>
    /// ```
    #[musli(packed)]
    Tuple2 {
        /// Tuple arguments.
        #[inst_display(display_with = display_array)]
        args: [InstAddress; 2],
    },
    /// Construct a push a three-tuple value onto the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <tuple>
    /// ```
    #[musli(packed)]
    Tuple3 {
        /// Tuple arguments.
        #[inst_display(display_with = display_array)]
        args: [InstAddress; 3],
    },
    /// Construct a push a four-tuple value onto the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <tuple>
    /// ```
    #[musli(packed)]
    Tuple4 {
        /// Tuple arguments.
        #[inst_display(display_with = display_array)]
        args: [InstAddress; 4],
    },
    /// Construct a push a tuple value onto the stack. The number of elements
    /// in the tuple are determined by `count` and are popped from the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value..>
    /// => <tuple>
    /// ```
    #[musli(packed)]
    Tuple {
        /// The size of the tuple.
        count: usize,
    },
    /// Take the tuple that is on top of the stack and push its content onto the
    /// stack.
    ///
    /// This is used to unpack an environment for closures - if the closure has
    /// an environment.
    ///
    /// # Operation
    ///
    /// ```text
    /// <tuple>
    /// => <value...>
    /// ```
    PushEnvironment {
        /// The expected size of the tuple.
        count: usize,
    },
    /// Construct a push an object onto the stack. The number of elements
    /// in the object are determined the slot of the object keys `slot` and are
    /// popped from the stack.
    ///
    /// For each element, a value is popped corresponding to the object key.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value..>
    /// => <object>
    /// ```
    #[musli(packed)]
    Object {
        /// The static slot of the object keys.
        slot: usize,
    },
    /// Construct a range. This will pop the start and end of the range from the
    /// stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// [start]
    /// [end]
    /// => <range>
    /// ```
    #[musli(packed)]
    Range {
        /// The kind of the range, which determines the number of arguments on the stack.
        range: InstRange,
    },
    /// Construct a push an object of the given type onto the stack. The type is
    /// an empty struct.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <object>
    /// ```
    #[musli(packed)]
    EmptyStruct {
        /// The type of the object to construct.
        hash: Hash,
    },
    /// Construct a push an object of the given type onto the stack. The number
    /// of elements in the object are determined the slot of the object keys
    /// `slot` and are popped from the stack.
    ///
    /// For each element, a value is popped corresponding to the object key.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value..>
    /// => <object>
    /// ```
    #[musli(packed)]
    Struct {
        /// The type of the object to construct.
        hash: Hash,
        /// The static slot of the object keys.
        slot: usize,
    },
    /// Construct a push an object variant of the given type onto the stack. The
    /// type is an empty struct.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <object>
    /// ```
    #[musli(packed)]
    UnitVariant {
        /// The type hash of the object variant to construct.
        hash: Hash,
    },
    /// Construct a push an object variant of the given type onto the stack. The
    /// number of elements in the object are determined the slot of the object
    /// keys `slot` and are popped from the stack.
    ///
    /// For each element, a value is popped corresponding to the object key.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value..>
    /// => <object>
    /// ```
    #[musli(packed)]
    StructVariant {
        /// The type hash of the object variant to construct.
        hash: Hash,
        /// The static slot of the object keys.
        slot: usize,
    },
    /// Load a literal string from a static string slot.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <string>
    /// ```
    #[musli(packed)]
    String {
        /// The static string slot to load the string from.
        slot: usize,
    },
    /// Load a literal byte string from a static byte string slot.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <bytes>
    /// ```
    #[musli(packed)]
    Bytes {
        /// The static byte string slot to load the string from.
        slot: usize,
    },
    /// Pop the given number of values from the stack, and concatenate a string
    /// from them.
    ///
    /// This is a dedicated template-string optimization.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value...>
    /// => <string>
    /// ```
    #[musli(packed)]
    StringConcat {
        /// The number of items to pop from the stack.
        len: usize,
        /// The minimum string size used.
        size_hint: usize,
    },
    /// Push a combined format specification and value onto the stack. The value
    /// used is the last value on the stack.
    #[musli(packed)]
    Format {
        /// The format specification to use.
        spec: FormatSpec,
    },
    /// Test if the top of the stack is a unit.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// => <boolean>
    /// ```
    IsUnit,
    /// Perform the try operation which takes the value at the given `address`
    /// and tries to unwrap it or return from the current call frame.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// => <boolean>
    /// ```
    #[musli(packed)]
    Try {
        /// Address to test if value.
        address: InstAddress,
        /// If the value on top of the stack should be preserved.
        preserve: bool,
    },
    /// Test if the top of the stack is a specific byte.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// => <boolean>
    /// ```
    #[musli(packed)]
    EqByte {
        /// The byte to test against.
        byte: u8,
    },
    /// Test if the top of the stack is a specific character.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// => <boolean>
    /// ```
    #[musli(packed)]
    EqChar {
        /// The character to test against.
        char: char,
    },
    /// Test if the top of the stack is a specific integer.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// => <boolean>
    /// ```
    #[musli(packed)]
    EqInteger {
        /// The integer to test against.
        integer: i64,
    },

    /// Test if the top of the stack is a specific boolean.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// => <boolean>
    /// ```
    #[musli(packed)]
    EqBool {
        /// The bool to test against.
        boolean: bool,
    },
    /// Compare the top of the stack against a static string slot.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// => <boolean>
    /// ```
    #[musli(packed)]
    EqString {
        /// The slot to test against.
        slot: usize,
    },
    /// Compare the top of the stack against a static bytes slot.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// => <boolean>
    /// ```
    #[musli(packed)]
    EqBytes {
        /// The slot to test against.
        slot: usize,
    },
    /// Test that the top of the stack has the given type.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// => <boolean>
    /// ```
    #[musli(packed)]
    MatchType {
        /// The type hash to match against.
        hash: Hash,
    },
    /// Test if the specified variant matches. This is distinct from
    /// [Inst::MatchType] because it will match immediately on the variant type
    /// if appropriate which is possible for internal types, but external types
    /// will require an additional runtime check for matching.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// => <boolean>
    /// ```
    #[musli(packed)]
    MatchVariant {
        /// The exact type hash of the variant.
        variant_hash: Hash,
        /// The container type.
        enum_hash: Hash,
        /// The index of the variant.
        index: usize,
    },
    /// Test if the top of the stack is the given builtin type or variant.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// => <boolean>
    /// ```
    #[musli(packed)]
    MatchBuiltIn {
        /// The type to check for.
        type_check: TypeCheck,
    },
    /// Test that the top of the stack is a tuple with the given length
    /// requirements.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// => <boolean>
    /// ```
    #[musli(packed)]
    MatchSequence {
        /// Type constraints that the sequence must match.
        type_check: TypeCheck,
        /// The minimum length to test for.
        len: usize,
        /// Whether the operation should check exact `true` or minimum length
        /// `false`.
        exact: bool,
    },
    /// Test that the top of the stack is an object matching the given slot of
    /// object keys.
    ///
    /// # Operation
    ///
    /// ```text
    /// <object>
    /// => <boolean>
    /// ```
    #[musli(packed)]
    MatchObject {
        /// The slot of object keys to use.
        slot: usize,
        /// Whether the operation should check exact `true` or minimum length
        /// `false`.
        exact: bool,
    },
    /// Perform a generator yield where the value yielded is expected to be
    /// found at the top of the stack.
    ///
    /// This causes the virtual machine to suspend itself.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// => <value>
    /// ```
    Yield {
        /// Whether the produced value from the yield should be kept or not.
        out: Output,
    },
    /// Perform a generator yield with a unit.
    ///
    /// This causes the virtual machine to suspend itself.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <unit>
    /// ```
    YieldUnit {
        /// Whether the produced value from the yield should be kept or not.
        out: Output,
    },
    /// Construct a built-in variant onto the stack.
    ///
    /// The variant will pop as many values of the stack as necessary to
    /// construct it.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value..>
    /// => <variant>
    /// ```
    #[musli(packed)]
    Variant {
        /// The kind of built-in variant to construct.
        variant: InstVariant,
    },
    /// A built-in operation like `a + b` that takes its operands and pushes its
    /// result to and from the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <value>
    /// ```
    #[musli(packed)]
    Op {
        /// The actual operation.
        op: InstOp,
        /// The address of the first argument.
        a: InstAddress,
        /// The address of the second argument.
        b: InstAddress,
        /// Whether the produced value from the operation should be kept or not.
        out: Output,
    },
    /// A built-in operation that assigns to the left-hand side operand. Like
    /// `a += b`.
    ///
    /// The target determines the left hand side operation.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// =>
    /// ```
    #[musli(packed)]
    Assign {
        /// The target of the operation.
        target: InstTarget,
        /// The actual operation.
        op: InstAssignOp,
    },
    /// Advance an iterator at the given position.
    #[musli(packed)]
    IterNext {
        /// The offset of the value being advanced.
        offset: usize,
        /// A relative jump to perform if the iterator could not be advanced.
        jump: usize,
    },
    /// Cause the VM to panic and error out without a reason.
    ///
    /// This should only be used during testing or extreme scenarios that are
    /// completely unrecoverable.
    #[musli(packed)]
    Panic {
        /// The reason for the panic.
        #[inst_display(display_with = PanicReason::ident)]
        reason: PanicReason,
    },
}

impl Inst {
    /// Construct an instruction to push a unit.
    pub fn unit() -> Self {
        Self::Push {
            value: InstValue::EmptyTuple,
        }
    }

    /// Construct an instruction to push a boolean.
    pub fn bool(b: bool) -> Self {
        Self::Push {
            value: InstValue::Bool(b),
        }
    }

    /// Construct an instruction to push a byte.
    pub fn byte(b: u8) -> Self {
        Self::Push {
            value: InstValue::Byte(b),
        }
    }

    /// Construct an instruction to push a character.
    pub fn char(c: char) -> Self {
        Self::Push {
            value: InstValue::Char(c),
        }
    }

    /// Construct an instruction to push an integer.
    pub fn integer(v: i64) -> Self {
        Self::Push {
            value: InstValue::Integer(v),
        }
    }

    /// Construct an instruction to push a float.
    pub fn float(v: f64) -> Self {
        Self::Push {
            value: InstValue::Float(v),
        }
    }
}

/// The calling convention of a function.
#[derive(
    Debug, TryClone, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode,
)]
#[try_clone(copy)]
#[non_exhaustive]
enum OutputKind {
    /// Push the produced value onto the stack.
    Keep(NonZeroUsize),
    /// Discard the produced value, leaving the stack unchanged.
    Discard,
}

/// The calling convention of a function.
#[derive(
    Debug, TryClone, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode,
)]
#[try_clone(copy)]
#[non_exhaustive]
#[musli(transparent)]
#[serde(transparent)]
pub struct Output {
    kind: OutputKind,
}

impl Output {
    /// Construct a keep output kind.
    #[inline]
    pub(crate) fn keep(index: usize) -> Self {
        let Some(index) = NonZeroUsize::new(index ^ usize::MAX) else {
            panic!("Index {index} is out of bounds")
        };

        Self {
            kind: OutputKind::Keep(index),
        }
    }

    /// Construct a discard output kind.
    #[inline]
    pub(crate) fn discard() -> Self {
        Self {
            kind: OutputKind::Discard,
        }
    }

    /// Check if the output is a keep.
    #[inline]
    pub(crate) fn into_keep(&self) -> Option<InstAddress> {
        match self.kind {
            OutputKind::Keep(index) => Some(InstAddress::new(index.get() ^ usize::MAX)),
            OutputKind::Discard => None,
        }
    }

    /// Write the current output to the provided stack.
    #[inline]
    pub fn store<O>(self, stack: &mut Stack, o: O) -> VmResult<()>
    where
        O: IntoResult<Output: TryInto<Value, Error: Into<VmErrorKind>>>,
    {
        if let Some(index) = self.into_keep() {
            let value = vm_try!(o.into_result());
            *vm_try!(stack.at_mut(index)) = vm_try!(value.try_into().map_err(Into::into));
        }

        VmResult::Ok(())
    }
}

/// Trait used to store a value.
pub trait IntoResult {
    #[doc(hidden)]
    type Output;

    /// Coerce into result.
    fn into_result(self) -> VmResult<Self::Output>;
}

impl<F, O> IntoResult for F
where
    F: FnOnce() -> O,
    O: IntoResult,
{
    type Output = O::Output;

    #[inline]
    fn into_result(self) -> VmResult<Self::Output> {
        self().into_result()
    }
}

impl<T, E> IntoResult for Result<T, E>
where
    VmErrorKind: From<E>,
{
    type Output = T;

    #[inline]
    fn into_result(self) -> VmResult<Self::Output> {
        VmResult::Ok(vm_try!(self))
    }
}

impl<T> IntoResult for VmResult<T> {
    type Output = T;

    #[inline]
    fn into_result(self) -> VmResult<Self::Output> {
        self
    }
}

impl IntoResult for Value {
    type Output = Value;

    #[inline]
    fn into_result(self) -> VmResult<Self::Output> {
        VmResult::Ok(self)
    }
}

impl IntoResult for bool {
    type Output = bool;

    #[inline]
    fn into_result(self) -> VmResult<Self::Output> {
        VmResult::Ok(self)
    }
}

impl IntoResult for u8 {
    type Output = u8;

    #[inline]
    fn into_result(self) -> VmResult<Self::Output> {
        VmResult::Ok(self)
    }
}

impl IntoResult for i64 {
    type Output = i64;

    #[inline]
    fn into_result(self) -> VmResult<Self::Output> {
        VmResult::Ok(self)
    }
}

impl IntoResult for f64 {
    type Output = f64;

    #[inline]
    fn into_result(self) -> VmResult<Self::Output> {
        VmResult::Ok(self)
    }
}

impl IntoResult for () {
    type Output = ();

    #[inline]
    fn into_result(self) -> VmResult<Self::Output> {
        VmResult::Ok(self)
    }
}

impl IntoResult for ValueKind {
    type Output = ValueKind;

    #[inline]
    fn into_result(self) -> VmResult<Self::Output> {
        VmResult::Ok(self)
    }
}

impl fmt::Display for Output {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            OutputKind::Keep(index) => write!(f, "keep({})", index.get() ^ usize::MAX),
            OutputKind::Discard => write!(f, "discard"),
        }
    }
}

/// How an instruction addresses a value.
#[derive(Default, Debug, TryClone, Clone, Copy, Serialize, Deserialize, Decode, Encode)]
#[try_clone(copy)]
pub struct InstAddress {
    offset: usize,
}

impl InstAddress {
    /// The first possible address.
    pub(crate) const FIRST: InstAddress = InstAddress { offset: 0 };

    /// Construct a new instruction address.
    #[inline]
    pub(crate) const fn new(offset: usize) -> Self {
        Self { offset }
    }

    /// Get the offset of the address.
    #[inline]
    pub(crate) fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for InstAddress {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.offset.fmt(f)
    }
}

/// Range limits of a range expression.
#[derive(Debug, TryClone, Clone, Copy, Serialize, Deserialize, Decode, Encode)]
#[try_clone(copy)]
pub enum InstRange {
    /// `start..`.
    RangeFrom,
    /// `..`.
    RangeFull,
    /// `start..=end`.
    RangeInclusive,
    /// `..=end`.
    RangeToInclusive,
    /// `..end`.
    RangeTo,
    /// `start..end`.
    Range,
}

impl fmt::Display for InstRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstRange::RangeFrom => write!(f, "start.."),
            InstRange::RangeFull => write!(f, ".."),
            InstRange::RangeInclusive => write!(f, "start..=end"),
            InstRange::RangeToInclusive => write!(f, "..=end"),
            InstRange::RangeTo => write!(f, "..end"),
            InstRange::Range => write!(f, "start..end"),
        }
    }
}

/// The target of an operation.
#[derive(Debug, TryClone, Clone, Copy, Serialize, Deserialize, Encode, Decode)]
#[try_clone(copy)]
pub enum InstTarget {
    /// Target is an offset to the current call frame.
    #[musli(packed)]
    Offset(usize),
    /// Target the field of an object.
    #[musli(packed)]
    Field(usize),
    /// Target a tuple field.
    #[musli(packed)]
    TupleField(usize),
}

impl fmt::Display for InstTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Offset(offset) => write!(f, "offset({offset})"),
            Self::Field(slot) => write!(f, "field({slot})"),
            Self::TupleField(slot) => write!(f, "tuple-field({slot})"),
        }
    }
}

/// An operation between two values on the machine.
#[derive(Debug, TryClone, Clone, Copy, Serialize, Deserialize, Decode, Encode)]
#[try_clone(copy)]
pub enum InstAssignOp {
    /// The add operation. `a + b`.
    Add,
    /// The sub operation. `a - b`.
    Sub,
    /// The multiply operation. `a * b`.
    Mul,
    /// The division operation. `a / b`.
    Div,
    /// The remainder operation. `a % b`.
    Rem,
    /// The bitwise and operation. `a & b`.
    BitAnd,
    /// The bitwise xor operation. `a ^ b`.
    BitXor,
    /// The bitwise or operation. `a | b`.
    BitOr,
    /// The shift left operation. `a << b`.
    Shl,
    /// The shift right operation. `a << b`.
    Shr,
}

impl fmt::Display for InstAssignOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Add => {
                write!(f, "+")?;
            }
            Self::Sub => {
                write!(f, "-")?;
            }
            Self::Mul => {
                write!(f, "*")?;
            }
            Self::Div => {
                write!(f, "/")?;
            }
            Self::Rem => {
                write!(f, "%")?;
            }
            Self::BitAnd => {
                write!(f, "&")?;
            }
            Self::BitXor => {
                write!(f, "^")?;
            }
            Self::BitOr => {
                write!(f, "|")?;
            }
            Self::Shl => {
                write!(f, "<<")?;
            }
            Self::Shr => {
                write!(f, ">>")?;
            }
        }

        Ok(())
    }
}

/// An operation between two values on the machine.
#[derive(Debug, TryClone, Clone, Copy, Serialize, Deserialize, Decode, Encode)]
#[try_clone(copy)]
pub enum InstOp {
    /// The add operation. `a + b`.
    Add,
    /// The sub operation. `a - b`.
    Sub,
    /// The multiply operation. `a * b`.
    Mul,
    /// The division operation. `a / b`.
    Div,
    /// The remainder operation. `a % b`.
    Rem,
    /// The bitwise and operation. `a & b`.
    BitAnd,
    /// The bitwise xor operation. `a ^ b`.
    BitXor,
    /// The bitwise or operation. `a | b`.
    BitOr,
    /// The shift left operation. `a << b`.
    Shl,
    /// The shift right operation. `a << b`.
    Shr,
    /// Compare two values on the stack for lt and push the result as a
    /// boolean on the stack.
    Lt,
    /// Compare two values on the stack for gt and push the result as a
    /// boolean on the stack.
    Gt,
    /// Compare two values on the stack for lte and push the result as a
    /// boolean on the stack.
    Lte,
    /// Compare two values on the stack for gte and push the result as a
    /// boolean on the stack.
    Gte,
    /// Compare two values on the stack for equality and push the result as a
    /// boolean on the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// <b>
    /// <a>
    /// => <bool>
    /// ```
    Eq,
    /// Compare two values on the stack for inequality and push the result as a
    /// boolean on the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// <b>
    /// <a>
    /// => <bool>
    /// ```
    Neq,
    /// Coerce a value into the given type.
    ///
    /// # Operation
    ///
    /// ```text
    /// <type>
    /// <value>
    /// => <boolean>
    /// ```
    As,
    /// Test if the top of the stack is an instance of the second item on the
    /// stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// <type>
    /// <value>
    /// => <boolean>
    /// ```
    Is,
    /// Test if the top of the stack is not an instance of the second item on
    /// the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// <type>
    /// <value>
    /// => <boolean>
    /// ```
    IsNot,
    /// Pop two values from the stack and test if they are both boolean true.
    ///
    /// # Operation
    ///
    /// ```text
    /// <boolean>
    /// <boolean>
    /// => <boolean>
    /// ```
    And,
    /// Pop two values from the stack and test if either of them are boolean
    /// true.
    ///
    /// # Operation
    ///
    /// ```text
    /// <boolean>
    /// <boolean>
    /// => <boolean>
    /// ```
    Or,
}

impl fmt::Display for InstOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Add => {
                write!(f, "+")?;
            }
            Self::Sub => {
                write!(f, "-")?;
            }
            Self::Mul => {
                write!(f, "*")?;
            }
            Self::Div => {
                write!(f, "/")?;
            }
            Self::Rem => {
                write!(f, "%")?;
            }
            Self::BitAnd => {
                write!(f, "&")?;
            }
            Self::BitXor => {
                write!(f, "^")?;
            }
            Self::BitOr => {
                write!(f, "|")?;
            }
            Self::Shl => {
                write!(f, "<<")?;
            }
            Self::Shr => {
                write!(f, ">>")?;
            }
            Self::Lt => {
                write!(f, "<")?;
            }
            Self::Gt => {
                write!(f, ">")?;
            }
            Self::Lte => {
                write!(f, "<=")?;
            }
            Self::Gte => {
                write!(f, ">=")?;
            }
            Self::Eq => {
                write!(f, "==")?;
            }
            Self::Neq => {
                write!(f, "!=")?;
            }
            Self::As => {
                write!(f, "as")?;
            }
            Self::Is => {
                write!(f, "is")?;
            }
            Self::IsNot => {
                write!(f, "is not")?;
            }
            Self::And => {
                write!(f, "&&")?;
            }
            Self::Or => {
                write!(f, "||")?;
            }
        }

        Ok(())
    }
}

/// A literal value that can be pushed.
#[derive(Debug, TryClone, Clone, Copy, Serialize, Deserialize, Decode, Encode)]
#[try_clone(copy)]
#[non_exhaustive]
pub enum InstValue {
    /// An empty tuple.
    EmptyTuple,
    /// A boolean.
    #[musli(packed)]
    Bool(bool),
    /// A byte.
    #[musli(packed)]
    Byte(u8),
    /// A character.
    #[musli(packed)]
    Char(char),
    /// An integer.
    #[musli(packed)]
    Integer(i64),
    /// A float.
    #[musli(packed)]
    Float(f64),
    /// A type hash.
    #[musli(packed)]
    Type(Type),
}

impl InstValue {
    /// Convert into a value that can be pushed onto the stack.
    pub fn into_value(self) -> alloc::Result<Value> {
        match self {
            Self::EmptyTuple => Value::empty(),
            Self::Bool(v) => Value::try_from(v),
            Self::Byte(v) => Value::try_from(v),
            Self::Char(v) => Value::try_from(v),
            Self::Integer(v) => Value::try_from(v),
            Self::Float(v) => Value::try_from(v),
            Self::Type(v) => Value::try_from(v),
        }
    }
}

impl fmt::Display for InstValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTuple => write!(f, "()")?,
            Self::Bool(v) => write!(f, "{}", v)?,
            Self::Byte(v) => {
                if v.is_ascii_graphic() {
                    write!(f, "b'{}'", *v as char)?
                } else {
                    write!(f, "b'\\x{:02x}'", v)?
                }
            }
            Self::Char(v) => write!(f, "{:?}", v)?,
            Self::Integer(v) => write!(f, "{}", v)?,
            Self::Float(v) => write!(f, "{}", v)?,
            Self::Type(v) => write!(f, "{}", v.into_hash())?,
        }

        Ok(())
    }
}

/// A variant that can be constructed.
#[derive(Debug, TryClone, Clone, Copy, Serialize, Deserialize, Decode, Encode)]
#[try_clone(copy)]
pub enum InstVariant {
    /// `Option::Some`, which uses one value.
    Some,
    /// `Option::None`, which uses no values.
    None,
    /// `Result::Ok`, which uses one value.
    Ok,
    /// `Result::Err`, which uses one value.
    Err,
}

impl fmt::Display for InstVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Some => {
                write!(f, "Some")?;
            }
            Self::None => {
                write!(f, "None")?;
            }
            Self::Ok => {
                write!(f, "Ok")?;
            }
            Self::Err => {
                write!(f, "Err")?;
            }
        }

        Ok(())
    }
}

fn display_array<T>(array: &[T]) -> impl fmt::Display + '_
where
    T: fmt::Display,
{
    DisplayArray(array)
}

struct DisplayArray<'a, T>(&'a [T]);

impl<T> fmt::Display for DisplayArray<'_, T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut it = self.0.iter();

        write!(f, "[")?;
        let last = it.next_back();

        for value in it {
            write!(f, "{value}, ")?;
        }

        if let Some(last) = last {
            last.fmt(f)?;
        }

        write!(f, "]")?;
        Ok(())
    }
}
