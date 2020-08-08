use crate::hash::Hash;
use std::fmt;

/// The reason why a panic was invoked in the virtual machine.
#[derive(Debug, Clone, Copy)]
pub enum Panic {
    /// A pattern didn't match where it unconditionally has to.
    UnmatchedPattern,
}

impl Panic {
    /// The identifier of the panic.
    fn ident(&self) -> &'static str {
        match *self {
            Self::UnmatchedPattern => "unmatched-pattern",
        }
    }
}

impl fmt::Display for Panic {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::UnmatchedPattern => write!(fmt, "pattern did not match")?,
        }

        Ok(())
    }
}

/// An operation in the stack-based virtual machine.
#[derive(Debug, Clone, Copy)]
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
    /// Add two things together.
    ///
    /// This is the result of an `<a> + <b>` expression.
    Add,
    /// Add a value to the given frame offset.
    ///
    /// This is the result of an `<offset> += <b>` expression.
    AddAssign {
        /// The frame offset to assign to.
        offset: usize,
    },
    /// Subtract two things.
    ///
    /// This is the result of an `<a> - <b>` expression.
    Sub,
    /// Subtract a value to the given frame offset.
    ///
    /// This is the result of an `<offset> -= <b>` expression.
    SubAssign {
        /// The frame offset to assign to.
        offset: usize,
    },
    /// Multiply two things.
    ///
    /// This is the result of an `<a> * <b>` expression.
    Mul,
    /// Multiply a value to the given frame offset.
    ///
    /// This is the result of an `<offset> *= <b>` expression.
    MulAssign {
        /// The frame offset to assign to.
        offset: usize,
    },
    /// Divide two things.
    ///
    /// This is the result of an `<a> / <b>` expression.
    Div,
    /// Divide a value to the given frame offset.
    ///
    /// This is the result of an `<offset> /= <b>` expression.
    DivAssign {
        /// The frame offset to assign to.
        offset: usize,
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
    /// Get the given index out of an array on the top of the stack. Errors if
    /// the item doesn't exist or the item at the top of the stack is not an
    /// array.
    ///
    /// Note: this is a specialized variant of `ExprIndexGet` where we know that the
    /// top of the stack is supposed to be an array.
    ///
    /// # Operation
    ///
    /// ```text
    /// <array>
    /// => <value>
    /// ```
    ArrayIndexGet {
        /// The index to fetch.
        index: usize,
    },
    /// Get the given index out of an object on the top of the stack. Errors if
    /// the item doesn't exist or the item at the top of the stack is not an
    /// array.
    ///
    /// The index is identifier by a static string slot, which is provided as an
    /// argument.
    ///
    /// Note: this is a specialized variant of `ExprIndexGet` where we know that the
    /// top of the stack is supposed to be an array.
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
    /// Push a variable from a location `offset` relative to the current call
    /// frame.
    ///
    /// A copy is very cheap. It simply means pushing a reference to the stack
    /// and increasing a reference count.
    Copy {
        /// Offset to copy value from.
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
    /// Compares the `branch` register with `value`, and if they match performs
    /// the jump to offset.
    ///
    /// This will clear the `branch` register.
    JumpIfBranch {
        /// The branch value to compare against.
        branch: usize,
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
    /// Construct a push an array value onto the stack. The number of elements
    /// in the array are determined by `count` and are popped from the stack.
    Array {
        /// The size of the array.
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
    Object {
        /// The static slot of the object keys.
        slot: usize,
    },
    /// Load a literal character.
    Char {
        /// The literal character to load.
        c: char,
    },
    /// Load a literal string.
    String {
        /// The static string slot to load the string from.
        slot: usize,
    },
    /// Load a static, unmodifiable string from the given static string slot
    /// onto the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <value>
    /// ```
    StaticString {
        /// The static slot to load the string from.
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
    /// <value>
    /// <type>
    /// => <boolean>
    /// ```
    Is,
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
    /// Test if the top of the stack is an error.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// => <boolean>
    /// ```
    IsErr,
    /// Unwrap a result from the top of the stack.
    /// This causes a vm error if the top of the stack is not an ok result.
    ///
    /// # Operation
    ///
    /// ```text
    /// <result>
    /// => <value>
    /// ```
    ResultUnwrap,
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
    /// Test that the top of the stack is an array with the given length
    /// requirements.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// => <boolean>
    /// ```
    MatchArray {
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
    /// Cause the VM to panic and error out without a reason.
    ///
    /// This should only be used during testing or extreme scenarios that are
    /// completely unrecoverable.
    Panic {
        /// The mark of the panic.
        reason: Panic,
    },
}

impl fmt::Display for Inst {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Not => {
                write!(fmt, "not")?;
            }
            Self::Add => {
                write!(fmt, "add")?;
            }
            Self::AddAssign { offset } => {
                write!(fmt, "add-assign {}", offset)?;
            }
            Self::Sub => {
                write!(fmt, "sub")?;
            }
            Self::SubAssign { offset } => {
                write!(fmt, "sub-assign {}", offset)?;
            }
            Self::Mul => {
                write!(fmt, "mul")?;
            }
            Self::MulAssign { offset } => {
                write!(fmt, "mul-assign {}", offset)?;
            }
            Self::Div => {
                write!(fmt, "div")?;
            }
            Self::DivAssign { offset } => {
                write!(fmt, "div-assign {}", offset)?;
            }
            Self::Call { hash, args } => {
                write!(fmt, "call {}, {}", hash, args)?;
            }
            Self::CallInstance { hash, args } => {
                write!(fmt, "call-instance {}, {}", hash, args)?;
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
            Self::ArrayIndexGet { index } => {
                write!(fmt, "array-index-get {}", index)?;
            }
            Self::ObjectSlotIndexGet { slot } => {
                write!(fmt, "object-slot-index-get {}", slot)?;
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
            Self::Array { count } => {
                write!(fmt, "array {}", count)?;
            }
            Self::Object { slot } => {
                write!(fmt, "object {}", slot)?;
            }
            Self::String { slot } => {
                write!(fmt, "string {}", slot)?;
            }
            Self::StaticString { slot } => {
                write!(fmt, "static-string {}", slot)?;
            }
            Self::StringConcat { len, size_hint } => {
                write!(fmt, "string-concat {}, {}", len, size_hint)?;
            }
            Self::Char { c } => {
                write!(fmt, "char {:?}", c)?;
            }
            Self::Is => {
                write!(fmt, "is")?;
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
            Self::IsErr => {
                write!(fmt, "is-err")?;
            }
            Self::ResultUnwrap => {
                write!(fmt, "result-unwrap")?;
            }
            Self::EqCharacter { character } => {
                write!(fmt, "eq-character {}", character)?;
            }
            Self::EqInteger { integer } => {
                write!(fmt, "eq-integer {}", integer)?;
            }
            Self::EqStaticString { slot } => {
                write!(fmt, "eq-static-string {}", slot)?;
            }
            Self::MatchArray { len, exact } => {
                write!(fmt, "match-array {}, {}", len, exact)?;
            }
            Self::MatchObject { slot, exact } => {
                write!(fmt, "match-object {}, {}", slot, exact)?;
            }
            Self::Type { hash } => {
                write!(fmt, "type {}", hash)?;
            }
            Self::Panic { reason } => {
                write!(fmt, "panic {}", reason.ident())?;
            }
        }

        Ok(())
    }
}
