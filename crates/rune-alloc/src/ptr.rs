//! This module contains (hopefully sound) re-implementations of unstable
//! `core::ptr` APIs.

pub(crate) use self::unique::Unique;
mod unique;
