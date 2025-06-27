use core::cmp::Ordering;
use core::fmt;

#[cfg(feature = "musli")]
use musli::{Decode, Encode};
use rune_macros::InstDisplay;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate as rune;
use crate::alloc;
use crate::alloc::prelude::*;
use crate::Hash;

use super::{Call, FormatSpec, Type, Value};

/// An instruction in the virtual machine.
#[derive(Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(transparent))]
#[cfg_attr(feature = "musli", derive(Decode, Encode), musli(transparent))]
pub struct Inst {
    pub(crate) kind: Kind,
}

impl Inst {
    #[inline]
    pub(crate) fn new(kind: Kind) -> Self {
        Self { kind }
    }
}

impl fmt::Display for Inst {
    #[inline]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(fmt)
    }
}

impl fmt::Debug for Inst {
    #[inline]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(fmt)
    }
}

impl TryClone for Inst {
    #[inline]
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(Self {
            kind: self.kind.try_clone()?,
        })
    }
}

/// Pre-canned panic reasons.
///
/// To formulate a custom reason, use
/// [`VmError::panic`][crate::runtime::VmError::panic].
#[derive(Debug, TryClone, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "musli", derive(Decode, Encode))]
#[try_clone(copy)]
pub(crate) enum PanicReason {
    /// A pattern didn't match where it unconditionally has to.
    UnmatchedPattern,
}

impl PanicReason {
    /// The identifier of the panic.
    fn ident(&self) -> &'static str {
        match *self {
            Self::UnmatchedPattern => "unmatched pattern",
        }
    }
}

impl fmt::Display for PanicReason {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::UnmatchedPattern => write!(fmt, "pattern did not match")?,
        }

        Ok(())
    }
}

/// The kind of an instruction in the virtual machine.
#[derive(Debug, TryClone, Clone, Copy, InstDisplay)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "musli", derive(Decode, Encode))]
#[try_clone(copy)]
pub(crate) enum Kind {
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
        addr: Address,
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
        addr: Address,
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
        addr: Address,
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
        addr: Address,
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
        addr: Address,
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
        addr: Address,
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
        addr: Address,
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
        function: Address,
        /// The address of the arguments being passed.
        addr: Address,
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
        target: Address,
        /// How the index is addressed.
        index: Address,
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
        target: Address,
        /// The index to set.
        index: usize,
        /// The value being assigned.
        value: Address,
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
        addr: Address,
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
        target: Address,
        /// The static string slot corresponding to the index to set.
        slot: usize,
        /// The value being assigned.
        value: Address,
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
        addr: Address,
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
        target: Address,
        /// The index to set.
        index: Address,
        /// The value being assigned.
        value: Address,
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
        addr: Address,
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
        addr: Address,
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
        addr: Address,
        /// Where the value is being copied to.
        out: Output,
    },
    /// Move a variable from a location `offset` relative to the current call
    /// frame.
    #[cfg_attr(feature = "musli", musli(packed))]
    Move {
        /// Address of the value being moved.
        addr: Address,
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
        a: Address,
        /// Offset to the second value.
        b: Address,
    },
    /// Pop the current stack frame and restore the instruction pointer from it.
    ///
    /// The stack frame will be cleared, and the value on the top of the stack
    /// will be left on top of it.
    #[cfg_attr(feature = "musli", musli(packed))]
    Return {
        /// The address of the value to return.
        addr: Address,
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
        cond: Address,
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
        cond: Address,
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
        addr: Address,
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
        addr: [Address; 1],
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
        addr: [Address; 2],
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
        addr: [Address; 3],
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
        addr: [Address; 4],
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
        addr: Address,
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
        addr: Address,
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
        addr: Address,
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
        addr: Address,
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
        addr: Address,
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
        addr: Address,
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
        addr: Address,
        /// The format specification to use.
        spec: FormatSpec,
        /// Where to store the produced format.
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
        addr: Address,
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
        addr: Address,
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
        addr: Address,
        /// The value to test against.
        value: i64,
        /// Where to store the result of the comparison.
        out: Output,
    },
    /// Test if the specified value is a specific unsigned integer.
    #[cfg_attr(feature = "musli", musli(packed))]
    EqUnsigned {
        /// Address of the value to compare.
        addr: Address,
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
        addr: Address,
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
        addr: Address,
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
        addr: Address,
        /// The slot to test against.
        slot: usize,
        /// Where to store the result of the comparison.
        out: Output,
    },
    /// Test if the specified type matches.
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
        /// The variant hash to match against.
        variant_hash: Hash,
        /// The address of the value to test.
        addr: Address,
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
        hash: Hash,
        /// The minimum length to test for.
        len: usize,
        /// Whether the operation should check exact `true` or minimum length
        /// `false`.
        exact: bool,
        /// The address of the value to test.
        addr: Address,
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
        addr: Address,
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
        addr: Address,
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
    /// An operation.
    #[cfg_attr(feature = "musli", musli(packed))]
    Op {
        /// The kind of operation.
        op: InstOp,
        /// The address of the first argument.
        a: Address,
        /// The address of the second argument.
        b: Address,
        /// Whether the produced value from the operation should be kept or not.
        out: Output,
    },
    /// An arithmetic operation.
    #[cfg_attr(feature = "musli", musli(packed))]
    Arithmetic {
        /// The kind of operation.
        op: InstArithmeticOp,
        /// The address of the first argument.
        a: Address,
        /// The address of the second argument.
        b: Address,
        /// Whether the produced value from the operation should be kept or not.
        out: Output,
    },
    /// A bitwise operation.
    #[cfg_attr(feature = "musli", musli(packed))]
    Bitwise {
        /// The kind of operation.
        op: InstBitwiseOp,
        /// The address of the first argument.
        a: Address,
        /// The address of the second argument.
        b: Address,
        /// Whether the produced value from the operation should be kept or not.
        out: Output,
    },
    /// A shift operation.
    #[cfg_attr(feature = "musli", musli(packed))]
    Shift {
        /// The kind of operation.
        op: InstShiftOp,
        /// The address of the first argument.
        a: Address,
        /// The address of the second argument.
        b: Address,
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
        rhs: Address,
    },
    /// Instruction for assigned bitwise operations.
    #[cfg_attr(feature = "musli", musli(packed))]
    AssignBitwise {
        /// The kind of operation.
        op: InstBitwiseOp,
        /// The target of the operation.
        target: InstTarget,
        /// The value being assigned.
        rhs: Address,
    },
    /// Instruction for assigned shift operations.
    #[cfg_attr(feature = "musli", musli(packed))]
    AssignShift {
        /// The kind of operation.
        op: InstShiftOp,
        /// The target of the operation.
        target: InstTarget,
        /// The value being assigned.
        rhs: Address,
    },
    /// Advance an iterator at the given position.
    #[cfg_attr(feature = "musli", musli(packed))]
    IterNext {
        /// The address of the iterator to advance.
        addr: Address,
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

impl Kind {
    /// Construct an instruction to push a unit.
    pub(crate) fn unit(out: Output) -> Self {
        Self::Store {
            value: InstValue::Unit,
            out,
        }
    }

    /// Construct an instruction to push a boolean.
    pub(crate) fn bool(b: bool, out: Output) -> Self {
        Self::Store {
            value: InstValue::Bool(b),
            out,
        }
    }

    /// Construct an instruction to push a character.
    pub(crate) fn char(c: char, out: Output) -> Self {
        Self::Store {
            value: InstValue::Char(c),
            out,
        }
    }

    /// Construct an instruction to push an integer.
    pub(crate) fn signed(v: i64, out: Output) -> Self {
        Self::Store {
            value: InstValue::Integer(v),
            out,
        }
    }

    /// Construct an instruction to push an unsigned integer.
    pub(crate) fn unsigned(v: u64, out: Output) -> Self {
        Self::Store {
            value: InstValue::Unsigned(v),
            out,
        }
    }

    /// Construct an instruction to push a float.
    pub(crate) fn float(v: f64, out: Output) -> Self {
        Self::Store {
            value: InstValue::Float(v),
            out,
        }
    }

    /// Construct an instruction to push a type.
    pub(crate) fn ty(ty: Type, out: Output) -> Self {
        Self::Store {
            value: InstValue::Type(ty),
            out,
        }
    }

    /// Construct an instruction to push an ordering.
    pub(crate) fn ordering(ordering: Ordering, out: Output) -> Self {
        Self::Store {
            value: InstValue::Ordering(ordering),
            out,
        }
    }

    /// Construct an instruction to push a type hash.
    pub(crate) fn hash(hash: Hash, out: Output) -> Self {
        Self::Store {
            value: InstValue::Hash(hash),
            out,
        }
    }
}

/// What to do with the output of an instruction.
#[derive(TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
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
    pub(crate) fn as_addr(&self) -> Option<Address> {
        if self.offset == usize::MAX {
            None
        } else {
            Some(Address::new(self.offset))
        }
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

/// How an instruction addresses a value.
#[derive(Default, TryClone, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(transparent))]
#[cfg_attr(feature = "musli", derive(Decode, Encode), musli(transparent))]
#[try_clone(copy)]
pub struct Address {
    offset: usize,
}

impl Address {
    /// The first possible address.
    pub const ZERO: Address = Address { offset: 0 };

    /// An invalid address.
    pub const INVALID: Address = Address { offset: usize::MAX };

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

impl fmt::Display for Address {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.offset == usize::MAX {
            write!(f, "invalid")
        } else {
            self.offset.fmt(f)
        }
    }
}

impl fmt::Debug for Address {
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
pub(crate) enum InstRange {
    /// `start..`.
    RangeFrom {
        /// The start address of the range.
        start: Address,
    },
    /// `..`.
    RangeFull,
    /// `start..=end`.
    RangeInclusive {
        /// The start address of the range.
        start: Address,
        /// The end address of the range.
        end: Address,
    },
    /// `..=end`.
    RangeToInclusive {
        /// The end address of the range.
        end: Address,
    },
    /// `..end`.
    RangeTo {
        /// The end address of the range.
        end: Address,
    },
    /// `start..end`.
    Range {
        /// The start address of the range.
        start: Address,
        /// The end address of the range.
        end: Address,
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
pub(crate) enum InstTarget {
    /// Target is an offset to the current call frame.
    #[cfg_attr(feature = "musli", musli(packed))]
    Address(Address),
    /// Target the field of an object.
    #[cfg_attr(feature = "musli", musli(packed))]
    Field(Address, usize),
    /// Target a tuple field.
    #[cfg_attr(feature = "musli", musli(packed))]
    TupleField(Address, usize),
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
pub(crate) enum InstArithmeticOp {
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
pub(crate) enum InstBitwiseOp {
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
pub(crate) enum InstShiftOp {
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
pub(crate) enum InstOp {
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
    #[inline]
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
pub(crate) enum InstValue {
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
    pub(crate) fn into_value(self) -> Value {
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
            Self::Bool(v) => write!(f, "{v}")?,
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
