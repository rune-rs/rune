//! Base package for frontends.

#![deny(missing_docs)]

/// Trait used to encode an object into a collection of instructions.
pub trait Encode {
    /// The error that can be raised by the encodeable object.
    type Err: std::error::Error;

    /// Encode the given object into a collection of instructions.
    fn encode(self) -> Result<st::Unit, Self::Err>;
}
