use serde::{Deserialize, Serialize};

/// A constant value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConstValue {
    /// A boolean constant value.
    Bool(bool),
    /// A string constant.
    String(usize),
}
