/// The trait used to identify when to search for fields
/// within a `AnyObj` via `Protocol::DYNAMIC_FIELD_GET` and
/// `Protocol::DYNAMIC_FIELD_SET`
pub trait DynamicFieldSearch {
    /// When to use Protocol::DYNAMIC_FIELD_SET/Protocol::DYNAMIC_FIELD_GET over Protocol::SET/Protocol::GET.
    const DYNAMIC_FIELD_MODE: DynamicFieldMode;
}

/// The possible values for the `MetaFieldMode` trait.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum DynamicFieldMode {
    /// Never use `Protocol::DYNAMIC_FIELD_GET` or `Protocol::DYNAMIC_FIELD_SET`
    Never,
    /// Use `Protocol::DYNAMIC_FIELD_GET` or `Protocol::DYNAMIC_FIELD_SET` before
    /// `Protocol::GET` and `Protocol::SET` respectively.
    First,
    /// Use `Protocol::GET` or `Protocol::SET` before
    /// `Protocol::DYNAMIC_FIELD_GET` and `Protocol::DYNAMIC_FIELD_SET` respectively.
    Last,
    /// Use `Protocol::DYNAMIC_FIELD_GET` or `Protocol::DYNAMIC_FIELD_SET` exclusively.
    Only,
}
