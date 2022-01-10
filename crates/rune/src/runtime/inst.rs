use crate::runtime::{FormatSpec, Value};
use crate::Hash;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Pre-canned panic reasons.
///
/// To formulate a custom reason, use [crate::runtime::Panic::custom].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
    /// An option type, and the specified variant index.
    Option(usize),
    /// A result type, and the specified variant index.
    Result(usize),
    /// A generator state type, and the specified variant index.
    GeneratorState(usize),
}

impl fmt::Display for TypeCheck {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unit => write!(fmt, "Unit"),
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Inst {
    /// Not operator. Takes a boolean from the top of the stack  and inverts its
    /// logical value.
    ///
    /// # Operation
    ///
    /// ```text
    /// <bool>
    /// => <bool>
    /// ```
    Not,
    /// Negate the numerical value on the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// <number>
    /// => <number>
    /// ```
    Neg,
    /// Construct a closure that takes the given number of arguments and
    /// captures `count` elements from the top of the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value..>
    /// => <fn>
    /// ```
    Closure {
        /// The hash of the internally stored closure function.
        hash: Hash,
        /// The number of arguments to store in the environment on the stack.
        count: usize,
    },
    /// Perform a function call.
    ///
    /// It will construct a new stack frame which includes the last `args`
    /// number of entries.
    Call {
        /// The hash of the function to call.
        hash: Hash,
        /// The number of arguments expected on the stack for this call.
        args: usize,
    },
    /// Perform a instance function call.
    ///
    /// The instance being called on should be on top of the stack, followed by
    /// `args` number of arguments.
    CallInstance {
        /// The hash of the name of the function to call.
        hash: Hash,
        /// The number of arguments expected on the stack for this call.
        args: usize,
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
    LoadInstanceFn {
        /// The name hash of the instance function.
        hash: Hash,
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
    CallFn {
        /// The number of arguments expected on the stack for this call.
        args: usize,
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
    IndexGet {
        /// How the target is addressed.
        target: InstAddress,
        /// How the index is addressed.
        index: InstAddress,
    },
    /// Get the given index out of a tuple on the top of the stack.
    /// Errors if the item doesn't exist or the item is not a tuple.
    ///
    /// # Operation
    ///
    /// ```text
    /// <tuple>
    /// => <value>
    /// ```
    TupleIndexGet {
        /// The index to fetch.
        index: usize,
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
    TupleIndexGetAt {
        /// The slot offset to load the tuple from.
        offset: usize,
        /// The index to fetch.
        index: usize,
    },
    /// Get the given index out of an object on the top of the stack.
    /// Errors if the item doesn't exist or the item is not an object.
    ///
    /// The index is identifier by a static string slot, which is provided as an
    /// argument.
    ///
    /// # Operation
    ///
    /// ```text
    /// <object>
    /// => <value>
    /// ```
    ObjectIndexGet {
        /// The static string slot corresponding to the index to fetch.
        slot: usize,
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
    ObjectIndexGetAt {
        /// The slot offset to get the value to load from.
        offset: usize,
        /// The static string slot corresponding to the index to fetch.
        slot: usize,
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
    Await,
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
    Select {
        /// The number of futures to poll.
        len: usize,
    },
    /// Load the given function by hash and push onto the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <value>
    /// ```
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
    Push {
        /// The value to push.
        value: InstValue,
    },
    /// Pop the value on the stack, discarding its result.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// =>
    /// ```
    Pop,
    /// Pop the given number of elements from the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value..>
    /// => *noop*
    /// ```
    PopN {
        /// The number of elements to pop from the stack.
        count: usize,
    },
    /// If the stop of the stack is false, will pop the given `count` entries on
    /// the stack and jump to the given offset.
    ///
    /// # Operation
    ///
    /// ```text
    /// <bool>
    /// => *noop*
    /// ```
    PopAndJumpIfNot {
        /// The number of entries to pop of the condition is true.
        count: usize,
        /// The offset to jump if the condition is true.
        offset: isize,
    },
    /// Clean the stack by keeping the top of it, and popping `count` values
    /// under it.
    ///
    /// # Operation
    ///
    /// ```text
    /// <top>
    /// <value..>
    /// => <top>
    /// ```
    Clean {
        /// The number of entries in the stack to pop.
        count: usize,
    },
    /// Copy a variable from a location `offset` relative to the current call
    /// frame.
    ///
    /// A copy is very cheap. It simply means pushing a reference to the stack.
    Copy {
        /// Offset to copy value from.
        offset: usize,
    },
    /// Move a variable from a location `offset` relative to the current call
    /// frame.
    Move {
        /// Offset to move value from.
        offset: usize,
    },
    /// Drop the value in the given frame offset, cleaning out it's slot in
    /// memory.
    ///
    /// # Operation
    ///
    /// ```text
    /// => *noop*
    /// ```
    Drop {
        /// Frame offset to drop.
        offset: usize,
    },
    /// Duplicate the value at the top of the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <value>
    /// ```
    Dup,
    /// Replace a value at the offset relative from the top of the stack, with
    /// the top of the stack.
    Replace {
        /// Offset to swap value from.
        offset: usize,
    },
    /// Pop the current stack frame and restore the instruction pointer from it.
    ///
    /// The stack frame will be cleared, and the value on the top of the stack
    /// will be left on top of it.
    Return {
        /// The address of the value to return.
        address: InstAddress,
        /// Number of variables to clean. If address is top, this should only
        /// specify variables in excess of the top variable. Otherwise, this
        /// includes the return value.
        clean: usize,
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
    Jump {
        /// Offset to jump to.
        offset: isize,
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
    JumpIf {
        /// Offset to jump to.
        offset: isize,
    },
    /// Jump to `offset` relative to the current instruction pointer if the
    /// condition is `true`. Will only pop the stack is a jump is not performed.
    ///
    /// # Operation
    ///
    /// ```text
    /// <boolean>
    /// => *nothing*
    /// ```
    JumpIfOrPop {
        /// Offset to jump to.
        offset: isize,
    },
    /// Jump to `offset` relative to the current instruction pointer if the
    /// condition is `false`. Will only pop the stack is a jump is not performed.
    ///
    /// # Operation
    ///
    /// ```text
    /// <boolean>
    /// => *nothing*
    /// ```
    JumpIfNotOrPop {
        /// Offset to jump to.
        offset: isize,
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
    JumpIfBranch {
        /// The branch value to compare against.
        branch: i64,
        /// The offset to jump.
        offset: isize,
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
    Tuple1 {
        /// First element of the tuple.
        args: [InstAddress; 1],
    },
    /// Construct a push a two-tuple value onto the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <tuple>
    /// ```
    Tuple2 {
        /// Tuple arguments.
        args: [InstAddress; 2],
    },
    /// Construct a push a three-tuple value onto the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <tuple>
    /// ```
    Tuple3 {
        /// Tuple arguments.
        args: [InstAddress; 3],
    },
    /// Construct a push a four-tuple value onto the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <tuple>
    /// ```
    Tuple4 {
        /// Tuple arguments.
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
    Tuple {
        /// The size of the tuple.
        count: usize,
    },
    /// Take the tuple that is on top of the stack and push its content onto the
    /// stack.
    ///
    /// Note: this is used by closures to "unpack" their environment into local
    /// variables.
    ///
    /// # Operation
    ///
    /// ```text
    /// <tuple>
    /// => <value...>
    /// ```
    PushTuple,
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
    /// <from>
    /// <to>
    /// => <range>
    /// ```
    Range {
        /// The limits of the range.
        limits: InstRangeLimits,
    },
    /// Construct a push an object of the given type onto the stack. The type is
    /// an empty struct.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <object>
    /// ```
    UnitStruct {
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
    StringConcat {
        /// The number of items to pop from the stack.
        len: usize,
        /// The minimum string size used.
        size_hint: usize,
    },
    /// Push a combined format specification and value onto the stack. The value
    /// used is the last value on the stack.
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
    Try {
        /// Address to test if value.
        address: InstAddress,
        /// Variable count that needs to be cleaned in case the operation
        /// results in a return.
        clean: usize,
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
    EqCharacter {
        /// The character to test against.
        character: char,
    },
    /// Test if the top of the stack is a specific integer.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// => <boolean>
    /// ```
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
    EqStaticString {
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
    Yield,
    /// Perform a generator yield with a unit.
    ///
    /// This causes the virtual machine to suspend itself.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <unit>
    /// ```
    YieldUnit,
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
    Op {
        /// The actual operation.
        op: InstOp,
        /// The address of the first argument.
        a: InstAddress,
        /// The address of the second argument.
        b: InstAddress,
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
    Assign {
        /// The target of the operation.
        target: InstTarget,
        /// The actual operation.
        op: InstAssignOp,
    },
    /// Advance an iterator at the given position.
    IterNext {
        /// The offset of the value being advanced.
        offset: usize,
        /// A relative jump to perform if the iterator could not be advanced.
        jump: isize,
    },
    /// Cause the VM to panic and error out without a reason.
    ///
    /// This should only be used during testing or extreme scenarios that are
    /// completely unrecoverable.
    Panic {
        /// The reason for the panic.
        reason: PanicReason,
    },
}

impl Inst {
    /// Construct an instruction to push a unit.
    pub fn unit() -> Self {
        Self::Push {
            value: InstValue::Unit,
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

impl fmt::Display for Inst {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Drop { offset } => {
                write!(fmt, "drop offset={}", offset)?;
            }
            Self::Not => {
                write!(fmt, "not")?;
            }
            Self::Neg => {
                write!(fmt, "neg")?;
            }
            Self::Call { hash, args } => {
                write!(fmt, "call hash={}, args={}", hash, args)?;
            }
            Self::CallInstance { hash, args } => {
                write!(fmt, "call-instance hash={}, args={}", hash, args)?;
            }
            Self::Closure { hash, count } => {
                write!(fmt, "closure hash={}, count={}", hash, count)?;
            }
            Self::CallFn { args } => {
                write!(fmt, "call-fn args={}", args)?;
            }
            Self::LoadInstanceFn { hash } => {
                write!(fmt, "load-instance-fn hash={}", hash)?;
            }
            Self::IndexGet { target, index } => {
                write!(fmt, "index-get target={}, index={}", target, index)?;
            }
            Self::TupleIndexGet { index } => {
                write!(fmt, "tuple-index-get index={}", index)?;
            }
            Self::TupleIndexSet { index } => {
                write!(fmt, "tuple-index-set index={}", index)?;
            }
            Self::TupleIndexGetAt { offset, index } => {
                write!(fmt, "tuple-index-get-at offset={}, index={}", offset, index)?;
            }
            Self::ObjectIndexGet { slot } => {
                write!(fmt, "object-index-get slot={}", slot)?;
            }
            Self::ObjectIndexSet { slot } => {
                write!(fmt, "object-index-set slot={}", slot)?;
            }
            Self::ObjectIndexGetAt { offset, slot } => {
                write!(fmt, "object-index-get-at offset={}, slot={}", offset, slot)?;
            }
            Self::IndexSet => {
                write!(fmt, "index-set")?;
            }
            Self::Await => {
                write!(fmt, "await")?;
            }
            Self::Select { len } => {
                write!(fmt, "select len={}", len)?;
            }
            Self::LoadFn { hash } => {
                write!(fmt, "load-fn hash={}", hash)?;
            }
            Self::Push { value } => {
                write!(fmt, "push value={}", value)?;
            }
            Self::Pop => {
                write!(fmt, "pop")?;
            }
            Self::PopN { count } => {
                write!(fmt, "pop-n count={}", count)?;
            }
            Self::PopAndJumpIfNot { count, offset } => {
                write!(
                    fmt,
                    "pop-and-jump-if-not count={}, offset={}",
                    count, offset
                )?;
            }
            Self::Clean { count } => {
                write!(fmt, "clean count={}", count)?;
            }
            Self::Copy { offset } => {
                write!(fmt, "copy offset={}", offset)?;
            }
            Self::Move { offset } => {
                write!(fmt, "move offset={}", offset)?;
            }
            Self::Dup => {
                write!(fmt, "dup")?;
            }
            Self::Replace { offset } => {
                write!(fmt, "replace offset={}", offset)?;
            }
            Self::Return { address, clean } => {
                write!(fmt, "return address={}, clean={}", address, clean)?;
            }
            Self::ReturnUnit => {
                write!(fmt, "return-unit")?;
            }
            Self::Jump { offset } => {
                write!(fmt, "jump offset={}", offset)?;
            }
            Self::JumpIf { offset } => {
                write!(fmt, "jump-if offset={}", offset)?;
            }
            Self::JumpIfOrPop { offset } => {
                write!(fmt, "jump-if-or-pop offset={}", offset)?;
            }
            Self::JumpIfNotOrPop { offset } => {
                write!(fmt, "jump-if-not-or-pop offset={}", offset)?;
            }
            Self::JumpIfBranch { branch, offset } => {
                write!(fmt, "jump-if-branch branch={}, offset={}", branch, offset)?;
            }
            Self::Vec { count } => {
                write!(fmt, "vec count={}", count)?;
            }
            Self::Tuple1 { args: [a] } => {
                write!(fmt, "tuple-1 {}", a)?;
            }
            Self::Tuple2 { args: [a, b] } => {
                write!(fmt, "tuple-2 {}, {}", a, b)?;
            }
            Self::Tuple3 { args: [a, b, c] } => {
                write!(fmt, "tuple-3 {}, {}, {}", a, b, c)?;
            }
            Self::Tuple4 { args: [a, b, c, d] } => {
                write!(fmt, "tuple-4 {}, {}, {}, {}", a, b, c, d)?;
            }
            Self::Tuple { count } => {
                write!(fmt, "tuple count={}", count)?;
            }
            Self::PushTuple => {
                write!(fmt, "push-tuple")?;
            }
            Self::UnitStruct { hash } => {
                write!(fmt, "unit-struct hash={}", hash)?;
            }
            Self::Struct { hash, slot } => {
                write!(fmt, "struct hash={}, slot={}", hash, slot)?;
            }
            Self::UnitVariant { hash } => {
                write!(fmt, "unit-variant hash={}", hash)?;
            }
            Self::StructVariant { hash, slot } => {
                write!(fmt, "struct-variant hash={}, slot={}", hash, slot)?;
            }
            Self::Object { slot } => {
                write!(fmt, "object slot={}", slot)?;
            }
            Self::Range { limits } => {
                write!(fmt, "range limits={}", limits)?;
            }
            Self::String { slot } => {
                write!(fmt, "string slot={}", slot)?;
            }
            Self::Bytes { slot } => {
                write!(fmt, "bytes slot={}", slot)?;
            }
            Self::StringConcat { len, size_hint } => {
                write!(fmt, "string-concat len={}, size_hint={}", len, size_hint)?;
            }
            Self::Format { spec } => {
                write!(
                    fmt,
                    "format {fill:?}, {align}, {flags:?}, {width}, {precision}, {format_type}",
                    fill = spec.fill,
                    align = spec.align,
                    flags = spec.flags,
                    width = option(&spec.width),
                    precision = option(&spec.precision),
                    format_type = spec.format_type
                )?;
            }
            Self::IsUnit => {
                write!(fmt, "is-unit")?;
            }
            Self::Try {
                address,
                clean,
                preserve,
            } => {
                write!(
                    fmt,
                    "try address={}, clean={}, preserve={}",
                    address, clean, preserve
                )?;
            }
            Self::EqByte { byte } => {
                write!(fmt, "eq-byte byte={:?}", byte)?;
            }
            Self::EqCharacter { character } => {
                write!(fmt, "eq-character character={:?}", character)?;
            }
            Self::EqInteger { integer } => {
                write!(fmt, "eq-integer integer={}", integer)?;
            }
            Self::EqBool { boolean } => {
                write!(fmt, "eq-integer boolean={}", boolean)?;
            }
            Self::EqStaticString { slot } => {
                write!(fmt, "eq-static-string slot={}", slot)?;
            }
            Self::MatchType { hash } => {
                write!(fmt, "match-type hash={}", hash,)?;
            }
            Self::MatchVariant {
                variant_hash,
                enum_hash,
                index,
            } => {
                write!(
                    fmt,
                    "match-variant variant_hash={}, enum_hash={}, index={}",
                    enum_hash, variant_hash, index
                )?;
            }
            Self::MatchBuiltIn { type_check } => {
                write!(fmt, "match-builtin type_check={}", type_check)?;
            }
            Self::MatchSequence {
                type_check,
                len,
                exact,
            } => {
                write!(
                    fmt,
                    "match-sequence type_check={}, len={}, exact={}",
                    type_check, len, exact
                )?;
            }
            Self::MatchObject { slot, exact } => {
                write!(fmt, "match-object slot={}, exact={}", slot, exact)?;
            }
            Self::Yield => {
                write!(fmt, "yield")?;
            }
            Self::YieldUnit => {
                write!(fmt, "yield-unit")?;
            }
            Self::Variant { variant } => {
                write!(fmt, "variant variant={}", variant)?;
            }
            Self::Op { op, a, b } => {
                write!(fmt, "op op={}, a={}, b={}", op, a, b)?;
            }
            Self::Assign { target, op } => {
                write!(fmt, "assign target={}, op={}", target, op)?;
            }
            Self::IterNext { offset, jump } => {
                write!(fmt, "iter-next offset={}, jump={}", offset, jump)?;
            }
            Self::Panic { reason } => {
                write!(fmt, "panic reason={}", reason.ident())?;
            }
        }

        return Ok(());

        fn option<T>(value: &Option<T>) -> OptionDebug<'_, T> {
            OptionDebug(value.as_ref())
        }

        struct OptionDebug<'a, T>(Option<&'a T>);

        impl<'a, T> fmt::Display for OptionDebug<'a, T>
        where
            T: fmt::Display,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self.0 {
                    Some(value) => write!(f, "{}", value),
                    None => write!(f, "?"),
                }
            }
        }
    }
}

/// How an instruction addresses a value.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum InstAddress {
    /// Addressed from the top of the stack.
    Top,
    /// Value addressed at the given offset.
    Offset(usize),
}

impl fmt::Display for InstAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Top => write!(f, "top"),
            Self::Offset(offset) => write!(f, "offset({})", offset),
        }
    }
}

/// Range limits of a range expression.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum InstRangeLimits {
    /// A half-open range `a .. b`.
    HalfOpen,
    /// A closed range `a ..= b`.
    Closed,
}

impl fmt::Display for InstRangeLimits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HalfOpen => write!(f, ".."),
            Self::Closed => write!(f, "..="),
        }
    }
}

/// The target of an operation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum InstTarget {
    /// Target is an offset to the current call frame.
    Offset(usize),
    /// Target the field of an object.
    Field(usize),
    /// Target a tuple field.
    TupleField(usize),
}

impl fmt::Display for InstTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Offset(offset) => write!(f, "offset({})", offset),
            Self::Field(slot) => write!(f, "field({})", slot),
            Self::TupleField(slot) => write!(f, "tuple-field({})", slot),
        }
    }
}

/// An operation between two values on the machine.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum InstValue {
    /// A unit.
    Unit,
    /// A boolean.
    Bool(bool),
    /// A byte.
    Byte(u8),
    /// A character.
    Char(char),
    /// An integer.
    Integer(i64),
    /// A float.
    Float(f64),
    /// A type hash.
    Type(Hash),
}

impl InstValue {
    /// Convert into a value that can be pushed onto the stack.
    pub fn into_value(self) -> Value {
        match self {
            Self::Unit => Value::Unit,
            Self::Bool(v) => Value::Bool(v),
            Self::Byte(v) => Value::Byte(v),
            Self::Char(v) => Value::Char(v),
            Self::Integer(v) => Value::Integer(v),
            Self::Float(v) => Value::Float(v),
            Self::Type(v) => Value::Type(v),
        }
    }
}

impl fmt::Display for InstValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unit => write!(f, "()")?,
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
            Self::Type(v) => write!(f, "{}", v)?,
        }

        Ok(())
    }
}

/// A variant that can be constructed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
