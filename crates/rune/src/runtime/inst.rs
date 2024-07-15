use core::fmt;
use core::num::NonZeroUsize;

use musli::{Decode, Encode};
use rune_macros::InstDisplay;
use serde::{Deserialize, Serialize};

use crate as rune;
use crate::alloc;
use crate::alloc::prelude::*;
use crate::runtime::{Call, FormatSpec, Stack, Type, Value, ValueKind, VmError, VmResult};
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
    /// Make sure that the memory region has `size` slots of memory available.
    Allocate {
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
        addr: InstAddress,
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
        /// The operand to negate.
        addr: InstAddress,
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
        /// Where to load captured values from.
        addr: InstAddress,
        /// The number of captured values to store in the environment.
        count: usize,
        /// Where to store the produced closure.
        out: Output,
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
        /// The number of arguments passed in at `addr`.
        args: usize,
        /// Whether the return value should be kept or not.
        out: Output,
    },
    /// Call a function by hash.
    ///
    /// The function will be looked up in the unit and context. The arguments
    /// passed to the function call are stored at `addr`, where `size`
    /// determines the number of arguments.
    ///
    /// The return value of the function call will be written to `out`.
    #[musli(packed)]
    Call {
        /// The hash of the function to call.
        hash: Hash,
        /// The address of the arguments being passed.
        addr: InstAddress,
        /// The number of arguments passed in at `addr`.
        args: usize,
        /// Whether the return value should be kept or not.
        out: Output,
    },
    /// Call an associated function.
    ///
    /// The instance being called should be the the object at address `addr`.
    /// The number of arguments specified should include this object.
    ///
    /// The return value of the function call will be written to `out`.
    #[musli(packed)]
    CallAssociated {
        /// The hash of the name of the function to call.
        hash: Hash,
        /// The address of arguments being passed.
        addr: InstAddress,
        /// The number of arguments passed in at `addr`.
        args: usize,
        /// Whether the return value should be kept or not.
        out: Output,
    },
    /// Look up an instance function.
    ///
    /// The instance being used is stored at `addr`, and the function hash to look up is `hash`.
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
        /// The number of arguments passed in at `addr`.
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
        /// The object being assigned to.
        target: InstAddress,
        /// The index to set.
        index: usize,
        /// The value being assigned.
        value: InstAddress,
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
        /// The object being assigned to.
        target: InstAddress,
        /// The static string slot corresponding to the index to set.
        slot: usize,
        /// The value being assigned.
        value: InstAddress,
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
    IndexSet {
        /// The object being assigned to.
        target: InstAddress,
        /// The index to set.
        index: InstAddress,
        /// The value being assigned.
        value: InstAddress,
    },
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
        /// Address of the future being awaited.
        addr: InstAddress,
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
        /// The base address of futures being waited on.
        addr: InstAddress,
        /// The number of futures to poll.
        len: usize,
        /// Where to store the branch value.
        branch: Output,
        /// Where to store the value produced by the future that completed.
        value: Output,
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
        /// Where to store the loaded function.
        out: Output,
    },
    /// Push a value onto the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <value>
    /// ```
    #[musli(packed)]
    Store {
        /// The value to push.
        value: InstValue,
        /// Where the value is being copied to.
        out: Output,
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
    /// Swap two values on the stack using their offsets relative to the current
    /// stack frame.
    #[musli(packed)]
    Swap {
        /// Offset to the first value.
        a: InstAddress,
        /// Offset to the second value.
        b: InstAddress,
    },
    /// Pop the current stack frame and restore the instruction pointer from it.
    ///
    /// The stack frame will be cleared, and the value on the top of the stack
    /// will be left on top of it.
    #[musli(packed)]
    Return {
        /// The address of the value to return.
        addr: InstAddress,
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
        /// The address of the condition for the jump.
        cond: InstAddress,
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
        /// The address of the condition for the jump.
        cond: InstAddress,
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
        /// Where the branch value is stored.
        branch: InstAddress,
        /// The branch value to compare against.
        value: i64,
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
        /// Where the arguments to the vector are stored.
        addr: InstAddress,
        /// The number of elements in the vector.
        count: usize,
        /// Where to store the produced vector.
        out: Output,
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
        #[inst_display(display_with = DisplayArray::new)]
        args: [InstAddress; 1],
        /// Where to store the produced tuple.
        out: Output,
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
        #[inst_display(display_with = DisplayArray::new)]
        args: [InstAddress; 2],
        /// Where to store the produced tuple.
        out: Output,
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
        #[inst_display(display_with = DisplayArray::new)]
        args: [InstAddress; 3],
        /// Where to store the produced tuple.
        out: Output,
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
        #[inst_display(display_with = DisplayArray::new)]
        args: [InstAddress; 4],
        /// Where to store the produced tuple.
        out: Output,
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
        /// Where the arguments to the tuple are stored.
        addr: InstAddress,
        /// The number of elements in the tuple.
        count: usize,
        /// Where to store the produced tuple.
        out: Output,
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
    Environment {
        /// The tuple to push.
        addr: InstAddress,
        /// The expected size of the tuple.
        count: usize,
        /// Where to unpack the environment.
        out: Output,
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
        /// Where the arguments to the tuple are stored.
        addr: InstAddress,
        /// The static slot of the object keys.
        slot: usize,
        /// Where to store the produced tuple.
        out: Output,
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
        /// Where the arguments of the range are stored.
        addr: InstAddress,
        /// Where to store the produced range.
        out: Output,
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
        /// Where to write the empty struct.
        out: Output,
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
        /// The address to load fields from.
        addr: InstAddress,
        /// The type of the object to construct.
        hash: Hash,
        /// The static slot of the object keys.
        slot: usize,
        /// Where to write the constructed struct.
        out: Output,
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
        /// The address to load fields from.
        addr: InstAddress,
        /// The type hash of the object variant to construct.
        hash: Hash,
        /// The static slot of the object keys.
        slot: usize,
        /// Where to write the constructed variant.
        out: Output,
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
        /// Where to store the string.
        out: Output,
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
        /// Where to store the bytes.
        out: Output,
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
        /// Where the strings to concatenate are stored.
        addr: InstAddress,
        /// The number of items to pop from the stack.
        len: usize,
        /// The minimum string size used.
        size_hint: usize,
        /// Where to store the produced string.
        out: Output,
    },
    /// Push a combined format specification and value onto the stack. The value
    /// used is the last value on the stack.
    #[musli(packed)]
    Format {
        /// Address of the value being formatted.
        addr: InstAddress,
        /// The format specification to use.
        spec: FormatSpec,
        /// Where to store the produced format.
        out: Output,
    },
    /// Test if the top of the stack is a unit.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// => <boolean>
    /// ```
    IsUnit {
        /// The address of the value to test.
        addr: InstAddress,
        /// Where to store the output.
        out: Output,
    },
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
        /// Address of value to try.
        addr: InstAddress,
        /// Where to store the value in case there is a continuation.
        out: Output,
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
        /// Address of the value to compare.
        addr: InstAddress,
        /// The byte to test against.
        #[inst_display(display_with = DisplayDebug::new)]
        value: u8,
        /// Where to store the result of the comparison.
        out: Output,
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
        /// Address of the value to compare.
        addr: InstAddress,
        /// The character to test against.
        #[inst_display(display_with = DisplayDebug::new)]
        value: char,
        /// Where to store the result of the comparison.
        out: Output,
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
        /// Address of the value to compare.
        addr: InstAddress,
        /// The value to test against.
        value: i64,
        /// Where to store the result of the comparison.
        out: Output,
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
        /// Address of the value to compare.
        addr: InstAddress,
        /// The value to test against.
        value: bool,
        /// Where to store the result of the comparison.
        out: Output,
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
        /// Address of the value to compare.
        addr: InstAddress,
        /// The slot to test against.
        slot: usize,
        /// Where to store the result of the comparison.
        out: Output,
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
        /// Address of the value to compare.
        addr: InstAddress,
        /// The slot to test against.
        slot: usize,
        /// Where to store the result of the comparison.
        out: Output,
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
        /// The address of the value to test.
        addr: InstAddress,
        /// Where to store the output.
        out: Output,
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
        /// The address of the value to test.
        addr: InstAddress,
        /// Where to store the output.
        out: Output,
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
        /// The address of the value to test.
        addr: InstAddress,
        /// Where to store the output.
        out: Output,
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
        /// The address of the value to test.
        addr: InstAddress,
        /// Where to store the output.
        out: Output,
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
        /// The address of the value to test.
        addr: InstAddress,
        /// Where to store the output.
        out: Output,
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
        /// Address of the value being yielded.
        addr: InstAddress,
        /// Where to store the produced resume value.
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
        /// Where to store the produced resume value.
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
        /// Where the arguments to construct the variant are stored.
        addr: InstAddress,
        /// The kind of built-in variant to construct.
        variant: InstVariant,
        /// Where to store the variant.
        out: Output,
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
        /// The value being assigned.
        value: InstAddress,
    },
    /// Advance an iterator at the given position.
    #[musli(packed)]
    IterNext {
        /// The address of the iterator to advance.
        addr: InstAddress,
        /// A relative jump to perform if the iterator could not be advanced.
        jump: usize,
        /// Where to store the produced value from the iterator.
        out: Output,
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
    pub fn unit(out: Output) -> Self {
        Self::Store {
            value: InstValue::EmptyTuple,
            out,
        }
    }

    /// Construct an instruction to push a boolean.
    pub fn bool(b: bool, out: Output) -> Self {
        Self::Store {
            value: InstValue::Bool(b),
            out,
        }
    }

    /// Construct an instruction to push a byte.
    pub fn byte(b: u8, out: Output) -> Self {
        Self::Store {
            value: InstValue::Byte(b),
            out,
        }
    }

    /// Construct an instruction to push a character.
    pub fn char(c: char, out: Output) -> Self {
        Self::Store {
            value: InstValue::Char(c),
            out,
        }
    }

    /// Construct an instruction to push an integer.
    pub fn integer(v: i64, out: Output) -> Self {
        Self::Store {
            value: InstValue::Integer(v),
            out,
        }
    }

    /// Construct an instruction to push a float.
    pub fn float(v: f64, out: Output) -> Self {
        Self::Store {
            value: InstValue::Float(v),
            out,
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

/// What to do with the output of an instruction.
#[derive(TryClone, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
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
    pub(crate) fn as_addr(&self) -> Option<InstAddress> {
        match self.kind {
            OutputKind::Keep(index) => Some(InstAddress::new(index.get() ^ usize::MAX)),
            OutputKind::Discard => None,
        }
    }

    /// Write output using the provided [`IntoOutput`] implementation onto the
    /// stack.
    ///
    /// The [`IntoOutput`] trait primarily allows for deferring a computation
    /// since it's implemented by [`FnOnce`]. However, you must take care that
    /// any side effects calling a function may have are executed outside of the
    /// call to `store`. Like if the function would error.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::{Output, Stack, ToValue, VmResult, InstAddress};
    /// use rune::vm_try;
    ///
    /// fn sum(stack: &mut Stack, addr: InstAddress, args: usize, out: Output) -> VmResult<()> {
    ///     let mut number = 0;
    ///
    ///     for value in vm_try!(stack.slice_at(addr, args)) {
    ///         number += vm_try!(value.as_integer());
    ///     }
    ///
    ///     out.store(stack, number);
    ///     VmResult::Ok(())
    /// }
    #[inline]
    pub fn store<O>(self, stack: &mut Stack, o: O) -> VmResult<()>
    where
        O: IntoOutput<Output: TryInto<Value, Error: Into<VmError>>>,
    {
        if let Some(index) = self.as_addr() {
            let value = vm_try!(o.into_output());
            *vm_try!(stack.at_mut(index)) = vm_try!(value.try_into().map_err(Into::into));
        }

        VmResult::Ok(())
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

impl fmt::Debug for Output {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// Trait used to coerce values into outputs.
pub trait IntoOutput {
    #[doc(hidden)]
    type Output;

    /// Coerce the current value into an output.
    fn into_output(self) -> VmResult<Self::Output>;
}

impl<F, O> IntoOutput for F
where
    F: FnOnce() -> O,
    O: IntoOutput,
{
    type Output = O::Output;

    #[inline]
    fn into_output(self) -> VmResult<Self::Output> {
        self().into_output()
    }
}

impl<T, E> IntoOutput for Result<T, E>
where
    VmError: From<E>,
{
    type Output = T;

    #[inline]
    fn into_output(self) -> VmResult<Self::Output> {
        VmResult::Ok(vm_try!(self))
    }
}

impl<T> IntoOutput for VmResult<T> {
    type Output = T;

    #[inline]
    fn into_output(self) -> VmResult<Self::Output> {
        self
    }
}

impl IntoOutput for Value {
    type Output = Value;

    #[inline]
    fn into_output(self) -> VmResult<Self::Output> {
        VmResult::Ok(self)
    }
}

impl IntoOutput for ValueKind {
    type Output = ValueKind;

    #[inline]
    fn into_output(self) -> VmResult<Self::Output> {
        VmResult::Ok(self)
    }
}

/// How an instruction addresses a value.
#[derive(Default, TryClone, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Decode, Encode)]
#[try_clone(copy)]
pub struct InstAddress {
    offset: usize,
}

impl InstAddress {
    /// The first possible address.
    pub const ZERO: InstAddress = InstAddress { offset: 0 };

    /// An invalid address.
    pub const INVALID: InstAddress = InstAddress { offset: usize::MAX };

    /// Construct a new address.
    #[inline]
    pub(crate) const fn new(offset: usize) -> Self {
        Self { offset }
    }

    /// Get the offset of the address.
    #[inline]
    pub(crate) fn offset(self) -> usize {
        self.offset
    }

    /// Get the address as an output.
    #[inline]
    pub(crate) fn output(self) -> Output {
        Output::keep(self.offset)
    }
}

impl fmt::Display for InstAddress {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.offset == usize::MAX {
            write!(f, "invalid")
        } else {
            self.offset.fmt(f)
        }
    }
}

impl fmt::Debug for InstAddress {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
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
    Address(InstAddress),
    /// Target the field of an object.
    #[musli(packed)]
    Field(InstAddress, usize),
    /// Target a tuple field.
    #[musli(packed)]
    TupleField(InstAddress, usize),
}

impl fmt::Display for InstTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Address(addr) => write!(f, "address({addr})"),
            Self::Field(addr, slot) => write!(f, "field({addr}, {slot})"),
            Self::TupleField(addr, slot) => write!(f, "tuple-field({addr}, {slot})"),
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

#[repr(transparent)]
struct DisplayArray<T>(T)
where
    T: ?Sized;

impl<T> DisplayArray<[T]> {
    #[inline]
    fn new(value: &[T]) -> &Self {
        // SAFETY: The `DisplayArray` struct is a transparent wrapper around the
        // value.
        unsafe { &*(value as *const [T] as *const Self) }
    }
}

impl<T> fmt::Display for DisplayArray<[T]>
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

#[repr(transparent)]
struct DisplayDebug<T>(T)
where
    T: ?Sized;

impl<T> DisplayDebug<T>
where
    T: ?Sized,
{
    #[inline]
    fn new(value: &T) -> &Self {
        // SAFETY: The `DisplayDebug` struct is a transparent wrapper around the
        // value.
        unsafe { &*(value as *const T as *const Self) }
    }
}

impl<T> fmt::Display for DisplayDebug<T>
where
    T: ?Sized + fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}
