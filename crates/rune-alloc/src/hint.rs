cfg_if! {
    if #[cfg(rune_nightly)] {
        pub(crate) use core::intrinsics::{likely, unlikely, assume};
    } else {
        pub(crate) use core::convert::{identity as likely, identity as unlikely};

        #[inline(always)]
        pub(crate) fn assume(_: bool) {
            // do nothing
        }
    }
}
