use core::cmp;
use core::fmt;
use core::hash::{self, Hash as _};
use core::ops;

use crate::compile::ItemBuf;
use crate::hash::IntoHash;
use crate::module::{AssociatedFunctionKind, AssociatedFunctionName, ToInstance};
use crate::{Hash, ToTypeHash};

/// A built in instance function.
#[derive(Debug, Clone, Copy)]
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
    pub(crate) repr: Option<&'static str>,
    /// Documentation for protocol.
    #[cfg(feature = "doc")]
    pub(crate) doc: &'static [&'static str],
}

impl IntoHash for Protocol {
    #[inline]
    fn into_hash(self) -> Hash {
        self.hash
    }
}

impl ToInstance for Protocol {
    #[inline]
    fn to_instance(self) -> AssociatedFunctionName {
        AssociatedFunctionName {
            kind: AssociatedFunctionKind::Protocol(self),
            parameters: Hash::EMPTY,
            #[cfg(feature = "doc")]
            parameter_types: vec![],
        }
    }
}

impl ToTypeHash for Protocol {
    #[inline]
    fn to_type_hash(&self) -> Hash {
        self.hash
    }

    #[inline]
    fn to_item(&self) -> Option<ItemBuf> {
        None
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
            $vis:vis const $ident:ident: Protocol = Protocol {
                name: $name:expr,
                hash: $hash:expr,
                repr: $repr:expr,
                doc: $doc:expr $(,)?
            };
        )*
    ) => {
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
        )*
    }
}

impl Protocol {
    define! {
        /// The function to access a field.
        pub const GET: Protocol = Protocol {
            name: "get",
            hash: 0x504007af1a8485a4,
            repr: Some("let output = $value"),
            doc: ["Allows a get operation to work."],
        };

        /// The function to set a field.
        pub const SET: Protocol = Protocol {
            name: "set",
            hash: 0x7d13d47fd8efef5a,
            repr: Some("$value = input"),
            doc: ["Allows a set operation to work."],
        };

        /// The function to access an index.
        pub const INDEX_GET: Protocol = Protocol {
            name: "index_get",
            hash: 0xadb5b27e2a4d2dec,
            repr: Some("let output = $value[index]"),
            doc: ["Allows an indexing get operation to work."],
        };

        /// The function to set an index.
        pub const INDEX_SET: Protocol = Protocol {
            name: "index_set",
            hash: 0x162943f7bd03ad36,
            repr: Some("$value[index] = input"),
            doc: ["Allows an indexing set operation to work."],
        };

        /// Check two types for equality.
        pub const EQ: Protocol = Protocol {
            name: "eq",
            hash: 0x418f5becbf885806,
            repr: Some("if $value == b { }"),
            doc: ["Allows an equality operation to work."],
        };

        /// The function to implement for the addition operation.
        pub const ADD: Protocol = Protocol {
            name: "add",
            hash: 0xe4ecf51fa0bf1076,
            repr: Some("let output = $value + b"),
            doc: [
                "Allows the `+` operator to apply to values of this type, where the current type is the left-hand side."
            ],
        };

        /// The function to implement for the addition assign operation.
        pub const ADD_ASSIGN: Protocol = Protocol {
            name: "add_assign",
            hash: 0x42451ccb0a2071a9,
            repr: Some("$value += b"),
            doc: [
                "Allows the `+=` operator to apply to values of this type, where the current type is the left-hand side."
            ],
        };

        /// The function to implement for the subtraction operation.
        pub const SUB: Protocol = Protocol {
            name: "sub",
            hash: 0x6fa86a5f18d0bf71,
            repr: Some("let output = $value - b"),
            doc: [
                "Allows the `-` operator to apply to values of this type, where the current type is the left-hand side."
            ],
        };

        /// The function to implement for the subtraction assign operation.
        pub const SUB_ASSIGN: Protocol = Protocol {
            name: "sub_assign",
            hash: 0x5939bb56a1415284,
            repr: Some("$value -= b"),
            doc: [
                "Allows the `-=` operator to apply to values of this type, where the current type is the left-hand side."
            ],
        };

        /// The function to implement for the multiply operation.
        pub const MUL: Protocol = Protocol {
            name: "mul",
            hash: 0xb09e99dc94091d1c,
            repr: Some("let output = $value * b"),
            doc: [
                "Allows the `*` operator to apply to values of this type, where the current type is the left-hand side."
            ],
        };

        /// The function to implement for the multiply assign operation.
        pub const MUL_ASSIGN: Protocol = Protocol {
            name: "mul_assign",
            hash: 0x29a54b727f980ebf,
            repr: Some("$value *= b"),
            doc: [
                "Allows the `*=` operator to apply to values of this type, where the current type is the left-hand side."
            ],
        };

        /// The function to implement for the division operation.
        pub const DIV: Protocol = Protocol {
            name: "div",
            hash: 0xf26d6eea1afca6e8,
            repr: Some("let output = $value / b"),
            doc: [
                "Allows the `/` operator to apply to values of this type, where the current type is the left-hand side."
            ],
        };

        /// The function to implement for the division assign operation.
        pub const DIV_ASSIGN: Protocol = Protocol {
            name: "div_assign",
            hash: 0x4dd087a8281c04e6,
            repr: Some("$value /= b"),
            doc: [
                "Allows the `/=` operator to apply to values of this type, where the current type is the left-hand side."
            ],
        };

        /// The function to implement for the remainder operation.
        pub const REM: Protocol = Protocol {
            name: "rem",
            hash: 0x5c6293639c74e671,
            repr: Some("let output = $value % b"),
            doc: [
                "Allows the `%` operator to apply to values of this type, where the current type is the left-hand side."
            ],
        };

        /// The function to implement for the remainder assign operation.
        pub const REM_ASSIGN: Protocol = Protocol {
            name: "rem_assign",
            hash: 0x3a8695980e77baf4,
            repr: Some("$value %= b"),
            doc: [
                "Allows the `%=` operator to apply to values of this type, where the current type is the left-hand side."
            ],
        };

        /// The function to implement for the bitwise and operation.
        pub const BIT_AND: Protocol = Protocol {
            name: "bit_and",
            hash: 0x0e11f20d940eebe8,
            repr: Some("let output = $value & b"),
            doc: [
                "Allows the `&` operator to apply to values of this type, where the current type is the left-hand side."
            ],
        };

        /// The function to implement for the bitwise and assign operation.
        pub const BIT_AND_ASSIGN: Protocol = Protocol {
            name: "bit_and_assign",
            hash: 0x95cb1ba235dfb5ec,
            repr: Some("$value &= b"),
            doc: [
                "Allows the `&=` operator to apply to values of this type, where the current type is the left-hand side."
            ],
        };

        /// The function to implement for the bitwise xor operation.
        pub const BIT_XOR: Protocol = Protocol {
            name: "bit_xor",
            hash: 0xa3099c54e1de4cbf,
            repr: Some("let output = $value ^ b"),
            doc: [
                "Allows the `^` operator to apply to values of this type, where the current type is the left-hand side."
            ],
        };

        /// The function to implement for the bitwise xor assign operation.
        pub const BIT_XOR_ASSIGN: Protocol = Protocol {
            name: "bit_xor_assign",
            hash: 0x01fa9706738f9867,
            repr: Some("$value ^= b"),
            doc: [
                "Allows the `^=` operator to apply to values of this type, where the current type is the left-hand side."
            ],
        };

        /// The function to implement for the bitwise or operation.
        pub const BIT_OR: Protocol = Protocol {
            name: "bit_or",
            hash: 0x05010afceb4a03d0,
            repr: Some("let output = $value | b"),
            doc: [
                "Allows the `|` operator to apply to values of this type, where the current type is the left-hand side."
            ],
        };

        /// The function to implement for the bitwise xor assign operation.
        pub const BIT_OR_ASSIGN: Protocol = Protocol {
            name: "bit_or_assign",
            hash: 0x606d79ff1750a7ec,
            repr: Some("$value |= b"),
            doc: [
                "Allows the `|=` operator to apply to values of this type, where the current type is the left-hand side."
            ],
        };

        /// The function to implement for the bitwise shift left operation.
        pub const SHL: Protocol = Protocol {
            name: "shl",
            hash: 0x6845f7d0cc9e002d,
            repr: Some("let output = $value << b"),
            doc: [
                "Allows the `<<` operator to apply to values of this type, where the current type is the left-hand side."
            ],
        };

        /// The function to implement for the bitwise shift left assign operation.
        pub const SHL_ASSIGN: Protocol = Protocol {
            name: "shl_assign",
            hash: 0xdc4702d0307ba27b,
            repr: Some("$value <<= b"),
            doc: [
                "Allows the `<<=` operator to apply to values of this type, where the current type is the left-hand side."
            ],
        };

        /// The function to implement for the bitwise shift right operation.
        pub const SHR: Protocol = Protocol {
            name: "shr",
            hash: 0x6b485e8e6e58fbc8,
            repr: Some("let output = $value >> b"),
            doc: [
                "Allows the `>>` operator to apply to values of this type, where the current type is the left-hand side."
            ],
        };

        /// The function to implement for the bitwise shift right assign operation.
        pub const SHR_ASSIGN: Protocol = Protocol {
            name: "shr_assign",
            hash: 0x61ff7c46ff00e74a,
            repr: Some("$value >>= b"),
            doc: [
                "Allows the `>>=` operator to apply to values of this type, where the current type is the left-hand side."
            ],
        };

        /// Protocol function used by template strings.
        pub const STRING_DISPLAY: Protocol = Protocol {
            name: "string_display",
            hash: 0x811b62957ea9d9f9,
            repr: Some("println(\"{}\", $value)"),
            doc: ["Allows the value to be display printed."],
        };

        /// Protocol function used by custom debug impls.
        pub const STRING_DEBUG: Protocol = Protocol {
            name: "string_debug",
            hash: 0x4064e3867aaa0717,
            repr: Some("println(\"{:?}\", $value)"),
            doc: ["Allows the value to be debug printed."],
        };

        /// Function used to convert an argument into an iterator.
        pub const INTO_ITER: Protocol = Protocol {
            name: "into_iter",
            hash: 0x15a85c8d774b4065,
            repr: Some("for item in $value { }"),
            doc: ["Allows the value to be converted into an iterator in a for-loop."],
        };

        /// The function to call to continue iteration.
        pub const NEXT: Protocol = Protocol {
            name: "next",
            hash: 0xc3cde069de2ba320,
            repr: None,
            doc: ["Allows iteration to be advanced for the type, this is used for iterators."],
        };

        /// Function used to convert an argument into a future.
        ///
        /// Signature: `fn(Value) -> Future`.
        pub const INTO_FUTURE: Protocol = Protocol {
            name: "into_future",
            hash: 0x596e6428deabfda2,
            repr: Some("value.await"),
            doc: ["This protocol allows the type to be converted into a future by awaiting them."],
        };

        /// Coerce a value into a type name. This is stored as a constant.
        pub const INTO_TYPE_NAME: Protocol = Protocol {
            name: "into_type_name",
            hash: 0xbffd08b816c24682,
            repr: None,
            doc: [
                "This protocol allows the type to be converted into a string which represents the type name.",
            ],
        };

        /// Function used to test if a value is a specific variant.
        ///
        /// Signature: `fn(self, usize) -> bool`.
        pub const IS_VARIANT: Protocol = Protocol {
            name: "is_variant",
            hash: 0xc030d82bbd4dabe8,
            repr: None,
            doc: ["Test if the provided argument is a variant."],
        };
    }
}
