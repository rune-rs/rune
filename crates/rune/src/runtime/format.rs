//! Types for dealing with formatting specifications.

use core::fmt;
use core::iter;
use core::mem::take;
use core::num::NonZeroUsize;
use core::str;

#[cfg(feature = "musli")]
use musli_core::{Decode, Encode};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate as rune;
use crate::alloc::clone::TryClone;
use crate::alloc::fmt::TryWrite;
use crate::alloc::{self, String};
use crate::runtime::{Formatter, Inline, ProtocolCaller, Repr, Value, VmError, VmErrorKind};
use crate::{Any, TypeHash};

/// Error raised when trying to parse a type string and it fails.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct TypeFromStrError;

impl fmt::Display for TypeFromStrError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bad type string")
    }
}

/// Error raised when trying to parse an alignment string and it fails.
#[derive(Debug, Clone, Copy)]
pub struct AlignmentFromStrError;

impl fmt::Display for AlignmentFromStrError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bad alignment string")
    }
}

/// A format specification, wrapping an inner value.
#[derive(Any, Debug, Clone, TryClone)]
#[rune(item = ::std::fmt)]
pub struct Format {
    /// The value being formatted.
    #[rune(dismantle)]
    pub(crate) value: Value,
    /// The specification.
    #[try_clone(copy)]
    pub(crate) spec: FormatSpec,
}

/// A format specification.
#[derive(Debug, Clone, Copy, TryClone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "musli", derive(Decode, Encode), musli(crate = musli_core))]
#[try_clone(copy)]
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
    /// Formatting precision, which may be zero.
    pub(crate) precision: Option<usize>,
    /// The type specification.
    pub(crate) format_type: Type,
}

/// How a value is padded out to the width it was written with.
#[derive(Clone, Copy)]
struct Padding {
    align: Alignment,
    fill: char,
    /// The sign written in front of the digits, which is written apart from
    /// them so that zero padding can go between the two.
    sign: Option<char>,
    /// Whether the padding is what the value is written in rather than what it
    /// is surrounded by, which is what zero padding is.
    ///
    /// It is only ever asked for by a number: the flag which asks for it is
    /// ignored for everything else, which pads the way it was told to.
    zero: bool,
}

impl FormatSpec {
    /// Construct a new format specification.
    pub fn new(
        flags: Flags,
        fill: char,
        align: Alignment,
        width: Option<NonZeroUsize>,
        precision: Option<usize>,
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

    /// The padding a value which is not a number is written with.
    fn padding(&self) -> Padding {
        Padding {
            align: self.align,
            fill: self.fill,
            sign: None,
            zero: false,
        }
    }

    /// The padding a number which is being zero padded is written with.
    fn zero_padding(&self, sign: Option<char>) -> Padding {
        Padding {
            align: Alignment::Right,
            fill: '0',
            sign,
            zero: true,
        }
    }

    /// get traits out of a floating point number.
    fn float_traits(&self, n: f64) -> (f64, Padding) {
        if self.flags.test(Flag::SignAwareZeroPad) {
            if n.is_sign_negative() {
                (-n, self.zero_padding(Some('-')))
            } else {
                (n, self.zero_padding(self.zero_pad_sign()))
            }
        } else if self.flags.test(Flag::SignPlus) && n.is_sign_positive() {
            (
                n,
                Padding {
                    sign: Some('+'),
                    ..self.padding()
                },
            )
        } else {
            (n, self.padding())
        }
    }

    /// The sign to write in front of a number which is being zero padded.
    ///
    /// The padding goes between the sign and the digits, so the sign is written
    /// separately, and one which was asked for with `+` has to be written here
    /// as well as the one a negative number carries.
    fn zero_pad_sign(&self) -> Option<char> {
        if self.flags.test(Flag::SignPlus) {
            return Some('+');
        }

        None
    }

    /// get traits out of an unsigned integer.
    fn uint_traits(&self, n: u64) -> (u64, Padding) {
        if self.flags.test(Flag::SignAwareZeroPad) {
            (n, self.zero_padding(self.zero_pad_sign()))
        } else if self.flags.test(Flag::SignPlus) {
            (
                n,
                Padding {
                    sign: Some('+'),
                    ..self.padding()
                },
            )
        } else {
            (n, self.padding())
        }
    }

    /// get traits out of an integer.
    ///
    /// The sign is written separately from the digits, so the magnitude is
    /// returned rather than the number itself. It is taken as an unsigned
    /// number because the magnitude of the smallest signed number is not one.
    fn int_traits(&self, n: i64) -> (u64, Padding) {
        let magnitude = n.unsigned_abs();

        if self.flags.test(Flag::SignAwareZeroPad) {
            if n < 0 {
                (magnitude, self.zero_padding(Some('-')))
            } else {
                (magnitude, self.zero_padding(self.zero_pad_sign()))
            }
        } else if n < 0 {
            (
                magnitude,
                Padding {
                    sign: Some('-'),
                    ..self.padding()
                },
            )
        } else if self.flags.test(Flag::SignPlus) {
            (
                magnitude,
                Padding {
                    sign: Some('+'),
                    ..self.padding()
                },
            )
        } else {
            (magnitude, self.padding())
        }
    }

    /// Format the given unsigned number.
    fn format_unsigned(&self, buf: &mut String, n: u64) -> alloc::Result<()> {
        let mut buffer = itoa::Buffer::new();
        buf.try_push_str(buffer.format(n))?;
        Ok(())
    }

    /// Write a string, which the precision cuts short.
    fn format_str(&self, buf: &mut String, s: &str) -> alloc::Result<()> {
        let Some(precision) = self.precision else {
            buf.try_push_str(s)?;
            return Ok(());
        };

        for c in s.chars().take(precision) {
            buf.try_push(c)?;
        }

        Ok(())
    }

    /// Format the given float.
    fn format_float(&self, buf: &mut String, n: f64) -> alloc::Result<()> {
        if let Some(precision) = self.precision {
            write!(buf, "{:.*}", precision, n)?;
        } else {
            let mut buffer = ryu::Buffer::new();
            buf.try_push_str(buffer.format(n))?;
        }

        Ok(())
    }

    /// Format fill.
    fn format_fill(&self, f: &mut Formatter, padding: Padding) -> alloc::Result<()> {
        self.format_fill_with(f, padding, "")
    }

    /// Format fill, writing `prefix` between the sign and the digits.
    ///
    /// A sign and a radix prefix belong to the digits rather than to the
    /// padding, so they are written next to them and count towards the width
    /// the same way. Zero padding is the exception: it is what a number is
    /// written in rather than what it is surrounded by, so it goes between the
    /// sign and the digits.
    fn format_fill_with(
        &self,
        f: &mut Formatter,
        padding: Padding,
        prefix: &str,
    ) -> alloc::Result<()> {
        let Padding {
            align,
            fill,
            sign,
            zero,
        } = padding;

        let (f, buf) = f.parts_mut();

        let mut w = self.width.map(|n| n.get()).unwrap_or_default();

        w = w
            .saturating_sub(buf.chars().count())
            .saturating_sub(prefix.chars().count())
            .saturating_sub(sign.map(|_| 1).unwrap_or_default());

        let head = |f: &mut dyn TryWrite| {
            if let Some(sign) = sign {
                f.try_write_char(sign)?;
            }

            f.try_write_str(prefix)
        };

        if w == 0 {
            head(f)?;
            f.try_write_str(buf)?;
            return Ok(());
        }

        if zero {
            head(f)?;

            for c in iter::repeat_n(fill, w) {
                f.try_write_char(c)?;
            }

            f.try_write_str(buf)?;
            return Ok(());
        }

        let mut filler = iter::repeat_n(fill, w);

        match align {
            Alignment::Left => {
                head(f)?;
                f.try_write_str(buf)?;

                for c in filler {
                    f.try_write_char(c)?;
                }
            }
            Alignment::Center => {
                for c in (&mut filler).take(w / 2) {
                    f.try_write_char(c)?;
                }

                head(f)?;
                f.try_write_str(buf)?;

                for c in filler {
                    f.try_write_char(c)?;
                }
            }
            Alignment::Right => {
                for c in filler {
                    f.try_write_char(c)?;
                }

                head(f)?;
                f.try_write_str(buf)?;
            }
        }

        Ok(())
    }

    fn format_display(
        &self,
        value: &Value,
        f: &mut Formatter,
        caller: &mut dyn ProtocolCaller,
    ) -> Result<(), VmError> {
        'fallback: {
            match value.as_ref() {
                Repr::Inline(value) => match value {
                    Inline::Char(c) => {
                        f.buf_mut().try_push(*c)?;
                        self.format_fill(f, self.padding())?;
                    }
                    Inline::Signed(n) => {
                        let (n, padding) = self.int_traits(*n);
                        self.format_unsigned(f.buf_mut(), n)?;
                        self.format_fill(f, padding)?;
                    }
                    Inline::Unsigned(n) => {
                        let (n, padding) = self.uint_traits(*n);
                        self.format_unsigned(f.buf_mut(), n)?;
                        self.format_fill(f, padding)?;
                    }
                    Inline::Bool(b) => {
                        f.buf_mut()
                            .try_push_str(if *b { "true" } else { "false" })?;
                        self.format_fill(f, self.padding())?;
                    }
                    Inline::Float(n) => {
                        let (n, padding) = self.float_traits(*n);
                        self.format_float(f.buf_mut(), n)?;
                        self.format_fill(f, padding)?;
                    }
                    _ => {
                        break 'fallback;
                    }
                },
                Repr::Dynamic(..) => {
                    break 'fallback;
                }
                Repr::Any(value) => match value.type_hash() {
                    String::HASH => {
                        let s = value.borrow_ref::<String>()?;
                        self.format_str(f.buf_mut(), &s)?;
                        self.format_fill(f, self.padding())?;
                    }
                    _ => {
                        break 'fallback;
                    }
                },
            }

            return Ok(());
        }

        value.display_fmt_with(f, caller)
    }

    fn format_debug(
        &self,
        value: &Value,
        f: &mut Formatter,
        caller: &mut dyn ProtocolCaller,
    ) -> Result<(), VmError> {
        'fallback: {
            match value.as_ref() {
                Repr::Inline(value) => match value {
                    Inline::Signed(n) => {
                        let (n, padding) = self.int_traits(*n);
                        self.format_unsigned(f.buf_mut(), n)?;
                        self.format_fill(f, padding)?;
                    }
                    Inline::Unsigned(n) => {
                        let (n, padding) = self.uint_traits(*n);
                        self.format_unsigned(f.buf_mut(), n)?;
                        self.format_fill(f, padding)?;
                    }
                    Inline::Bool(b) => {
                        f.buf_mut()
                            .try_push_str(if *b { "true" } else { "false" })?;
                        self.format_fill(f, self.padding())?;
                    }
                    Inline::Float(n) => {
                        let (n, padding) = self.float_traits(*n);
                        self.format_float(f.buf_mut(), n)?;
                        self.format_fill(f, padding)?;
                    }
                    _ => {
                        break 'fallback;
                    }
                },
                Repr::Dynamic(..) => {
                    break 'fallback;
                }
                Repr::Any(value) => match value.type_hash() {
                    String::HASH => {
                        let s = value.borrow_ref::<String>()?;
                        write!(f, "{s:?}")?;
                    }
                    _ => {
                        break 'fallback;
                    }
                },
            }

            return Ok(());
        };

        value.debug_fmt_with(f, caller)
    }

    /// The prefix a number written in a radix carries when one was asked for
    /// with `#`.
    fn radix_prefix(&self, prefix: &'static str) -> &'static str {
        if self.flags.test(Flag::Alternate) {
            prefix
        } else {
            ""
        }
    }

    fn format_upper_hex(&self, value: &Value, f: &mut Formatter) -> Result<(), VmError> {
        match value.as_inline() {
            Some(Inline::Signed(n)) => {
                let (n, padding) = self.uint_traits(*n as u64);
                write!(f.buf_mut(), "{n:X}")?;
                self.format_fill_with(f, padding, self.radix_prefix("0x"))?;
            }
            Some(Inline::Unsigned(n)) => {
                let (n, padding) = self.uint_traits(*n);
                write!(f.buf_mut(), "{n:X}")?;
                self.format_fill_with(f, padding, self.radix_prefix("0x"))?;
            }
            _ => {
                return Err(VmError::new(VmErrorKind::IllegalFormat));
            }
        }

        Ok(())
    }

    fn format_lower_hex(&self, value: &Value, f: &mut Formatter) -> Result<(), VmError> {
        match value.as_inline() {
            Some(Inline::Signed(n)) => {
                let (n, padding) = self.uint_traits(*n as u64);
                write!(f.buf_mut(), "{n:x}")?;
                self.format_fill_with(f, padding, self.radix_prefix("0x"))?;
            }
            Some(Inline::Unsigned(n)) => {
                let (n, padding) = self.uint_traits(*n);
                write!(f.buf_mut(), "{n:x}")?;
                self.format_fill_with(f, padding, self.radix_prefix("0x"))?;
            }
            _ => {
                return Err(VmError::new(VmErrorKind::IllegalFormat));
            }
        }

        Ok(())
    }

    fn format_binary(&self, value: &Value, f: &mut Formatter) -> Result<(), VmError> {
        match value.as_inline() {
            Some(Inline::Signed(n)) => {
                let (n, padding) = self.uint_traits(*n as u64);
                write!(f.buf_mut(), "{n:b}")?;
                self.format_fill_with(f, padding, self.radix_prefix("0b"))?;
            }
            Some(Inline::Unsigned(n)) => {
                let (n, padding) = self.uint_traits(*n);
                write!(f.buf_mut(), "{n:b}")?;
                self.format_fill_with(f, padding, self.radix_prefix("0b"))?;
            }
            _ => {
                return Err(VmError::new(VmErrorKind::IllegalFormat));
            }
        }

        Ok(())
    }

    fn format_pointer(&self, value: &Value, f: &mut Formatter) -> Result<(), VmError> {
        match value.as_inline() {
            Some(Inline::Signed(n)) => {
                let (n, padding) = self.uint_traits(*n as u64);
                write!(f.buf_mut(), "{:p}", n as *const ())?;
                self.format_fill(f, padding)?;
            }
            _ => {
                return Err(VmError::new(VmErrorKind::IllegalFormat));
            }
        }

        Ok(())
    }

    /// Format the given value to the out buffer `out`, using `buf` for
    /// intermediate work if necessary.
    pub(crate) fn format(
        &self,
        value: &Value,
        f: &mut Formatter,
        caller: &mut dyn ProtocolCaller,
    ) -> Result<(), VmError> {
        f.buf_mut().clear();

        match self.format_type {
            Type::Display => self.format_display(value, f, caller)?,
            Type::Debug => self.format_debug(value, f, caller)?,
            Type::UpperHex => self.format_upper_hex(value, f)?,
            Type::LowerHex => self.format_lower_hex(value, f)?,
            Type::Binary => self.format_binary(value, f)?,
            Type::Pointer => self.format_pointer(value, f)?,
        }

        Ok(())
    }
}

impl fmt::Display for FormatSpec {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "format(fill = {fill:?}, align = {align}, flags = {flags:?}, width = {width}, precision = {precision}, format_type = {format_type})",
            fill = self.fill,
            align = self.align,
            flags = self.flags,
            width = OptionDebug(self.width.as_ref()),
            precision = OptionDebug(self.precision.as_ref()),
            format_type = self.format_type
        )
    }
}

struct OptionDebug<'a, T>(Option<&'a T>);

impl<T> fmt::Display for OptionDebug<'_, T>
where
    T: fmt::Display,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(value) => write!(f, "{value}"),
            None => write!(f, "?"),
        }
    }
}

/// The type of formatting requested.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "musli", derive(Decode, Encode), musli(crate = musli_core))]
#[non_exhaustive]
pub enum Type {
    /// Display type (default).
    #[default]
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

impl str::FromStr for Type {
    type Err = TypeFromStrError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "display" => Ok(Self::Display),
            "debug" => Ok(Self::Debug),
            "upper_hex" => Ok(Self::UpperHex),
            "lower_hex" => Ok(Self::LowerHex),
            "binary" => Ok(Self::Binary),
            "pointer" => Ok(Self::Pointer),
            _ => Err(TypeFromStrError),
        }
    }
}

impl fmt::Display for Type {
    #[inline]
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
#[derive(Default, Debug, Clone, Copy, TryClone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "musli", derive(Decode, Encode), musli(crate = musli_core))]
#[try_clone(copy)]
#[non_exhaustive]
pub enum Alignment {
    /// Left alignment.
    #[default]
    Left,
    /// Center alignment.
    Center,
    /// Right alignment.
    Right,
}

impl str::FromStr for Alignment {
    type Err = AlignmentFromStrError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "left" => Ok(Self::Left),
            "center" => Ok(Self::Center),
            "right" => Ok(Self::Right),
            _ => Err(AlignmentFromStrError),
        }
    }
}

impl fmt::Display for Alignment {
    #[inline]
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
    /// Alternate specifier `#`.
    Alternate,
    /// Sign-aware zero pad `0`.
    SignAwareZeroPad,
}

/// Format specification flags.
#[derive(Clone, Copy, TryClone, Default, PartialEq, Eq)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(transparent))]
#[cfg_attr(feature = "musli", derive(Decode, Encode), musli(crate = musli_core, transparent))]
#[try_clone(copy)]
pub struct Flags(u32);

impl Flags {
    /// Check if the set of flags is empty.
    #[inline]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Get the flags as a number. This representation is not guaranteed to be
    /// stable.
    #[inline]
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
                    if !take(&mut $o) {
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
