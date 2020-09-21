/// A constant value.
#[derive(Debug, Clone, Copy)]
pub enum ConstValue {
    /// A boolean constant value.
    Bool(bool),
    /// A string constant designated by its slot.
    String(usize),
    /// An integer constant.
    Integer(i64),
}
