use crate::Hash;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Pre-canned panic reasons.
///
/// To formulate a custom reason, use [crate::Panic::custom].
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

/// An encoded type check.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
    /// Matches the type with the corresponding hash.
    Type(Hash),
    /// Matches the variant with the corresponding hash.
    Variant(Hash),
}

impl fmt::Display for TypeCheck {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unit => write!(fmt, "Unit"),
            Self::Tuple => write!(fmt, "Tuple"),
            Self::Object => write!(fmt, "Object"),
            Self::Vec => write!(fmt, "Vec"),
            Self::Option(variant) => write!(fmt, "Option::{}", variant),
            Self::Result(variant) => write!(fmt, "Result::{}", variant),
            Self::GeneratorState(variant) => write!(fmt, "GeneratorState::{}", variant),
            Self::Type(hash) => write!(fmt, "Type({})", hash),
            Self::Variant(hash) => write!(fmt, "Variant({})", hash),
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
    /// Encode a function pointer on the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <fn>
    /// ```
    Fn {
        /// The hash to construct a function pointer from.
        hash: Hash,
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
    IndexGet,
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
    ObjectSlotIndexGet {
        /// The static string slot corresponding to the index to fetch.
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
    ObjectSlotIndexGetAt {
        /// The slot offset to get the value to test from.
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
    /// Push a literal integer.
    Integer {
        /// The number to push.
        number: i64,
    },
    /// Push a literal float into a slot.
    Float {
        /// The number to push.
        number: f64,
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
    Return,
    /// Pop the current stack frame and restore the instruction pointer from it.
    ///
    /// The stack frame will be cleared, and a unit value will be pushed to the
    /// top of the stack.
    ReturnUnit,
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
    /// condition is `false`.
    ///
    /// # Operation
    ///
    /// ```text
    /// <boolean>
    /// => *nothing*
    /// ```
    JumpIfNot {
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
    /// Push a unit value onto the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <unit>
    /// ```
    Unit,
    /// Push a boolean value onto the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <boolean>
    /// ```
    Bool {
        /// The boolean value to push.
        value: bool,
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
    TypedObject {
        /// The type of the object to construct.
        hash: Hash,
        /// The static slot of the object keys.
        slot: usize,
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
    VariantObject {
        /// The enum the variant belongs to.
        enum_hash: Hash,
        /// The type of the object to construct.
        hash: Hash,
        /// The static slot of the object keys.
        slot: usize,
    },
    /// Load a literal character.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <char>
    /// ```
    Char {
        /// The literal character to load.
        c: char,
    },
    /// Load a literal byte.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <byte>
    /// ```
    Byte {
        /// The literal byte to load.
        b: u8,
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
    /// Test if the top of the stack is a unit.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// => <boolean>
    /// ```
    IsUnit,
    /// Test if the top of the stack is a value.
    ///
    /// This expects the top of the stack to be an `option` or a `result`,
    /// and it is a value if these are either `Some` or `Ok`.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// => <boolean>
    /// ```
    IsValue,
    /// Unwrap a result from the top of the stack.
    /// This causes a vm error if the top of the stack is not an ok result.
    ///
    /// # Operation
    ///
    /// ```text
    /// <result>
    /// => <value>
    /// ```
    Unwrap,
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
        /// Type constraints that the object must match.
        type_check: TypeCheck,
        /// The slot of object keys to use.
        slot: usize,
        /// Whether the operation should check exact `true` or minimum length
        /// `false`.
        exact: bool,
    },
    /// Push the type with the given hash as a value on the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <value>
    /// ```
    Type {
        /// The hash of the type.
        hash: Hash,
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
    /// A built-in operation like `a + b` that takes its operands and pushes its
    /// result to and from the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// <value>
    /// => <value>
    /// ```
    StackNumeric {
        /// The actual operation.
        op: InstNumericOp,
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
    AssignNumeric {
        /// The target of the operation.
        target: InstTarget,
        /// The actual operation.
        op: InstNumericOp,
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

impl fmt::Display for Inst {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Drop { offset } => {
                write!(fmt, "drop {}", offset)?;
            }
            Self::Not => {
                write!(fmt, "not")?;
            }
            Self::Call { hash, args } => {
                write!(fmt, "call {}, {}", hash, args)?;
            }
            Self::CallInstance { hash, args } => {
                write!(fmt, "call-instance {}, {}", hash, args)?;
            }
            Self::Fn { hash } => {
                write!(fmt, "fn {}", hash)?;
            }
            Self::Closure { hash, count } => {
                write!(fmt, "closure {}, {}", hash, count)?;
            }
            Self::CallFn { args } => {
                write!(fmt, "call-fn {}", args)?;
            }
            Self::LoadInstanceFn { hash } => {
                write!(fmt, "load-instance-fn {}", hash)?;
            }
            Self::IndexGet => {
                write!(fmt, "index-get")?;
            }
            Self::TupleIndexGet { index } => {
                write!(fmt, "tuple-index-get {}", index)?;
            }
            Self::TupleIndexSet { index } => {
                write!(fmt, "tuple-index-set {}", index)?;
            }
            Self::TupleIndexGetAt { offset, index } => {
                write!(fmt, "tuple-index-get-at {}, {}", offset, index)?;
            }
            Self::ObjectSlotIndexGet { slot } => {
                write!(fmt, "object-slot-index-get {}", slot)?;
            }
            Self::ObjectSlotIndexGetAt { offset, slot } => {
                write!(fmt, "object-slot-index-get-at {}, {}", offset, slot)?;
            }
            Self::IndexSet => {
                write!(fmt, "index-set")?;
            }
            Self::Integer { number } => {
                write!(fmt, "integer {}", number)?;
            }
            Self::Float { number } => {
                write!(fmt, "float {}", number)?;
            }
            Self::Await => {
                write!(fmt, "await")?;
            }
            Self::Select { len } => {
                write!(fmt, "select {}", len)?;
            }
            Self::Pop => {
                write!(fmt, "pop")?;
            }
            Self::PopN { count } => {
                write!(fmt, "pop-n {}", count)?;
            }
            Self::PopAndJumpIfNot { count, offset } => {
                write!(fmt, "pop-and-jump-if-not {}, {}", count, offset)?;
            }
            Self::Clean { count } => {
                write!(fmt, "clean {}", count)?;
            }
            Self::Copy { offset } => {
                write!(fmt, "copy {}", offset)?;
            }
            Self::Dup => {
                write!(fmt, "dup")?;
            }
            Self::Replace { offset } => {
                write!(fmt, "replace {}", offset)?;
            }
            Self::Return => {
                write!(fmt, "return")?;
            }
            Self::ReturnUnit => {
                write!(fmt, "return-unit")?;
            }
            Self::Lt => {
                write!(fmt, "lt")?;
            }
            Self::Gt => {
                write!(fmt, "gt")?;
            }
            Self::Lte => {
                write!(fmt, "lte")?;
            }
            Self::Gte => {
                write!(fmt, "gte")?;
            }
            Self::Eq => {
                write!(fmt, "eq")?;
            }
            Self::Neq => {
                write!(fmt, "neq")?;
            }
            Self::Jump { offset } => {
                write!(fmt, "jump {}", offset)?;
            }
            Self::JumpIf { offset } => {
                write!(fmt, "jump-if {}", offset)?;
            }
            Self::JumpIfNot { offset } => {
                write!(fmt, "jump-if-not {}", offset)?;
            }
            Self::JumpIfBranch { branch, offset } => {
                write!(fmt, "jump-if-branch {}, {}", branch, offset)?;
            }
            Self::Unit => {
                write!(fmt, "unit")?;
            }
            Self::Bool { value } => {
                write!(fmt, "bool {}", value)?;
            }
            Self::Vec { count } => {
                write!(fmt, "vec {}", count)?;
            }
            Self::Tuple { count } => {
                write!(fmt, "tuple {}", count)?;
            }
            Self::PushTuple => {
                write!(fmt, "push-tuple")?;
            }
            Self::TypedObject { hash, slot } => {
                write!(fmt, "typed-object {}, {}", hash, slot)?;
            }
            Self::VariantObject {
                enum_hash,
                hash,
                slot,
            } => {
                write!(fmt, "variant-object {}, {}, {}", enum_hash, hash, slot)?;
            }
            Self::Object { slot } => {
                write!(fmt, "object {}", slot)?;
            }
            Self::String { slot } => {
                write!(fmt, "string {}", slot)?;
            }
            Self::Bytes { slot } => {
                write!(fmt, "bytes {}", slot)?;
            }
            Self::StringConcat { len, size_hint } => {
                write!(fmt, "string-concat {}, {}", len, size_hint)?;
            }
            Self::Char { c } => {
                write!(fmt, "char {:?}", c)?;
            }
            Self::Byte { b } => {
                write!(fmt, "byte {:?}", b)?;
            }
            Self::Is => {
                write!(fmt, "is")?;
            }
            Self::IsNot => {
                write!(fmt, "is-not")?;
            }
            Self::And => {
                write!(fmt, "and")?;
            }
            Self::Or => {
                write!(fmt, "or")?;
            }
            Self::IsUnit => {
                write!(fmt, "is-unit")?;
            }
            Self::IsValue => {
                write!(fmt, "is-value")?;
            }
            Self::Unwrap => {
                write!(fmt, "unwrap")?;
            }
            Self::EqByte { byte } => {
                write!(fmt, "eq-byte {:?}", byte)?;
            }
            Self::EqCharacter { character } => {
                write!(fmt, "eq-character {:?}", character)?;
            }
            Self::EqInteger { integer } => {
                write!(fmt, "eq-integer {}", integer)?;
            }
            Self::EqStaticString { slot } => {
                write!(fmt, "eq-static-string {}", slot)?;
            }
            Self::MatchSequence {
                type_check,
                len,
                exact,
            } => {
                write!(fmt, "match-sequence {}, {}, {}", type_check, len, exact)?;
            }
            Self::MatchObject {
                type_check,
                slot,
                exact,
            } => {
                write!(fmt, "match-object {}, {}, {}", type_check, slot, exact)?;
            }
            Self::Type { hash } => {
                write!(fmt, "type {}", hash)?;
            }
            Self::Yield => {
                write!(fmt, "yield")?;
            }
            Self::YieldUnit => {
                write!(fmt, "yield-unit")?;
            }
            Self::StackNumeric { op } => {
                write!(fmt, "stack-numeric {}", op)?;
            }
            Self::AssignNumeric { target, op } => {
                write!(fmt, "assign-numeric {}, {}", target, op)?;
            }
            Self::Panic { reason } => {
                write!(fmt, "panic {}", reason.ident())?;
            }
        }

        Ok(())
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
pub enum InstNumericOp {
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

impl fmt::Display for InstNumericOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Add => write!(f, "+"),
            Self::Sub => write!(f, "-"),
            Self::Mul => write!(f, "*"),
            Self::Div => write!(f, "/"),
            Self::Rem => write!(f, "%"),
            Self::BitAnd => write!(f, "&"),
            Self::BitXor => write!(f, "^"),
            Self::BitOr => write!(f, "|"),
            Self::Shl => write!(f, "<<"),
            Self::Shr => write!(f, ">>"),
        }
    }
}
