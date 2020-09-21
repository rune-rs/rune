/// A constant value.
#[derive(Debug, Clone)]
pub enum ConstValue {
    /// A boolean constant value.
    Bool(bool),
    /// A string constant designated by its slot.
    String(Box<str>),
    /// An integer constant.
    Integer(i64),
    /// An float constant.
    Float(f64),
}
