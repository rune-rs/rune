use core::cmp::Ordering;
use core::fmt;

#[cfg(feature = "musli")]
use musli::{Decode, Encode};
use rune_macros::InstDisplay;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate as rune;
use crate::alloc::prelude::*;
use crate::Hash;

use super::{Call, FormatSpec, Memory, RuntimeError, Type, Value};

/// Pre-canned panic reasons.
///
/// To formulate a custom reason, use
/// [`VmError::panic`][crate::runtime::VmError::panic].
#[derive(Debug, TryClone, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "musli", derive(Decode, Encode))]
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
#[derive(Debug, TryClone, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "musli", derive(Decode, Encode))]
#[try_clone(copy)]
#[non_exhaustive]
pub enum TypeCheck {
    /// Matches a unit type.
    Unit,
    /// Matches an anonymous tuple.
    Tuple,
    /// Matches an anonymous object.
    Object,
    /// Matches a vector.
    Vec,
}

impl fmt::Display for TypeCheck {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unit => write!(fmt, "Unit"),
            Self::Tuple => write!(fmt, "Tuple"),
            Self::Object => write!(fmt, "Object"),
            Self::Vec => write!(fmt, "Vec"),
        }
    }
}

/// An operation in the stack-based virtual machine.
#[derive(Debug, TryClone, Clone, Copy, InstDisplay)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "musli", derive(Decode, Encode))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
    /// determines the number of arguments. The arguments will be dropped.
    ///
    /// The return value of the function call will be written to `out`.
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
    /// Select over `len` futures stored at address `addr`.
    ///
    /// Once a branch has been matched, will store the branch that matched in
    /// the branch register and perform a jump by the index of the branch that
    /// matched.
    ///
    /// Will also store the output if the future into `value`. If no branch
    /// matched, the empty value will be stored.
    #[cfg_attr(feature = "musli", musli(packed))]
    Select {
        /// The base address of futures being waited on.
        addr: InstAddress,
        /// The number of futures to poll.
        len: usize,
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
    Copy {
        /// Address of the value being copied.
        addr: InstAddress,
        /// Where the value is being copied to.
        out: Output,
    },
    /// Move a variable from a location `offset` relative to the current call
    /// frame.
    #[cfg_attr(feature = "musli", musli(packed))]
    Move {
        /// Address of the value being moved.
        addr: InstAddress,
        /// Where the value is being moved to.
        out: Output,
    },
    /// Drop the given value set.
    #[cfg_attr(feature = "musli", musli(packed))]
    Drop {
        /// An indicator of the set of addresses to drop.
        set: usize,
    },
    /// Swap two values on the stack using their offsets relative to the current
    /// stack frame.
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
    JumpIfNot {
        /// The address of the condition for the jump.
        cond: InstAddress,
        /// The offset to jump if the condition is true.
        jump: usize,
    },
    /// Construct a vector at `out`, populating it with `count` elements from
    /// `addr`.
    ///
    /// The values at `addr` are dropped.
    #[cfg_attr(feature = "musli", musli(packed))]
    Vec {
        /// Where the arguments to the vector are stored.
        addr: InstAddress,
        /// The number of elements in the vector.
        count: usize,
        /// Where to store the produced vector.
        out: Output,
    },
    /// Construct a one element tuple at `out`, populating it with `count`
    /// elements from `addr`.
    ///
    /// The values at `addr` are not dropped.
    #[cfg_attr(feature = "musli", musli(packed))]
    Tuple1 {
        /// Tuple arguments.
        #[inst_display(display_with = DisplayArray::new)]
        addr: [InstAddress; 1],
        /// Where to store the produced tuple.
        out: Output,
    },
    /// Construct a two element tuple at `out`, populating it with `count`
    /// elements from `addr`.
    ///
    /// The values at `addr` are not dropped.
    #[cfg_attr(feature = "musli", musli(packed))]
    Tuple2 {
        /// Tuple arguments.
        #[inst_display(display_with = DisplayArray::new)]
        addr: [InstAddress; 2],
        /// Where to store the produced tuple.
        out: Output,
    },
    /// Construct a three element tuple at `out`, populating it with `count`
    /// elements from `addr`.
    ///
    /// The values at `addr` are not dropped.
    #[cfg_attr(feature = "musli", musli(packed))]
    Tuple3 {
        /// Tuple arguments.
        #[inst_display(display_with = DisplayArray::new)]
        addr: [InstAddress; 3],
        /// Where to store the produced tuple.
        out: Output,
    },
    /// Construct a four element tuple at `out`, populating it with `count`
    /// elements from `addr`.
    ///
    /// The values at `addr` are not dropped.
    #[cfg_attr(feature = "musli", musli(packed))]
    Tuple4 {
        /// Tuple arguments.
        #[inst_display(display_with = DisplayArray::new)]
        addr: [InstAddress; 4],
        /// Where to store the produced tuple.
        out: Output,
    },
    /// Construct a tuple at `out`, populating it with `count` elements from
    /// `addr`.
    ///
    /// Unlike `TupleN` variants, values at `addr` are dropped.
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
    Object {
        /// Where the arguments to the tuple are stored.
        addr: InstAddress,
        /// The static slot of the object keys.
        slot: usize,
        /// Where to store the produced tuple.
        out: Output,
    },
    /// Construct a range.
    ///
    /// The arguments loaded are determined by the range being constructed.
    #[cfg_attr(feature = "musli", musli(packed))]
    Range {
        /// The kind of the range, which determines the number arguments on the
        /// stack.
        range: InstRange,
        /// Where to store the produced range.
        out: Output,
    },
    /// Construct a struct of type `hash` at `out`, populating it with fields
    /// from `addr`. The number of fields and their names is determined by the
    /// `slot` being referenced.
    ///
    /// The values at `addr` are dropped.
    #[cfg_attr(feature = "musli", musli(packed))]
    Struct {
        /// The address to load fields from.
        addr: InstAddress,
        /// The type of the struct to construct.
        hash: Hash,
        /// Where to write the constructed struct.
        out: Output,
    },
    /// Construct a struct from a constant.
    ///
    /// The values at `addr` are dropped.
    #[cfg_attr(feature = "musli", musli(packed))]
    ConstConstruct {
        /// Where constructor arguments are stored.
        addr: InstAddress,
        /// The type of the struct to construct.
        hash: Hash,
        /// The number of constructor arguments.
        count: usize,
        /// Where to write the constructed struct.
        out: Output,
    },
    /// Load a literal string from a static string slot.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <string>
    /// ```
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
    Try {
        /// Address of value to try.
        addr: InstAddress,
        /// Where to store the value in case there is a continuation.
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
    #[cfg_attr(feature = "musli", musli(packed))]
    EqChar {
        /// Address of the value to compare.
        addr: InstAddress,
        /// The character to test against.
        #[inst_display(display_with = DisplayDebug::new)]
        value: char,
        /// Where to store the result of the comparison.
        out: Output,
    },
    /// Test if the specified value is a specific signed integer.
    #[cfg_attr(feature = "musli", musli(packed))]
    EqSigned {
        /// Address of the value to compare.
        addr: InstAddress,
        /// The value to test against.
        value: i64,
        /// Where to store the result of the comparison.
        out: Output,
    },
    /// Test if the specified value is a specific unsigned integer.
    #[cfg_attr(feature = "musli", musli(packed))]
    EqUnsigned {
        /// Address of the value to compare.
        addr: InstAddress,
        /// The value to test against.
        value: u64,
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
    MatchVariant {
        /// The type hash of the containing enum.
        enum_hash: Hash,
        /// The type hash of the variant.
        variant_hash: Hash,
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
    Variant {
        /// Where the arguments to construct the variant are stored.
        addr: InstAddress,
        /// The kind of built-in variant to construct.
        variant: InstVariant,
        /// Where to store the variant.
        out: Output,
    },
    /// An operation.
    #[cfg_attr(feature = "musli", musli(packed))]
    Op {
        /// The kind of operation.
        op: InstOp,
        /// The address of the first argument.
        a: InstAddress,
        /// The address of the second argument.
        b: InstAddress,
        /// Whether the produced value from the operation should be kept or not.
        out: Output,
    },
    /// An arithmetic operation.
    #[cfg_attr(feature = "musli", musli(packed))]
    Arithmetic {
        /// The kind of operation.
        op: InstArithmeticOp,
        /// The address of the first argument.
        a: InstAddress,
        /// The address of the second argument.
        b: InstAddress,
        /// Whether the produced value from the operation should be kept or not.
        out: Output,
    },
    /// A bitwise operation.
    #[cfg_attr(feature = "musli", musli(packed))]
    Bitwise {
        /// The kind of operation.
        op: InstBitwiseOp,
        /// The address of the first argument.
        a: InstAddress,
        /// The address of the second argument.
        b: InstAddress,
        /// Whether the produced value from the operation should be kept or not.
        out: Output,
    },
    /// A shift operation.
    #[cfg_attr(feature = "musli", musli(packed))]
    Shift {
        /// The kind of operation.
        op: InstShiftOp,
        /// The address of the first argument.
        a: InstAddress,
        /// The address of the second argument.
        b: InstAddress,
        /// Whether the produced value from the operation should be kept or not.
        out: Output,
    },
    /// Instruction for assigned arithmetic operations.
    #[cfg_attr(feature = "musli", musli(packed))]
    AssignArithmetic {
        /// The kind of operation.
        op: InstArithmeticOp,
        /// The target of the operation.
        target: InstTarget,
        /// The value being assigned.
        rhs: InstAddress,
    },
    /// Instruction for assigned bitwise operations.
    #[cfg_attr(feature = "musli", musli(packed))]
    AssignBitwise {
        /// The kind of operation.
        op: InstBitwiseOp,
        /// The target of the operation.
        target: InstTarget,
        /// The value being assigned.
        rhs: InstAddress,
    },
    /// Instruction for assigned shift operations.
    #[cfg_attr(feature = "musli", musli(packed))]
    AssignShift {
        /// The kind of operation.
        op: InstShiftOp,
        /// The target of the operation.
        target: InstTarget,
        /// The value being assigned.
        rhs: InstAddress,
    },
    /// Advance an iterator at the given position.
    #[cfg_attr(feature = "musli", musli(packed))]
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
    #[cfg_attr(feature = "musli", musli(packed))]
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
            value: InstValue::Unit,
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

    /// Construct an instruction to push a character.
    pub fn char(c: char, out: Output) -> Self {
        Self::Store {
            value: InstValue::Char(c),
            out,
        }
    }

    /// Construct an instruction to push an integer.
    pub fn signed(v: i64, out: Output) -> Self {
        Self::Store {
            value: InstValue::Integer(v),
            out,
        }
    }

    /// Construct an instruction to push an unsigned integer.
    pub fn unsigned(v: u64, out: Output) -> Self {
        Self::Store {
            value: InstValue::Unsigned(v),
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

    /// Construct an instruction to push a type.
    pub fn ty(ty: Type, out: Output) -> Self {
        Self::Store {
            value: InstValue::Type(ty),
            out,
        }
    }

    /// Construct an instruction to push an ordering.
    pub fn ordering(ordering: Ordering, out: Output) -> Self {
        Self::Store {
            value: InstValue::Ordering(ordering),
            out,
        }
    }

    /// Construct an instruction to push a type hash.
    pub fn hash(hash: Hash, out: Output) -> Self {
        Self::Store {
            value: InstValue::Hash(hash),
            out,
        }
    }
}

/// What to do with the output of an instruction.
#[derive(TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(transparent))]
#[cfg_attr(feature = "musli", derive(Decode, Encode), musli(transparent))]
pub struct Output {
    offset: usize,
}

impl Output {
    /// Construct a keep output kind.
    #[inline]
    pub(crate) fn keep(offset: usize) -> Self {
        assert_ne!(offset, usize::MAX, "Address is invalid");
        Self { offset }
    }

    /// Construct a discard output kind.
    #[inline]
    pub(crate) fn discard() -> Self {
        Self { offset: usize::MAX }
    }

    /// Check if the output is a keep.
    #[inline(always)]
    pub(crate) fn as_addr(&self) -> Option<InstAddress> {
        if self.offset == usize::MAX {
            None
        } else {
            Some(InstAddress::new(self.offset))
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
    /// use rune::runtime::{Output, Memory, ToValue, VmError, InstAddress};
    /// use rune::vm_try;
    ///
    /// fn sum(stack: &mut dyn Memory, addr: InstAddress, args: usize, out: Output) -> Result<(), VmError> {
    ///     let mut number = 0;
    ///
    ///     for value in stack.slice_at(addr, args)? {
    ///         number += value.as_integer::<i64>()?;
    ///     }
    ///
    ///     out.store(stack, number)?;
    ///     Ok(())
    /// }
    #[inline(always)]
    pub fn store<M, O>(self, stack: &mut M, o: O) -> Result<(), RuntimeError>
    where
        M: ?Sized + Memory,
        O: IntoOutput,
    {
        if let Some(addr) = self.as_addr() {
            *stack.at_mut(addr)? = o.into_output()?;
        }

        Ok(())
    }
}

impl fmt::Display for Output {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.offset == usize::MAX {
            write!(f, "discard")
        } else {
            write!(f, "keep({})", self.offset)
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
    /// Coerce the current value into an output.
    fn into_output(self) -> Result<Value, RuntimeError>;
}

impl<F, O> IntoOutput for F
where
    F: FnOnce() -> O,
    O: IntoOutput,
{
    #[inline]
    fn into_output(self) -> Result<Value, RuntimeError> {
        self().into_output()
    }
}

impl<T, E> IntoOutput for Result<T, E>
where
    T: IntoOutput,
    RuntimeError: From<E>,
{
    #[inline]
    fn into_output(self) -> Result<Value, RuntimeError> {
        self?.into_output()
    }
}

impl IntoOutput for Value {
    #[inline]
    fn into_output(self) -> Result<Value, RuntimeError> {
        Ok(self)
    }
}

/// How an instruction addresses a value.
#[derive(Default, TryClone, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(transparent))]
#[cfg_attr(feature = "musli", derive(Decode, Encode), musli(transparent))]
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
#[derive(Debug, TryClone, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "musli", derive(Decode, Encode))]
#[try_clone(copy)]
#[non_exhaustive]
pub enum InstRange {
    /// `start..`.
    RangeFrom {
        /// The start address of the range.
        start: InstAddress,
    },
    /// `..`.
    RangeFull,
    /// `start..=end`.
    RangeInclusive {
        /// The start address of the range.
        start: InstAddress,
        /// The end address of the range.
        end: InstAddress,
    },
    /// `..=end`.
    RangeToInclusive {
        /// The end address of the range.
        end: InstAddress,
    },
    /// `..end`.
    RangeTo {
        /// The end address of the range.
        end: InstAddress,
    },
    /// `start..end`.
    Range {
        /// The start address of the range.
        start: InstAddress,
        /// The end address of the range.
        end: InstAddress,
    },
}

impl fmt::Display for InstRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstRange::RangeFrom { start } => write!(f, "{start}.."),
            InstRange::RangeFull => write!(f, ".."),
            InstRange::RangeInclusive { start, end } => write!(f, "{start}..={end}"),
            InstRange::RangeToInclusive { end } => write!(f, "..={end}"),
            InstRange::RangeTo { end } => write!(f, "..{end}"),
            InstRange::Range { start, end } => write!(f, "{start}..{end}"),
        }
    }
}

/// The target of an operation.
#[derive(Debug, TryClone, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "musli", derive(Decode, Encode))]
#[try_clone(copy)]
pub enum InstTarget {
    /// Target is an offset to the current call frame.
    #[cfg_attr(feature = "musli", musli(packed))]
    Address(InstAddress),
    /// Target the field of an object.
    #[cfg_attr(feature = "musli", musli(packed))]
    Field(InstAddress, usize),
    /// Target a tuple field.
    #[cfg_attr(feature = "musli", musli(packed))]
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
#[derive(Debug, TryClone, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "musli", derive(Decode, Encode))]
#[try_clone(copy)]
pub enum InstArithmeticOp {
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
}

impl fmt::Display for InstArithmeticOp {
    #[inline]
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
        }

        Ok(())
    }
}

/// An operation between two values on the machine.
#[derive(Debug, TryClone, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "musli", derive(Decode, Encode))]
#[try_clone(copy)]
pub enum InstBitwiseOp {
    /// The bitwise and operation. `a & b`.
    BitAnd,
    /// The bitwise xor operation. `a ^ b`.
    BitXor,
    /// The bitwise or operation. `a | b`.
    BitOr,
}

impl fmt::Display for InstBitwiseOp {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BitAnd => {
                write!(f, "&")?;
            }
            Self::BitXor => {
                write!(f, "^")?;
            }
            Self::BitOr => {
                write!(f, "|")?;
            }
        }

        Ok(())
    }
}

/// An operation between two values on the machine.
#[derive(Debug, TryClone, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "musli", derive(Decode, Encode))]
#[try_clone(copy)]
pub enum InstShiftOp {
    /// The shift left operation. `a << b`.
    Shl,
    /// The shift right operation. `a << b`.
    Shr,
}

impl fmt::Display for InstShiftOp {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
#[derive(Debug, TryClone, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "musli", derive(Decode, Encode))]
#[try_clone(copy)]
pub enum InstOp {
    /// Compare two values on the stack for lt and push the result as a
    /// boolean on the stack.
    Lt,
    /// Compare two values on the stack for lte and push the result as a
    /// boolean on the stack.
    Le,
    /// Compare two values on the stack for gt and push the result as a
    /// boolean on the stack.
    Gt,
    /// Compare two values on the stack for gte and push the result as a
    /// boolean on the stack.
    Ge,
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
            Self::Lt => {
                write!(f, "<")?;
            }
            Self::Gt => {
                write!(f, ">")?;
            }
            Self::Le => {
                write!(f, "<=")?;
            }
            Self::Ge => {
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
#[derive(Debug, TryClone, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "musli", derive(Decode, Encode))]
#[try_clone(copy)]
#[non_exhaustive]
pub enum InstValue {
    /// An empty tuple.
    Unit,
    /// A boolean.
    #[cfg_attr(feature = "musli", musli(packed))]
    Bool(bool),
    /// A character.
    #[cfg_attr(feature = "musli", musli(packed))]
    Char(char),
    /// An unsigned integer.
    #[cfg_attr(feature = "musli", musli(packed))]
    Unsigned(u64),
    /// An integer.
    #[cfg_attr(feature = "musli", musli(packed))]
    Integer(i64),
    /// A float.
    #[cfg_attr(feature = "musli", musli(packed))]
    Float(f64),
    /// A type hash.
    #[cfg_attr(feature = "musli", musli(packed))]
    Type(Type),
    /// An ordering.
    Ordering(
        #[cfg_attr(feature = "musli", musli(with = crate::musli::ordering))]
        #[cfg_attr(feature = "serde", serde(with = "crate::serde::ordering"))]
        Ordering,
    ),
    /// A hash.
    #[cfg_attr(feature = "musli", musli(packed))]
    Hash(Hash),
}

impl InstValue {
    /// Convert into a value that can be pushed onto the stack.
    pub fn into_value(self) -> Value {
        match self {
            Self::Unit => Value::unit(),
            Self::Bool(v) => Value::from(v),
            Self::Char(v) => Value::from(v),
            Self::Unsigned(v) => Value::from(v),
            Self::Integer(v) => Value::from(v),
            Self::Float(v) => Value::from(v),
            Self::Type(v) => Value::from(v),
            Self::Ordering(v) => Value::from(v),
            Self::Hash(v) => Value::from(v),
        }
    }
}

impl fmt::Display for InstValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unit => write!(f, "()")?,
            Self::Bool(v) => write!(f, "{}", v)?,
            Self::Char(v) => write!(f, "{v:?}")?,
            Self::Unsigned(v) => write!(f, "{v}u64")?,
            Self::Integer(v) => write!(f, "{v}i64")?,
            Self::Float(v) => write!(f, "{v}")?,
            Self::Type(v) => write!(f, "{}", v.into_hash())?,
            Self::Ordering(v) => write!(f, "{v:?}")?,
            Self::Hash(v) => write!(f, "{v:?}")?,
        }

        Ok(())
    }
}

/// A variant that can be constructed.
#[derive(Debug, TryClone, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "musli", derive(Decode, Encode))]
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

impl IntoOutput for &str {
    #[inline]
    fn into_output(self) -> Result<Value, RuntimeError> {
        Ok(Value::try_from(self)?)
    }
}
