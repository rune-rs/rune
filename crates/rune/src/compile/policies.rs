/// The policy to apply.
#[derive(Clone, Copy)]
pub(crate) enum Policy {
    /// Allow the given action.
    Allow,
    /// Warn about the given action.
    Warn,
    /// Deny the given action.
    Deny,
}

pub(crate) struct Policies {
    /// Policy to use when a pattern might panic.
    pub(crate) pattern_might_panic: Policy,
}

impl Default for Policies {
    #[inline]
    fn default() -> Self {
        Self {
            pattern_might_panic: Policy::Warn,
        }
    }
}
