pub(crate) mod aserror;
pub(crate) mod display;
pub(crate) use ::thiserror_impl::Error;

#[doc(hidden)]
pub(crate) mod __private {
    pub(crate) use super::aserror::AsDynError;
    pub(crate) use super::display::{DisplayAsDisplay, PathAsDisplay};
}
