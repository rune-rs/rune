use crate::Hash;
use std::cmp;
use std::hash;

/// Static type information.
#[derive(Debug)]
pub struct StaticType {
    /// The name of the static type.
    pub name: &'static str,
    /// The hash of the static type.
    pub hash: Hash,
}

impl cmp::PartialEq for &'static StaticType {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl cmp::Eq for &'static StaticType {}

impl hash::Hash for &'static StaticType {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.hash.hash(state)
    }
}

/// The specialized type information for a unit.
pub static UNIT_TYPE: &StaticType = &StaticType {
    name: "unit",
    hash: Hash::new(0x9de148b05752dbb3),
};

/// The specialized type information for a byte type.
pub static BYTE_TYPE: &StaticType = &StaticType {
    name: "byte",
    hash: Hash::new(0x190cacf7c7187189),
};

/// The specialized type information for a bool type.
pub static BOOL_TYPE: &StaticType = &StaticType {
    name: "bool",
    hash: Hash::new(0xbe6bff4422d0c759),
};

/// The specialized type information for a char type.
pub static CHAR_TYPE: &StaticType = &StaticType {
    name: "char",
    hash: Hash::new(0xc56a31d061187c8b),
};

/// The specialized type information for a integer type.
pub static INTEGER_TYPE: &StaticType = &StaticType {
    name: "integer",
    hash: Hash::new(0xbb378867da3981e2),
};

/// The specialized type information for a float type.
pub static FLOAT_TYPE: &StaticType = &StaticType {
    name: "float",
    hash: Hash::new(0x13e40c27462ed8fc),
};

/// The specialized type information for a string type.
pub static STRING_TYPE: &StaticType = &StaticType {
    name: "String",
    hash: Hash::new(0x823ede4114ff8de6),
};

/// The specialized type information for a bytes type.
pub static BYTES_TYPE: &StaticType = &StaticType {
    name: "Bytes",
    hash: Hash::new(0x957fa73126817683),
};

/// The specialized type information for a vector type.
pub static VEC_TYPE: &StaticType = &StaticType {
    name: "Vec",
    hash: Hash::new(0x6c129752545b4223),
};

/// The specialized type information for an anonymous tuple type.
pub static TUPLE_TYPE: &StaticType = &StaticType {
    name: "Tuple",
    hash: Hash::new(0x6da74f62cfa5cc1f),
};

/// The specialized type information for an anonymous object type.
pub static OBJECT_TYPE: &StaticType = &StaticType {
    name: "Object",
    hash: Hash::new(0x65f4e1cf10b1f34c),
};

/// The specialized type information for a future type.
pub static FUTURE_TYPE: &StaticType = &StaticType {
    name: "Future",
    hash: Hash::new(0xafab4a2797436aee),
};

/// The specialized type information for a generator type.
pub static GENERATOR_TYPE: &StaticType = &StaticType {
    name: "Generator",
    hash: Hash::new(0x50deff8c6ef7532c),
};

/// The specialized type information for a generator state type.
pub static GENERATOR_STATE_TYPE: &StaticType = &StaticType {
    name: "GeneratorState",
    hash: Hash::new(0xdd4141d4d8a3ac31),
};

/// The specialized type information for the `Stream` type.
pub static STREAM_TYPE: &StaticType = &StaticType {
    name: "Stream",
    hash: Hash::new(0xd94133730d02c3ea),
};

/// The specialized type information for a result type.
pub static RESULT_TYPE: &StaticType = &StaticType {
    name: "Result",
    hash: Hash::new(0xecec15e1363240ac),
};

/// The specialized type information for a option type.
pub static OPTION_TYPE: &StaticType = &StaticType {
    name: "Option",
    hash: Hash::new(0x5e08dc3f663c72db),
};

/// The specialized type information for a function pointer type.
pub static FN_PTR_TYPE: &StaticType = &StaticType {
    name: "Function",
    hash: Hash::new(0x45b788b02e7f231c),
};
