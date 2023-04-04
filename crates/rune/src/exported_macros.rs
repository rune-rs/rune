/// Helper to perform the try operation over
/// [`VmResult`][crate::runtime::VmResult].
#[macro_export]
macro_rules! vm_try {
    ($expr:expr) => {
        match $crate::runtime::try_result($expr) {
            $crate::runtime::VmResult::Ok(value) => value,
            $crate::runtime::VmResult::Err(err) => return $crate::runtime::VmResult::Err(err),
        }
    };
}
