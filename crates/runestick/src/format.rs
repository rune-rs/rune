//! Types for dealing with formatting specifications.

use crate::{
    FromValue, Named, RawRef, RawStr, Ref, Shared, UnsafeFromValue, Value, VmError, VmErrorKind,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::num::NonZeroUsize;
use thiserror::Error;

/// Error raised when trying to parse a type string and it fails.
#[derive(Debug, Clone, Copy, Error)]
#[error("bad type string")]
pub struct TypeFromStrError(());

/// Error raised when trying to parse an alignment string and it fails.
#[derive(Debug, Clone, Copy, Error)]
#[error("bad alignment string")]
pub struct AlignmentFromStrError(());

/// A format specification, wrapping an inner value.
#[derive(Debug)]
pub struct Format {
    /// The value being formatted.
    pub(crate) value: Value,
    /// The specification.
    pub(crate) spec: FormatSpec,
}

/// A format specification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FormatSpec {
    /// Formatting flags.
    pub(crate) flags: Flags,
    /// The fill character.
    pub(crate) fill: char,
    /// The alignment specification.
    pub(crate) align: Alignment,
    /// Formatting width.
    pub(crate) width: Option<NonZeroUsize>,
    /// Formatting precision.
    pub(crate) precision: Option<NonZeroUsize>,
    /// The type specification.
    pub(crate) format_type: Type,
}

impl FormatSpec {
    /// Construct a new format specification.
    pub fn new(
        flags: Flags,
        fill: char,
        align: Alignment,
        width: Option<NonZeroUsize>,
        precision: Option<NonZeroUsize>,
        format_type: Type,
    ) -> Self {
        Self {
            flags,
            fill,
            align,
            width,
            precision,
            format_type,
        }
    }

    /// Format the given value to the out buffer `out`, using `buf` for
    /// intermediate work if necessary.
    pub(crate) fn format_with_buf(
        &self,
        value: &Value,
        out: &mut String,
        buf: &mut String,
    ) -> Result<(), VmErrorKind> {
        use std::fmt::Write as _;
        use std::iter;

        let mut fill = self.fill;

        buf.clear();
        let mut sign_aware = false;
        let mut sign = None;

        match value {
            Value::String(s) => match self.format_type {
                Type::Display => {
                    buf.push_str(&*s.borrow_ref()?);
                }
                Type::Debug => {
                    write!(out, "{:?}", &*s.borrow_ref()?).map_err(|_| VmErrorKind::FormatError)?;
                    return Ok(());
                }
                _ => return Err(VmErrorKind::FormatError),
            },
            Value::StaticString(s) => match self.format_type {
                Type::Display => {
                    buf.push_str(s.as_ref());
                }
                Type::Debug => {
                    write!(out, "{:?}", s.as_ref()).map_err(|_| VmErrorKind::FormatError)?;
                    return Ok(());
                }
                _ => return Err(VmErrorKind::FormatError),
            },
            Value::Integer(n) => {
                let mut n = *n;

                if self.flags.test(Flag::SignAwareZeroPad) {
                    fill = '0';
                    sign_aware = true;

                    if n < 0 {
                        sign = Some('-');
                        n = -n;
                    }
                } else if self.flags.test(Flag::SignPlus) && n >= 0 {
                    sign = Some('+');
                }

                match self.format_type {
                    Type::Display | Type::Debug => {
                        let mut buffer = itoa::Buffer::new();
                        buf.push_str(buffer.format(n));
                    }
                    Type::UpperHex => {
                        write!(buf, "{:X}", n).map_err(|_| VmErrorKind::FormatError)?;
                    }
                    Type::LowerHex => {
                        write!(buf, "{:x}", n).map_err(|_| VmErrorKind::FormatError)?;
                    }
                    Type::Binary => {
                        write!(buf, "{:b}", n).map_err(|_| VmErrorKind::FormatError)?;
                    }
                    _ => {
                        return Err(VmErrorKind::FormatError);
                    }
                }
            }
            Value::Float(n) => {
                let mut n = *n;

                if self.flags.test(Flag::SignAwareZeroPad) {
                    fill = '0';
                    sign_aware = true;

                    if n.is_sign_negative() {
                        sign = Some('-');
                        n = -n;
                    }
                } else if self.flags.test(Flag::SignPlus) && n.is_sign_positive() {
                    sign = Some('+');
                }

                match self.format_type {
                    Type::Display | Type::Debug => {
                        if let Some(precision) = self.precision {
                            write!(buf, "{:.*}", precision.get(), n)
                                .map_err(|_| VmErrorKind::FormatError)?;
                        } else {
                            let mut buffer = ryu::Buffer::new();
                            buf.push_str(buffer.format(n));
                        }
                    }
                    _ => return Err(VmErrorKind::FormatError),
                }
            }
            value => {
                if let Type::Debug = self.format_type {
                    write!(buf, "{:?}", value).map_err(|_| VmErrorKind::FormatError)?;
                    return Ok(());
                }

                return Err(VmErrorKind::FormatError);
            }
        }

        let extra = self
            .width
            .map(|n| n.get())
            .unwrap_or_default()
            .saturating_sub(buf.len())
            .saturating_sub(sign.map(|_| 1).unwrap_or_default());

        if extra > 0 {
            let mut filler = iter::repeat(fill).take(extra);

            if let Some(sign) = sign.take() {
                out.push(sign);
            }

            if sign_aware {
                out.extend(filler);
                out.push_str(&buf);
            } else {
                match self.align {
                    Alignment::Left => {
                        out.push_str(&buf);
                        out.extend(filler);
                    }
                    Alignment::Center => {
                        out.extend((&mut filler).take(extra / 2));
                        out.push_str(&buf);
                        out.extend(filler);
                    }
                    Alignment::Right => {
                        out.extend(filler);
                        out.push_str(&buf);
                    }
                }
            }
        } else {
            if let Some(sign) = sign.take() {
                out.push(sign);
            }

            out.push_str(&buf);
        }

        Ok(())
    }
}

impl Named for Format {
    const NAME: RawStr = RawStr::from_str("Format");
}

impl FromValue for Shared<Format> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_format()?)
    }
}

impl FromValue for Format {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_format()?.take()?)
    }
}

impl UnsafeFromValue for &Format {
    type Output = *const Format;
    type Guard = RawRef;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let generator = value.into_format()?;
        let (generator, guard) = Ref::into_raw(generator.into_ref()?);
        Ok((generator, guard))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

/// The type of formatting requested.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Type {
    /// Display type (default).
    Display,
    /// Debug type.
    Debug,
    /// Upper hex type.
    UpperHex,
    /// Upper hex type.
    LowerHex,
    /// Binary formatting type.
    Binary,
    /// Pointer formatting type.
    Pointer,
}

impl std::str::FromStr for Type {
    type Err = TypeFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "display" => Ok(Self::Display),
            "debug" => Ok(Self::Debug),
            "upper_hex" => Ok(Self::UpperHex),
            "lower_hex" => Ok(Self::LowerHex),
            "binary" => Ok(Self::Binary),
            "pointer" => Ok(Self::Pointer),
            _ => Err(TypeFromStrError(())),
        }
    }
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
            Self::UpperHex => {
                write!(f, "upper_hex")?;
            }
            Self::LowerHex => {
                write!(f, "lower_hex")?;
            }
            Self::Binary => {
                write!(f, "binary")?;
            }
            Self::Pointer => {
                write!(f, "pointer")?;
            }
        }

        Ok(())
    }
}

/// The alignment requested.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Alignment {
    /// Left alignment.
    Left,
    /// Center alignment.
    Center,
    /// Right alignment.
    Right,
}

impl Default for Alignment {
    fn default() -> Self {
        Self::Left
    }
}

impl std::str::FromStr for Alignment {
    type Err = AlignmentFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "left" => Ok(Self::Left),
            "center" => Ok(Self::Center),
            "right" => Ok(Self::Right),
            _ => Err(AlignmentFromStrError(())),
        }
    }
}

impl fmt::Display for Alignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Left => {
                write!(f, "left")?;
            }
            Self::Center => {
                write!(f, "center")?;
            }
            Self::Right => {
                write!(f, "right")?;
            }
        }

        Ok(())
    }
}

/// A single flag for format spec.
#[derive(Clone, Copy)]
#[repr(u32)]
#[non_exhaustive]
pub enum Flag {
    /// Plus sign `+`.
    SignPlus,
    /// Minus sign `-`.
    SignMinus,
    /// Atlernate specifier `#`.
    Alternate,
    /// Sign-aware zero pad `0`.
    SignAwareZeroPad,
}

/// Format specification flags.
#[derive(Clone, Copy, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Flags(u32);

impl Flags {
    /// Check if the set of flags is empty.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Get the flags as a number. This representation is not guaranteed to be
    /// stable.
    pub fn into_u32(self) -> u32 {
        self.0
    }

    /// Set the given flag.
    #[inline]
    pub fn set(&mut self, flag: Flag) {
        self.0 |= &(1 << flag as u32);
    }

    /// Test the given flag.
    #[inline]
    pub fn test(&self, flag: Flag) -> bool {
        (self.0 & (1 << flag as u32)) != 0
    }
}

impl From<u32> for Flags {
    fn from(flags: u32) -> Self {
        Self(flags)
    }
}

impl fmt::Debug for Flags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        macro_rules! fmt_flag {
            ($flag:ident, $o:ident, $spec:literal) => {
                if self.test(Flag::$flag) {
                    if !std::mem::take(&mut $o) {
                        write!(f, ", ")?;
                    }

                    write!(f, $spec)?;
                }
            };
        }

        let mut o = true;
        write!(f, "Flags{{")?;
        fmt_flag!(SignPlus, o, "+");
        fmt_flag!(SignMinus, o, "-");
        fmt_flag!(Alternate, o, "#");
        fmt_flag!(SignAwareZeroPad, o, "0");
        write!(f, "}}")?;
        Ok(())
    }
}
