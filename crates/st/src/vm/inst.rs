use crate::hash::Hash;
use std::fmt;

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
    /// Subtract two things.
    ///
    /// This is the result of an `<a> - <b>` expression.
    Sub,
    /// Divide two things.
    ///
    /// This is the result of an `<a> / <b>` expression.
    Div,
    /// Multiply two things.
    ///
    /// This is the result of an `<a> * <b>` expression.
    Mul,
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
    /// Pop the value on the stack.
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
    /// Replace a value at the offset relative from the top of the stack, with
    /// the top of the stack.
    Replace {
        /// Offset to swap value from.
        offset: usize,
    },
    /// Pop a reference from the stack and replace what it points to with the
    /// value on the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// <ptr>
    /// <value>
    /// => *noop*
    /// ```
    ReplaceDeref,
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
    /// in the object are determined by `count` and are popped from the stack.
    ///
    /// For each element, a key and a value is popped.
    Object {
        /// The size of the object.
        count: usize,
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
    /// Test if the top of the stack is a unit.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// => <boolean>
    /// ```
    IsUnit,
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
    /// Construct a ptr to the given stack location relative to the current
    /// frame and push it on the stack.
    ///
    /// # Operation
    ///
    /// ```text
    /// => <value>
    /// ```
    Ptr {
        /// The offset to construct a pointer out of in the current stack frame.
        offset: usize,
    },
    /// Derefence the top of the stack. Dereferenced value must be a reference.
    ///
    /// # Operation
    ///
    /// ```text
    /// <value>
    /// => <value>
    /// ```
    Deref,
}

impl fmt::Display for Inst {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Inst::Not => {
                write!(fmt, "not")?;
            }
            Inst::Add => {
                write!(fmt, "add")?;
            }
            Inst::Sub => {
                write!(fmt, "sub")?;
            }
            Inst::Div => {
                write!(fmt, "div")?;
            }
            Inst::Mul => {
                write!(fmt, "mul")?;
            }
            Inst::Call { hash, args } => {
                write!(fmt, "call {}, {}", hash, args)?;
            }
            Inst::CallInstance { hash, args } => {
                write!(fmt, "call-instance {}, {}", hash, args)?;
            }
            Inst::IndexGet => {
                write!(fmt, "index-get")?;
            }
            Inst::IndexSet => {
                write!(fmt, "index-set")?;
            }
            Inst::Integer { number } => {
                write!(fmt, "integer {}", number)?;
            }
            Inst::Float { number } => {
                write!(fmt, "float {}", number)?;
            }
            Inst::Pop => {
                write!(fmt, "pop")?;
            }
            Inst::PopN { count } => {
                write!(fmt, "pop-n {}", count)?;
            }
            Inst::Clean { count } => {
                write!(fmt, "clean {}", count)?;
            }
            Inst::Copy { offset } => {
                write!(fmt, "copy {}", offset)?;
            }
            Inst::Replace { offset } => {
                write!(fmt, "replace {}", offset)?;
            }
            Inst::ReplaceDeref => {
                write!(fmt, "replace-deref")?;
            }
            Inst::Return => {
                write!(fmt, "return")?;
            }
            Inst::ReturnUnit => {
                write!(fmt, "return-unit")?;
            }
            Inst::Lt => {
                write!(fmt, "lt")?;
            }
            Inst::Gt => {
                write!(fmt, "gt")?;
            }
            Inst::Lte => {
                write!(fmt, "lte")?;
            }
            Inst::Gte => {
                write!(fmt, "gte")?;
            }
            Inst::Eq => {
                write!(fmt, "eq")?;
            }
            Inst::Neq => {
                write!(fmt, "neq")?;
            }
            Inst::Jump { offset } => {
                write!(fmt, "jump {}", offset)?;
            }
            Inst::JumpIf { offset } => {
                write!(fmt, "jump-if {}", offset)?;
            }
            Inst::JumpIfNot { offset } => {
                write!(fmt, "jump-if-not {}", offset)?;
            }
            Inst::Unit => {
                write!(fmt, "unit")?;
            }
            Inst::Bool { value } => {
                write!(fmt, "bool {}", value)?;
            }
            Inst::Array { count } => {
                write!(fmt, "array {}", count)?;
            }
            Inst::Object { count } => {
                write!(fmt, "object {}", count)?;
            }
            Inst::String { slot } => {
                write!(fmt, "string {}", slot)?;
            }
            Inst::Char { c } => {
                write!(fmt, "char {:?}", c)?;
            }
            Inst::Is => {
                write!(fmt, "is")?;
            }
            Inst::IsUnit => {
                write!(fmt, "is-unit")?;
            }
            Inst::Type { hash } => {
                write!(fmt, "type {}", hash)?;
            }
            Inst::Ptr { offset } => {
                write!(fmt, "ptr {}", offset)?;
            }
            Inst::Deref => {
                write!(fmt, "deref")?;
            }
        }

        Ok(())
    }
}
