//! Types for dealing with formatting specifications.

use crate::{FromValue, Named, RawRef, RawStr, Ref, Shared, UnsafeFromValue, Value, VmError};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A format specification, wrapping an inner value.
#[derive(Debug)]
pub struct FormatSpec {
    pub(crate) value: Value,
    pub(crate) ty: Type,
}

impl Named for FormatSpec {
    const NAME: RawStr = RawStr::from_str("FormatSpec");
}

impl FromValue for Shared<FormatSpec> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_format_spec()?)
    }
}

impl FromValue for FormatSpec {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_format_spec()?.take()?)
    }
}

impl UnsafeFromValue for &FormatSpec {
    type Output = *const FormatSpec;
    type Guard = RawRef;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let generator = value.into_format_spec()?;
        let (generator, guard) = Ref::into_raw(generator.into_ref()?);
        Ok((generator, guard))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

/// The type of formatting requested.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Type {
    /// Display type (default).
    Display,
    /// Debug type.
    Debug,
}

impl Default for Type {
    fn default() -> Self {
        Self::Display
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Display => {
                write!(f, "display")?;
            }
            Self::Debug => {
                write!(f, "debug")?;
            }
        }

        Ok(())
    }
}
