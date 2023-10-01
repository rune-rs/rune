use core::cmp;
use core::fmt;
use core::hash::{self, Hash as _};
use core::ops;

use crate as rune;
#[cfg(feature = "alloc")]
use crate::alloc;
use crate::alloc::prelude::*;
use crate::hash::IntoHash;
use crate::hash::{Hash, ToTypeHash};
use crate::item::ItemBuf;

/// A built in instance function.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Protocol {
    /// The name of the builtin function.
    pub name: &'static str,
    /// The hash of the builtin function.
    pub hash: Hash,
    /// Representative expression for the protocol.
    ///
    /// If no such expression is present, then it means that its an internal
    /// protocol.
    #[cfg(feature = "doc")]
    #[doc(hidden)]
    pub repr: Option<&'static str>,
    /// Documentation for protocol.
    #[cfg(feature = "doc")]
    #[doc(hidden)]
    pub doc: &'static [&'static str],
}

impl IntoHash for Protocol {
    #[inline]
    fn into_hash(self) -> Hash {
        self.hash
    }
}

impl ToTypeHash for Protocol {
    #[inline]
    fn to_type_hash(&self) -> Hash {
        self.hash
    }

    #[inline]
    #[cfg(feature = "alloc")]
    fn to_item(&self) -> alloc::Result<Option<ItemBuf>> {
        Ok(None)
    }

    #[inline]
    fn hash_type<H>(&self, hasher: &mut H)
    where
        H: hash::Hasher,
    {
        self.hash.hash(hasher);
    }
}

impl ops::Deref for Protocol {
    type Target = Hash;

    fn deref(&self) -> &Self::Target {
        &self.hash
    }
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl cmp::PartialEq for Protocol {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.hash.eq(&other.hash)
    }
}

impl cmp::Eq for Protocol {}

impl hash::Hash for Protocol {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.hash.hash(state)
    }
}

macro_rules! define {
    (
        $(
            $(#[$($meta:meta)*])*
            $vis:vis const [$ident:ident, $hash_ident:ident]: Protocol = Protocol {
                name: $name:expr,
                hash: $hash:expr,
                repr: $repr:expr,
                doc: $doc:expr $(,)?
            };
        )*
    ) => {
        impl Protocol {
            $(
                $(#[$($meta)*])*
                $vis const $ident: Protocol = Protocol {
                    name: $name,
                    hash: Hash::new($hash),
                    #[cfg(feature = "doc")]
                    repr: $repr,
                    #[cfg(feature = "doc")]
                    doc: &$doc,
                };

                $vis const $hash_ident: Hash = Hash::new($hash);
            )*

            /// Look up protocol for the given hash.
            pub fn from_hash(hash: Hash) -> Option<Self> {
                match hash {
                    $(
                        Self::$hash_ident => {
                            Some(Self::$ident)
                        },
                    )*
                    _ => None,
                }
            }
        }

        #[test]
        fn ensure_unique_hashes() {
            let mut map = ::rust_std::collections::HashMap::<_, &'static str>::new();

            $(
                if let Some(ident) = map.insert($hash, stringify!($ident)) {
                    panic!("Trying to define protocol hash `{}` for `{}`, but it's already defined for {ident}", $hash, stringify!($ident));
                }
            )*
        }
    }
}

define! {
    /// The function to access a field.
    pub const [GET, GET_HASH]: Protocol = Protocol {
        name: "get",
        hash: 0x504007af1a8485a4u64,
        repr: Some("let output = $value"),
        doc: ["Allows a get operation to work."],
    };

    /// The function to set a field.
    pub const [SET, SET_HASH]: Protocol = Protocol {
        name: "set",
        hash: 0x7d13d47fd8efef5au64,
        repr: Some("$value = input"),
        doc: ["Allows a set operation to work."],
    };

    /// The function to access an index.
    pub const [INDEX_GET, INDEX_GET_HASH]: Protocol = Protocol {
        name: "index_get",
        hash: 0xadb5b27e2a4d2decu64,
        repr: Some("let output = $value[index]"),
        doc: ["Allows an indexing get operation to work."],
    };

    /// The function to set an index.
    pub const [INDEX_SET, INDEX_SET_HASH]: Protocol = Protocol {
        name: "index_set",
        hash: 0x162943f7bd03ad36u64,
        repr: Some("$value[index] = input"),
        doc: ["Allows an indexing set operation to work."],
    };

    /// Check two types for partial equality.
    pub const [PARTIAL_EQ, PARTIAL_EQ_HASH]: Protocol = Protocol {
        name: "partial_eq",
        hash: 0x4b6bc4701445e318u64,
        repr: Some("if $value == b { }"),
        doc: ["Allows for partial equality operations to work."],
    };

    /// Check two types for total equality.
    pub const [EQ, EQ_HASH]: Protocol = Protocol {
        name: "eq",
        hash: 0x418f5becbf885806u64,
        repr: Some("if $value == b { }"),
        doc: ["Allows an equality operation to work."],
    };

    /// Perform an partial comparison between two values.
    pub const [PARTIAL_CMP, PARTIAL_CMP_HASH]: Protocol = Protocol {
        name: "partial_cmp",
        hash: 0x8d4430991253343cu64,
        repr: Some("if $value < b { }"),
        doc: ["Allows for partial ordering to work."],
    };

    /// Perform an total comparison between two values.
    pub const [CMP, CMP_HASH]: Protocol = Protocol {
        name: "cmp",
        hash: 0x240f1b75466cd1a3u64,
        repr: Some("if $value < b { }"),
        doc: ["Allows for total ordering to work."],
    };

    /// The function to implement for the addition operation.
    pub const [ADD, ADD_HASH]: Protocol = Protocol {
        name: "add",
        hash: 0xe4ecf51fa0bf1076u64,
        repr: Some("let output = $value + b"),
        doc: [
            "Allows the `+` operator to apply to values of this type, where the current type is the left-hand side."
        ],
    };

    /// The function to implement for the addition assign operation.
    pub const [ADD_ASSIGN, ADD_ASSIGN_HASH]: Protocol = Protocol {
        name: "add_assign",
        hash: 0x42451ccb0a2071a9u64,
        repr: Some("$value += b"),
        doc: [
            "Allows the `+=` operator to apply to values of this type, where the current type is the left-hand side."
        ],
    };

    /// The function to implement for the subtraction operation.
    pub const [SUB, SUB_HASH]: Protocol = Protocol {
        name: "sub",
        hash: 0x6fa86a5f18d0bf71u64,
        repr: Some("let output = $value - b"),
        doc: [
            "Allows the `-` operator to apply to values of this type, where the current type is the left-hand side."
        ],
    };

    /// The function to implement for the subtraction assign operation.
    pub const [SUB_ASSIGN, SUB_ASSIGN_HASH]: Protocol = Protocol {
        name: "sub_assign",
        hash: 0x5939bb56a1415284u64,
        repr: Some("$value -= b"),
        doc: [
            "Allows the `-=` operator to apply to values of this type, where the current type is the left-hand side."
        ],
    };

    /// The function to implement for the multiply operation.
    pub const [MUL, MUL_HASH]: Protocol = Protocol {
        name: "mul",
        hash: 0xb09e99dc94091d1cu64,
        repr: Some("let output = $value * b"),
        doc: [
            "Allows the `*` operator to apply to values of this type, where the current type is the left-hand side."
        ],
    };

    /// The function to implement for the multiply assign operation.
    pub const [MUL_ASSIGN, MUL_ASSIGN_HASH]: Protocol = Protocol {
        name: "mul_assign",
        hash: 0x29a54b727f980ebfu64,
        repr: Some("$value *= b"),
        doc: [
            "Allows the `*=` operator to apply to values of this type, where the current type is the left-hand side."
        ],
    };

    /// The function to implement for the division operation.
    pub const [DIV, DIV_HASH]: Protocol = Protocol {
        name: "div",
        hash: 0xf26d6eea1afca6e8u64,
        repr: Some("let output = $value / b"),
        doc: [
            "Allows the `/` operator to apply to values of this type, where the current type is the left-hand side."
        ],
    };

    /// The function to implement for the division assign operation.
    pub const [DIV_ASSIGN, DIV_ASSIGN_HASH]: Protocol = Protocol {
        name: "div_assign",
        hash: 0x4dd087a8281c04e6u64,
        repr: Some("$value /= b"),
        doc: [
            "Allows the `/=` operator to apply to values of this type, where the current type is the left-hand side."
        ],
    };

    /// The function to implement for the remainder operation.
    pub const [REM, REM_HASH]: Protocol = Protocol {
        name: "rem",
        hash: 0x5c6293639c74e671u64,
        repr: Some("let output = $value % b"),
        doc: [
            "Allows the `%` operator to apply to values of this type, where the current type is the left-hand side."
        ],
    };

    /// The function to implement for the remainder assign operation.
    pub const [REM_ASSIGN, REM_ASSIGN_HASH]: Protocol = Protocol {
        name: "rem_assign",
        hash: 0x3a8695980e77baf4u64,
        repr: Some("$value %= b"),
        doc: [
            "Allows the `%=` operator to apply to values of this type, where the current type is the left-hand side."
        ],
    };

    /// The function to implement for the bitwise and operation.
    pub const [BIT_AND, BIT_AND_HASH]: Protocol = Protocol {
        name: "bit_and",
        hash: 0x0e11f20d940eebe8u64,
        repr: Some("let output = $value & b"),
        doc: [
            "Allows the `&` operator to apply to values of this type, where the current type is the left-hand side."
        ],
    };

    /// The function to implement for the bitwise and assign operation.
    pub const [BIT_AND_ASSIGN, BIT_AND_ASSIGN_HASH]: Protocol = Protocol {
        name: "bit_and_assign",
        hash: 0x95cb1ba235dfb5ecu64,
        repr: Some("$value &= b"),
        doc: [
            "Allows the `&=` operator to apply to values of this type, where the current type is the left-hand side."
        ],
    };

    /// The function to implement for the bitwise xor operation.
    pub const [BIT_XOR, BIT_XOR_HASH]: Protocol = Protocol {
        name: "bit_xor",
        hash: 0xa3099c54e1de4cbfu64,
        repr: Some("let output = $value ^ b"),
        doc: [
            "Allows the `^` operator to apply to values of this type, where the current type is the left-hand side."
        ],
    };

    /// The function to implement for the bitwise xor assign operation.
    pub const [BIT_XOR_ASSIGN, BIT_XOR_ASSIGN_HASH]: Protocol = Protocol {
        name: "bit_xor_assign",
        hash: 0x01fa9706738f9867u64,
        repr: Some("$value ^= b"),
        doc: [
            "Allows the `^=` operator to apply to values of this type, where the current type is the left-hand side."
        ],
    };

    /// The function to implement for the bitwise or operation.
    pub const [BIT_OR, BIT_OR_HASH]: Protocol = Protocol {
        name: "bit_or",
        hash: 0x05010afceb4a03d0u64,
        repr: Some("let output = $value | b"),
        doc: [
            "Allows the `|` operator to apply to values of this type, where the current type is the left-hand side."
        ],
    };

    /// The function to implement for the bitwise xor assign operation.
    pub const [BIT_OR_ASSIGN, BIT_OR_ASSIGN_HASH]: Protocol = Protocol {
        name: "bit_or_assign",
        hash: 0x606d79ff1750a7ecu64,
        repr: Some("$value |= b"),
        doc: [
            "Allows the `|=` operator to apply to values of this type, where the current type is the left-hand side."
        ],
    };

    /// The function to implement for the bitwise shift left operation.
    pub const [SHL, SHL_HASH]: Protocol = Protocol {
        name: "shl",
        hash: 0x6845f7d0cc9e002du64,
        repr: Some("let output = $value << b"),
        doc: [
            "Allows the `<<` operator to apply to values of this type, where the current type is the left-hand side."
        ],
    };

    /// The function to implement for the bitwise shift left assign operation.
    pub const [SHL_ASSIGN, SHL_ASSIGN_HASH]: Protocol = Protocol {
        name: "shl_assign",
        hash: 0xdc4702d0307ba27bu64,
        repr: Some("$value <<= b"),
        doc: [
            "Allows the `<<=` operator to apply to values of this type, where the current type is the left-hand side."
        ],
    };

    /// The function to implement for the bitwise shift right operation.
    pub const [SHR, SHR_HASH]: Protocol = Protocol {
        name: "shr",
        hash: 0x6b485e8e6e58fbc8u64,
        repr: Some("let output = $value >> b"),
        doc: [
            "Allows the `>>` operator to apply to values of this type, where the current type is the left-hand side."
        ],
    };

    /// The function to implement for the bitwise shift right assign operation.
    pub const [SHR_ASSIGN, SHR_ASSIGN_HASH]: Protocol = Protocol {
        name: "shr_assign",
        hash: 0x61ff7c46ff00e74au64,
        repr: Some("$value >>= b"),
        doc: [
            "Allows the `>>=` operator to apply to values of this type, where the current type is the left-hand side."
        ],
    };

    /// Protocol function used by template strings.
    pub const [STRING_DISPLAY, STRING_DISPLAY_HASH]: Protocol = Protocol {
        name: "string_display",
        hash: 0x811b62957ea9d9f9u64,
        repr: Some("println(\"{}\", $value)"),
        doc: ["Allows the value to be display printed."],
    };

    /// Protocol function used by custom debug impls.
    pub const [STRING_DEBUG, STRING_DEBUG_HASH]: Protocol = Protocol {
        name: "string_debug",
        hash: 0x4064e3867aaa0717u64,
        repr: Some("println(\"{:?}\", $value)"),
        doc: ["Allows the value to be debug printed."],
    };

    /// Function used to convert an argument into an iterator.
    pub const [INTO_ITER, INTO_ITER_HASH]: Protocol = Protocol {
        name: "into_iter",
        hash: 0x15a85c8d774b4065u64,
        repr: Some("for item in $value { }"),
        doc: ["Allows the value to be converted into an iterator in a for-loop."],
    };

    /// The function to call to continue iteration.
    pub const [NEXT, NEXT_HASH]: Protocol = Protocol {
        name: "next",
        hash: 0xc3cde069de2ba320u64,
        repr: None,
        doc: ["Allows iteration to be advanced for the type, this is used for iterators."],
    };

    /// Function used to convert an argument into a future.
    ///
    /// Signature: `fn(Value) -> Future`.
    pub const [INTO_FUTURE, INTO_FUTURE_HASH]: Protocol = Protocol {
        name: "into_future",
        hash: 0x596e6428deabfda2u64,
        repr: Some("value.await"),
        doc: ["This protocol allows the type to be converted into a future by awaiting them."],
    };

    /// Coerce a value into a type name. This is stored as a constant.
    pub const [INTO_TYPE_NAME, INTO_TYPE_NAME_HASH]: Protocol = Protocol {
        name: "into_type_name",
        hash: 0xbffd08b816c24682u64,
        repr: None,
        doc: [
            "This protocol allows the type to be converted into a string which represents the type name.",
        ],
    };

    /// Function used to test if a value is a specific variant.
    ///
    /// Signature: `fn(self, usize) -> bool`.
    pub const [IS_VARIANT, IS_VARIANT_HASH]: Protocol = Protocol {
        name: "is_variant",
        hash: 0xc030d82bbd4dabe8u64,
        repr: None,
        doc: ["Test if the provided argument is a variant."],
    };

    /// Function used for the question mark operation.
    ///
    /// Signature: `fn(self) -> Result`.
    ///
    /// Note that it uses the `Result` like [`Try`] uses [`ControlFlow`] i.e.,
    /// for `Result::<T, E>` it should return `Result<T, Result<(), E>>`
    ///
    /// [`Try`]: ::core::ops::Try
    /// [`ControlFlow`]: ::core::ops::ControlFlow
    pub const [TRY, TRY_HASH]: Protocol = Protocol {
        name: "try",
        hash: 0x5da1a80787003354u64,
        repr: Some("value?"),
        doc: ["Allows the `?` operator to apply to values of this type."],
    };

    /// Protocol used when calculating a hash.
    pub const [HASH, HASH_HASH]: Protocol = Protocol {
        name: "hash",
        hash: 0xf6cf2d9f416cef08u64,
        repr: Some("let output = hash($value)"),
        doc: ["Hash the given value."],
    };
}
