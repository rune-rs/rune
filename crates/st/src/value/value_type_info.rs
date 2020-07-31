use std::fmt;

/// Type information about a value, that can be printed for human consumption
/// through its [Display][fmt::Display] implementation.
#[derive(Debug, Clone, Copy)]
pub enum ValueTypeInfo {
    /// An empty unit.
    Unit,
    /// A string.
    String,
    /// An array.
    Array,
    /// An object.
    Object,
    /// A number.
    Integer,
    /// A float.
    Float,
    /// A boolean.
    Bool,
    /// A character.
    Char,
    /// Reference to a foreign type.
    External(&'static str),
    /// The type of a value.
    Type,
}

impl fmt::Display for ValueTypeInfo {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ValueTypeInfo::Unit => {
                write!(fmt, "unit")?;
            }
            ValueTypeInfo::String => {
                write!(fmt, "String")?;
            }
            ValueTypeInfo::Array => {
                write!(fmt, "Array")?;
            }
            ValueTypeInfo::Object => {
                write!(fmt, "Object")?;
            }
            ValueTypeInfo::Integer => {
                write!(fmt, "int")?;
            }
            ValueTypeInfo::Float => {
                write!(fmt, "float")?;
            }
            ValueTypeInfo::Bool => {
                write!(fmt, "bool")?;
            }
            ValueTypeInfo::Char => {
                write!(fmt, "char")?;
            }
            ValueTypeInfo::External(type_name) => {
                write!(fmt, "{}", type_name)?;
            }
            ValueTypeInfo::Type => {
                write!(fmt, "type")?;
            }
        }

        Ok(())
    }
}
