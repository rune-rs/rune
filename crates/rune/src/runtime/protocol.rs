use std::cmp;
use std::fmt;
use std::hash;
use std::hash::Hash as _;

use crate::compile::{AssociatedFunctionKind, AssociatedFunctionName, ItemBuf, ToInstance};
use crate::hash::IntoHash;
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

impl std::ops::Deref for Protocol {
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

impl Protocol {
    /// The function to access a field.
    pub const GET: Protocol = Protocol {
        name: "get",
        hash: Hash::new(0x504007af1a8485a4),
        #[cfg(feature = "doc")]
        repr: Some("let value = a.field"),
        #[cfg(feature = "doc")]
        doc: &["Allows a get operation to work."],
    };

    /// The function to set a field.
    pub const SET: Protocol = Protocol {
        name: "set",
        hash: Hash::new(0x7d13d47fd8efef5a),
        #[cfg(feature = "doc")]
        repr: Some("a.field = b"),
        #[cfg(feature = "doc")]
        doc: &["Allows a set operation to work."],
    };

    /// The function to access an index.
    pub const INDEX_GET: Protocol = Protocol {
        name: "index_get",
        hash: Hash::new(0xadb5b27e2a4d2dec),
        #[cfg(feature = "doc")]
        repr: Some("let value = a[index]"),
        #[cfg(feature = "doc")]
        doc: &["Allows an indexing get operation to work."],
    };

    /// The function to set an index.
    pub const INDEX_SET: Protocol = Protocol {
        name: "index_set",
        hash: Hash::new(0x162943f7bd03ad36),
        #[cfg(feature = "doc")]
        repr: Some("a[index] = b"),
        #[cfg(feature = "doc")]
        doc: &["Allows an indexing set operation to work."],
    };

    /// Check two types for equality.
    pub const EQ: Protocol = Protocol {
        name: "eq",
        hash: Hash::new(0x418f5becbf885806),
        #[cfg(feature = "doc")]
        repr: Some("if a == b { }"),
        #[cfg(feature = "doc")]
        doc: &["Allows an equality operation to work."],
    };

    /// The function to implement for the addition operation.
    pub const ADD: Protocol = Protocol {
        name: "add",
        hash: Hash::new(0xe4ecf51fa0bf1076),
        #[cfg(feature = "doc")]
        repr: Some("let value = a + b"),
        #[cfg(feature = "doc")]
        doc: &[
            "Allows the `+` operator to apply to values of this type, where the current type is the left-hand side."
        ]
    };

    /// The function to implement for the addition assign operation.
    pub const ADD_ASSIGN: Protocol = Protocol {
        name: "add_assign",
        hash: Hash::new(0x42451ccb0a2071a9),
        #[cfg(feature = "doc")]
        repr: Some("a += b"),
        #[cfg(feature = "doc")]
        doc: &[
            "Allows the `+=` operator to apply to values of this type, where the current type is the left-hand side."
        ]
    };

    /// The function to implement for the subtraction operation.
    pub const SUB: Protocol = Protocol {
        name: "sub",
        hash: Hash::new(0x6fa86a5f18d0bf71),
        #[cfg(feature = "doc")]
        repr: Some("let value = a - b"),
        #[cfg(feature = "doc")]
        doc: &[
            "Allows the `-` operator to apply to values of this type, where the current type is the left-hand side."
        ]
    };

    /// The function to implement for the subtraction assign operation.
    pub const SUB_ASSIGN: Protocol = Protocol {
        name: "sub_assign",
        hash: Hash::new(0x5939bb56a1415284),
        #[cfg(feature = "doc")]
        repr: Some("a -= b"),
        #[cfg(feature = "doc")]
        doc: &[
            "Allows the `-=` operator to apply to values of this type, where the current type is the left-hand side."
        ]
    };

    /// The function to implement for the multiply operation.
    pub const MUL: Protocol = Protocol {
        name: "mul",
        hash: Hash::new(0xb09e99dc94091d1c),
        #[cfg(feature = "doc")]
        repr: Some("let value = a * b"),
        #[cfg(feature = "doc")]
        doc: &[
            "Allows the `*` operator to apply to values of this type, where the current type is the left-hand side."
        ]
    };

    /// The function to implement for the multiply assign operation.
    pub const MUL_ASSIGN: Protocol = Protocol {
        name: "mul_assign",
        hash: Hash::new(0x29a54b727f980ebf),
        #[cfg(feature = "doc")]
        repr: Some("a *= b"),
        #[cfg(feature = "doc")]
        doc: &[
            "Allows the `*=` operator to apply to values of this type, where the current type is the left-hand side."
        ]
    };

    /// The function to implement for the division operation.
    pub const DIV: Protocol = Protocol {
        name: "div",
        hash: Hash::new(0xf26d6eea1afca6e8),
        #[cfg(feature = "doc")]
        repr: Some("let value = a / b"),
        #[cfg(feature = "doc")]
        doc: &[
            "Allows the `/` operator to apply to values of this type, where the current type is the left-hand side."
        ]
    };

    /// The function to implement for the division assign operation.
    pub const DIV_ASSIGN: Protocol = Protocol {
        name: "div_assign",
        hash: Hash::new(0x4dd087a8281c04e6),
        #[cfg(feature = "doc")]
        repr: Some("a /= b"),
        #[cfg(feature = "doc")]
        doc: &[
            "Allows the `/=` operator to apply to values of this type, where the current type is the left-hand side."
        ]
    };

    /// The function to implement for the remainder operation.
    pub const REM: Protocol = Protocol {
        name: "rem",
        hash: Hash::new(0x5c6293639c74e671),
        #[cfg(feature = "doc")]
        repr: Some("let value = a % b"),
        #[cfg(feature = "doc")]
        doc: &[
            "Allows the `%` operator to apply to values of this type, where the current type is the left-hand side."
        ]
    };

    /// The function to implement for the remainder assign operation.
    pub const REM_ASSIGN: Protocol = Protocol {
        name: "rem_assign",
        hash: Hash::new(0x3a8695980e77baf4),
        #[cfg(feature = "doc")]
        repr: Some("a %= b"),
        #[cfg(feature = "doc")]
        doc: &[
            "Allows the `%=` operator to apply to values of this type, where the current type is the left-hand side."
        ]
    };

    /// The function to implement for the bitwise and operation.
    pub const BIT_AND: Protocol = Protocol {
        name: "bit_and",
        hash: Hash::new(0x0e11f20d940eebe8),
        #[cfg(feature = "doc")]
        repr: Some("let value = a & b"),
        #[cfg(feature = "doc")]
        doc: &[
            "Allows the `&` operator to apply to values of this type, where the current type is the left-hand side."
        ]
    };

    /// The function to implement for the bitwise and assign operation.
    pub const BIT_AND_ASSIGN: Protocol = Protocol {
        name: "bit_and_assign",
        hash: Hash::new(0x95cb1ba235dfb5ec),
        #[cfg(feature = "doc")]
        repr: Some("a &= b"),
        #[cfg(feature = "doc")]
        doc: &[
            "Allows the `&=` operator to apply to values of this type, where the current type is the left-hand side."
        ]
    };

    /// The function to implement for the bitwise xor operation.
    pub const BIT_XOR: Protocol = Protocol {
        name: "bit_xor",
        hash: Hash::new(0xa3099c54e1de4cbf),
        #[cfg(feature = "doc")]
        repr: Some("let value = a ^ b"),
        #[cfg(feature = "doc")]
        doc: &[
            "Allows the `^` operator to apply to values of this type, where the current type is the left-hand side."
        ]
    };

    /// The function to implement for the bitwise xor assign operation.
    pub const BIT_XOR_ASSIGN: Protocol = Protocol {
        name: "bit_xor_assign",
        hash: Hash::new(0x01fa9706738f9867),
        #[cfg(feature = "doc")]
        repr: Some("a ^= b"),
        #[cfg(feature = "doc")]
        doc: &[
            "Allows the `^=` operator to apply to values of this type, where the current type is the left-hand side."
        ]
    };

    /// The function to implement for the bitwise or operation.
    pub const BIT_OR: Protocol = Protocol {
        name: "bit_or",
        hash: Hash::new(0x05010afceb4a03d0),
        #[cfg(feature = "doc")]
        repr: Some("let value = a | b"),
        #[cfg(feature = "doc")]
        doc: &[
            "Allows the `|` operator to apply to values of this type, where the current type is the left-hand side."
        ]
    };

    /// The function to implement for the bitwise xor assign operation.
    pub const BIT_OR_ASSIGN: Protocol = Protocol {
        name: "bit_or_assign",
        hash: Hash::new(0x606d79ff1750a7ec),
        #[cfg(feature = "doc")]
        repr: Some("a |= b"),
        #[cfg(feature = "doc")]
        doc: &[
            "Allows the `|=` operator to apply to values of this type, where the current type is the left-hand side."
        ]
    };

    /// The function to implement for the bitwise shift left operation.
    pub const SHL: Protocol = Protocol {
        name: "shl",
        hash: Hash::new(0x6845f7d0cc9e002d),
        #[cfg(feature = "doc")]
        repr: Some("let value = a << b"),
        #[cfg(feature = "doc")]
        doc: &[
            "Allows the `<<` operator to apply to values of this type, where the current type is the left-hand side."
        ]
    };

    /// The function to implement for the bitwise shift left assign operation.
    pub const SHL_ASSIGN: Protocol = Protocol {
        name: "shl_assign",
        hash: Hash::new(0xdc4702d0307ba27b),
        #[cfg(feature = "doc")]
        repr: Some("a <<= b"),
        #[cfg(feature = "doc")]
        doc: &[
            "Allows the `<<=` operator to apply to values of this type, where the current type is the left-hand side."
        ]
    };

    /// The function to implement for the bitwise shift right operation.
    pub const SHR: Protocol = Protocol {
        name: "shr",
        hash: Hash::new(0x6b485e8e6e58fbc8),
        #[cfg(feature = "doc")]
        repr: Some("let value = a >> b"),
        #[cfg(feature = "doc")]
        doc: &[
            "Allows the `>>` operator to apply to values of this type, where the current type is the left-hand side."
        ]
    };

    /// The function to implement for the bitwise shift right assign operation.
    pub const SHR_ASSIGN: Protocol = Protocol {
        name: "shr_assign",
        hash: Hash::new(0x61ff7c46ff00e74a),
        #[cfg(feature = "doc")]
        repr: Some("a >>= b"),
        #[cfg(feature = "doc")]
        doc: &[
            "Allows the `>>=` operator to apply to values of this type, where the current type is the left-hand side."
        ]
    };

    /// Protocol function used by template strings.
    pub const STRING_DISPLAY: Protocol = Protocol {
        name: "string_display",
        hash: Hash::new(0x811b62957ea9d9f9),
        #[cfg(feature = "doc")]
        repr: Some("println(\"{}\", value)"),
        #[cfg(feature = "doc")]
        doc: &["Allows the value to be display printed."],
    };

    /// Protocol function used by custom debug impls.
    pub const STRING_DEBUG: Protocol = Protocol {
        name: "string_debug",
        hash: Hash::new(0x4064e3867aaa0717),
        #[cfg(feature = "doc")]
        repr: Some("println(\"{:?}\", value)"),
        #[cfg(feature = "doc")]
        doc: &["Allows the value to be debug printed."],
    };

    /// Function used to convert an argument into an iterator.
    pub const INTO_ITER: Protocol = Protocol {
        name: "into_iter",
        hash: Hash::new(0x15a85c8d774b4065),
        #[cfg(feature = "doc")]
        repr: Some("for item in value { }"),
        #[cfg(feature = "doc")]
        doc: &["Allows the value to be converted into an iterator in a for-loop."],
    };

    /// The function to call to continue iteration.
    pub const NEXT: Protocol = Protocol {
        name: "next",
        hash: Hash::new(0xc3cde069de2ba320),
        #[cfg(feature = "doc")]
        repr: None,
        #[cfg(feature = "doc")]
        doc: &["Allows iteration to be advanced for the type, this is used for iterators."],
    };

    /// Function used to convert an argument into a future.
    ///
    /// Signature: `fn(Value) -> Future`.
    pub const INTO_FUTURE: Protocol = Protocol {
        name: "into_future",
        hash: Hash::new(0x596e6428deabfda2),
        #[cfg(feature = "doc")]
        repr: Some("value.await"),
        #[cfg(feature = "doc")]
        doc: &["This protocol allows the type to be converted into a future by awaiting them."],
    };

    /// Coerce a value into a type name. This is stored as a constant.
    pub const INTO_TYPE_NAME: Protocol = Protocol {
        name: "into_type_name",
        hash: Hash::new(0xbffd08b816c24682),
        #[cfg(feature = "doc")]
        repr: None,
        #[cfg(feature = "doc")]
        doc: &[
            "This protocol allows the type to be converted into a string which represents the type name.",
        ]
    };

    /// Function used to test if a value is a specific variant.
    ///
    /// Signature: `fn(self, usize) -> bool`.
    pub const IS_VARIANT: Protocol = Protocol {
        name: "is_variant",
        hash: Hash::new(0xc030d82bbd4dabe8),
        #[cfg(feature = "doc")]
        repr: None,
        #[cfg(feature = "doc")]
        doc: &["Test if the provided argument is a variant."],
    };
}
