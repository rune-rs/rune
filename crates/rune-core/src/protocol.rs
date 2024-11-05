use core::cmp;
use core::fmt;
use core::hash::Hasher;

#[cfg(feature = "alloc")]
use crate::alloc;
use crate::hash::{Hash, ToTypeHash};
use crate::item::ItemBuf;

/// A built in instance function.
#[derive(Debug)]
#[non_exhaustive]
pub struct Protocol {
    /// The name of the builtin function.
    pub name: &'static str,
    /// If this protocol defines an associated method, this is the name of that
    /// method.
    pub method: Option<&'static str>,
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

impl ToTypeHash for &Protocol {
    #[inline]
    fn to_type_hash(&self) -> Hash {
        self.hash
    }

    #[inline]
    #[cfg(feature = "alloc")]
    fn to_item(&self) -> alloc::Result<Option<ItemBuf>> {
        Ok(None)
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

impl core::hash::Hash for Protocol {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state)
    }
}

macro_rules! maybe {
    ($tt:tt $($rest:tt)*) => { Some($tt $($rest)*) };
    () => { None };
}

macro_rules! define {
    (
        $(
            $(#[$($meta:meta)*])*
            $vis:vis const $ident:ident: Protocol = Protocol {
                $(method: $method:expr,)?
                hash: $hash:expr,
                $(repr: $repr:expr,)?
                $(#[doc = $doc:expr])*
            };
        )*
    ) => {
        impl Protocol {
            $(
                $(#[$($meta)*])*
                $vis const $ident: Protocol = Protocol {
                    name: stringify!($ident),
                    method: maybe!($($method)*),
                    hash: Hash($hash),
                    #[cfg(feature = "doc")]
                    repr: maybe!($($repr)*),
                    #[cfg(feature = "doc")]
                    doc: &[$($doc),*],
                };
            )*

            /// Look up protocol for the given hash.
            pub fn from_hash(hash: Hash) -> Option<Self> {
                match hash {
                    $(
                        Hash($hash) => {
                            Some(Self::$ident)
                        },
                    )*
                    _ => None,
                }
            }
        }

        #[test]
        fn ensure_unique_hashes() {
            let mut map = ::rust_std::collections::HashMap::<Hash, &'static str>::new();

            $(
                if let Some(ident) = map.insert(Hash($hash), stringify!($ident)) {
                    panic!("Trying to define protocol hash `{}` for `{}`, but it's already defined for {ident}", $hash, stringify!($ident));
                }
            )*
        }
    }
}

define! {
    /// The function to access a field.
    pub const GET: Protocol = Protocol {
        hash: 0x504007af1a8485a4u64,
        repr: "let $out = $value",
        /// Allows a get operation to work.
    };

    /// The function to set a field.
    pub const SET: Protocol = Protocol {
        hash: 0x7d13d47fd8efef5au64,
        repr: "$value = $input",
        /// Allows a set operation to work.
    };

    /// The function to access an index.
    pub const INDEX_GET: Protocol = Protocol {
        hash: 0xadb5b27e2a4d2decu64,
        repr: "let $out = $value[index]",
        /// Allows an indexing get operation to work.
    };

    /// The function to set an index.
    pub const INDEX_SET: Protocol = Protocol {
        hash: 0x162943f7bd03ad36u64,
        repr: "$value[index] = $input",
        /// Allows an indexing set operation to work.
    };

    /// Check two types for partial equality.
    pub const PARTIAL_EQ: Protocol = Protocol {
        method: "eq",
        hash: 0x4b6bc4701445e318u64,
        repr: "if $value == b { }",
        /// Allows for partial equality operations to work.
    };

    /// Check two types for total equality.
    pub const EQ: Protocol = Protocol {
        hash: 0x418f5becbf885806u64,
        repr: "if $value == b { }",
        /// Allows an equality operation to work.
    };

    /// Perform an partial comparison between two values.
    pub const PARTIAL_CMP: Protocol = Protocol {
        method: "partial_cmp",
        hash: 0x8d4430991253343cu64,
        repr: "if $value < b { }",
        /// Allows for partial ordering to work. This is used as the basis for all internal comparisons.
    };

    /// The protocol behind the `>` operator.
    pub const GT: Protocol = Protocol {
        method: "gt",
        hash: 0x29d9486ee6aa98ddu64,
        repr: "if $a > $b { }",
        /// The protocol behind the `>` operator.
    };

    /// The protocol behind the `>=` operator.
    pub const GE: Protocol = Protocol {
        method: "ge",
        hash: 0x2bb35b8f086340bu64,
        repr: "if $a >= $b { }",
        /// The protocol behind the `>=` operator.
    };

    /// The protocol behind the `>` operator.
    pub const LT: Protocol = Protocol {
        method: "lt",
        hash: 0x82cb74423db0a3b6u64,
        repr: "if $a < $b { }",
        /// The protocol behind the `<` operator.
    };

    /// The protocol behind the `<=` operator.
    pub const LE: Protocol = Protocol {
        method: "le",
        hash: 0xcba7d52a7ca8c617u64,
        repr: "if $a <= $b { }",
        /// The protocol behind the `<=` operator.
    };

    pub const MAX: Protocol = Protocol {
        method: "max",
        hash: 0xca63c8386a41c812u64,
        repr: "$a.max($b)",
        /// The implementation protocol for the `PartialOrd::max` method.
    };

    pub const MIN: Protocol = Protocol {
        method: "min",
        hash: 0x454f2aabc9d16509u64,
        repr: "$a.min($b)",
        /// The implementation protocol for the `PartialOrd::min` method.
    };

    /// Perform an total comparison between two values.
    pub const CMP: Protocol = Protocol {
        method: "cmp",
        hash: 0x240f1b75466cd1a3u64,
        repr: "if $value < b { }",
        /// Allows for total ordering to work.
    };

    /// The function to implement for the addition operation.
    pub const ADD: Protocol = Protocol {
        hash: 0xe4ecf51fa0bf1076u64,
        repr: "let $out = $value + $b",
        /// Allows the `+` operator to apply to values of this type, where the current type is the left-hand side.
    };

    /// The function to implement for the addition assign operation.
    pub const ADD_ASSIGN: Protocol = Protocol {
        hash: 0x42451ccb0a2071a9u64,
        repr: "$value += $b",
        /// Allows the `+=` operator to apply to values of this type, where the current type is the left-hand side.
    };

    /// The function to implement for the subtraction operation.
    pub const SUB: Protocol = Protocol {
        hash: 0x6fa86a5f18d0bf71u64,
        repr: "let $out = $value - $b",
        /// Allows the `-` operator to apply to values of this type, where the current type is the left-hand side.
    };

    /// The function to implement for the subtraction assign operation.
    pub const SUB_ASSIGN: Protocol = Protocol {
        hash: 0x5939bb56a1415284u64,
        repr: "$value -= $b",
        /// Allows the `-=` operator to apply to values of this type, where the current type is the left-hand side.
    };

    /// The function to implement for the multiply operation.
    pub const MUL: Protocol = Protocol {
        hash: 0xb09e99dc94091d1cu64,
        repr: "let $out = $value * $b",
        /// Allows the `*` operator to apply to values of this type, where the current type is the left-hand side.
    };

    /// The function to implement for the multiply assign operation.
    pub const MUL_ASSIGN: Protocol = Protocol {
        hash: 0x29a54b727f980ebfu64,
        repr: "$value *= $b",
        /// Allows the `*=` operator to apply to values of this type, where the current type is the left-hand side.
    };

    /// The function to implement for the division operation.
    pub const DIV: Protocol = Protocol {
        hash: 0xf26d6eea1afca6e8u64,
        repr: "let $out = $value / $b",
        /// Allows the `/` operator to apply to values of this type, where the current type is the left-hand side.
    };

    /// The function to implement for the division assign operation.
    pub const DIV_ASSIGN: Protocol = Protocol {
        hash: 0x4dd087a8281c04e6u64,
        repr: "$value /= $b",
        /// Allows the `/=` operator to apply to values of this type, where the current type is the left-hand side.
    };

    /// The function to implement for the remainder operation.
    pub const REM: Protocol = Protocol {
        hash: 0x5c6293639c74e671u64,
        repr: "let $out = $value % $b",
        /// Allows the `%` operator to apply to values of this type, where the current type is the left-hand side.
    };

    /// The function to implement for the remainder assign operation.
    pub const REM_ASSIGN: Protocol = Protocol {
        hash: 0x3a8695980e77baf4u64,
        repr: "$value %= $b",
        /// Allows the `%=` operator to apply to values of this type, where the current type is the left-hand side.
    };

    /// The function to implement for the bitwise and operation.
    pub const BIT_AND: Protocol = Protocol {
        hash: 0x0e11f20d940eebe8u64,
        repr: "let $out = $value & $b",
        /// Allows the `&` operator to apply to values of this type, where the current type is the left-hand side.
    };

    /// The function to implement for the bitwise and assign operation.
    pub const BIT_AND_ASSIGN: Protocol = Protocol {
        hash: 0x95cb1ba235dfb5ecu64,
        repr: "$value &= $b",
        /// Allows the `&=` operator to apply to values of this type, where the current type is the left-hand side.
    };

    /// The function to implement for the bitwise xor operation.
    pub const BIT_XOR: Protocol = Protocol {
        hash: 0xa3099c54e1de4cbfu64,
        repr: "let $out = $value ^ $b",
        /// Allows the `^` operator to apply to values of this type, where the current type is the left-hand side.
    };

    /// The function to implement for the bitwise xor assign operation.
    pub const BIT_XOR_ASSIGN: Protocol = Protocol {
        hash: 0x01fa9706738f9867u64,
        repr: "$value ^= $b",
        /// Allows the `^=` operator to apply to values of this type, where the current type is the left-hand side.
    };

    /// The function to implement for the bitwise or operation.
    pub const BIT_OR: Protocol = Protocol {
        hash: 0x05010afceb4a03d0u64,
        repr: "let $out = $value | $b",
        /// Allows the `|` operator to apply to values of this type, where the current type is the left-hand side.
    };

    /// The function to implement for the bitwise xor assign operation.
    pub const BIT_OR_ASSIGN: Protocol = Protocol {
        hash: 0x606d79ff1750a7ecu64,
        repr: "$value |= $b",
        /// Allows the `|=` operator to apply to values of this type, where the current type is the left-hand side.
    };

    /// The function to implement for the bitwise shift left operation.
    pub const SHL: Protocol = Protocol {
        hash: 0x6845f7d0cc9e002du64,
        repr: "let $out = $value << $b",
        /// Allows the `<<` operator to apply to values of this type, where the current type is the left-hand side.
    };

    /// The function to implement for the bitwise shift left assign operation.
    pub const SHL_ASSIGN: Protocol = Protocol {
        hash: 0xdc4702d0307ba27bu64,
        repr: "$value <<= $b",
        /// Allows the `<<=` operator to apply to values of this type, where the current type is the left-hand side.
    };

    /// The function to implement for the bitwise shift right operation.
    pub const SHR: Protocol = Protocol {
        hash: 0x6b485e8e6e58fbc8u64,
        repr: "let $out = $value >> $b",
        /// Allows the `>>` operator to apply to values of this type, where the current type is the left-hand side.
    };

    /// The function to implement for the bitwise shift right assign operation.
    pub const SHR_ASSIGN: Protocol = Protocol {
        hash: 0x61ff7c46ff00e74au64,
        repr: "$value >>= $b",
        /// Allows the `>>=` operator to apply to values of this type, where the current type is the left-hand side.
    };

    /// Protocol function used by template strings.
    pub const DISPLAY_FMT: Protocol = Protocol {
        hash: 0x811b62957ea9d9f9u64,
        repr: "format!(\"{}\", $value)",
        /// Allows the value to be display printed.
    };

    /// Protocol function used by custom debug impls.
    pub const DEBUG_FMT: Protocol = Protocol {
        hash: 0x4064e3867aaa0717u64,
        repr: "format!(\"{:?}\", $value)",
        /// Allows the value to be debug printed.
    };

    /// Function used to convert an argument into an iterator.
    pub const INTO_ITER: Protocol = Protocol {
        hash: 0x15a85c8d774b4065u64,
        repr: "for item in $value { }",
        /// Allows the value to be converted into an iterator in a for-loop.
    };

    /// The function to call to continue iteration.
    pub const NEXT: Protocol = Protocol {
        method: "next",
        hash: 0xc3cde069de2ba320u64,
        repr: "let $out = $value.next()",
        /// Allows iteration to be advanced for the type, this is used for iterators.
    };

    /// The function to call to continue iteration at the nth element.
    pub const NTH: Protocol = Protocol {
        method: "nth",
        hash: 0x6704550736c82a58u64,
        repr: "let $out = $value.nth(index)",
        /// Allows iteration to be advanced for the type to the nth element, this is used for iterators.
    };

    /// The function to call to continue iteration at the nth element form the back.
    pub const NTH_BACK: Protocol = Protocol {
        method: "nth_back",
        hash: 0x4885ca2fd53a08c8u64,
        /// Allows iteration to be advanced for the type to the nth element from the back, this is used for iterators.
    };

    /// Protocol used when getting the size hint of an iterator.
    pub const SIZE_HINT: Protocol = Protocol {
        method: "size_hint",
        hash: 0x1a7b50baabc6e094u64,
        repr: "let $out = $value.size_hint()",
        /// Get the size hint of an iterator.
    };

    /// Protocol used when getting the exact length of an iterator.
    pub const LEN: Protocol = Protocol {
        method: "len",
        hash: 0x52dd3b9489d39c42u64,
        repr: "let $out = $value.len()",
        /// Get the length of an iterator.
    };

    /// Protocol used when cloning a value.
    pub const NEXT_BACK: Protocol = Protocol {
        method: "next_back",
        hash: 0x91149fef42c0a8aeu64,
        repr: "let $out = $value.next_back()",
        /// Get the next value from the back of the iterator.
    };

    /// Function used to convert an argument into a future.
    ///
    /// Signature: `fn(Value) -> Future`.
    pub const INTO_FUTURE: Protocol = Protocol {
        hash: 0x596e6428deabfda2u64,
        repr: "$value.await",
        /// This protocol allows the type to be converted into a future by awaiting them.
    };

    /// Coerce a value into a type name. This is stored as a constant.
    pub const INTO_TYPE_NAME: Protocol = Protocol {
        hash: 0xbffd08b816c24682u64,
        /// This protocol allows the type to be converted into a string which represents the type name."
    };

    /// Function used to test if a value is a specific variant.
    ///
    /// Signature: `fn(self, usize) -> bool`.
    pub const IS_VARIANT: Protocol = Protocol {
        hash: 0xc030d82bbd4dabe8u64,
        /// Test if the provided argument is a variant.
    };

    /// Function used for the question mark operation.
    ///
    /// Signature: `fn(self) -> Result`.
    ///
    /// Note that it uses the `Result` like [`Try`] uses [`ControlFlow`] i.e.,
    /// for `Result::<T, E>` it should return `Result<T, Result<(), E>>`
    ///
    /// [`Try`]: core::ops::Try
    /// [`ControlFlow`]: core::ops::ControlFlow
    pub const TRY: Protocol = Protocol {
        hash: 0x5da1a80787003354u64,
        repr: "value?",
        /// Allows the `?` operator to apply to values of this type.
    };

    /// Protocol used when calculating a hash.
    pub const HASH: Protocol = Protocol {
        hash: 0xf6cf2d9f416cef08u64,
        repr: "let $out = hash($value)",
        /// Hash a value.
    };

    /// Protocol used when cloning a value.
    pub const CLONE: Protocol = Protocol {
        method: "clone",
        hash: 0x2af2c875e36971eu64,
        repr: "let $out = clone($value)",
        /// Clone a value.
    };
}
