//! Public types related to using rune in #[no_std] environments.

/// Environment that needs to be stored somewhere.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct RawEnv {
    pub(crate) context: *const (),
    pub(crate) unit: *const (),
}

impl RawEnv {
    /// Initialize an empty raw environment.
    pub const fn null() -> RawEnv {
        RawEnv {
            context: core::ptr::null(),
            unit: core::ptr::null(),
        }
    }
}
