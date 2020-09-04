use crate::{Hash, IntoHash, IntoInstFnHash};
use std::fmt;

/// A built in instance function.
#[derive(Debug, Clone, Copy)]
pub struct Protocol {
    /// The name of the builtin function.
    pub name: &'static str,
    /// The hash of the builtin function.
    pub hash: Hash,
}

impl IntoInstFnHash for Protocol {
    fn to_hash(self) -> Hash {
        self.hash
    }

    fn to_name(self) -> String {
        String::from(self.name)
    }
}

impl IntoHash for Protocol {
    fn into_hash(self) -> Hash {
        self.hash
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

/// The function to access an index.
pub const INDEX_GET: Protocol = Protocol {
    name: "index_get",
    hash: Hash::new(0xadb5b27e2a4d2dec),
};

/// The function to set an index.
pub const INDEX_SET: Protocol = Protocol {
    name: "index_set",
    hash: Hash::new(0x162943f7bd03ad36),
};

/// The function to implement for the addition operation.
pub const ADD: Protocol = Protocol {
    name: "add",
    hash: Hash::new(0xe4ecf51fa0bf1076),
};

/// The function to implement for the addition assign operation.
pub const ADD_ASSIGN: Protocol = Protocol {
    name: "add_assign",
    hash: Hash::new(0x42451ccb0a2071a9),
};

/// The function to implement for the subtraction operation.
pub const SUB: Protocol = Protocol {
    name: "sub",
    hash: Hash::new(0x6fa86a5f18d0bf71),
};

/// The function to implement for the subtraction assign operation.
pub const SUB_ASSIGN: Protocol = Protocol {
    name: "sub_assign",
    hash: Hash::new(0x5939bb56a1415284),
};

/// The function to implement for the multiply operation.
pub const MUL: Protocol = Protocol {
    name: "mul",
    hash: Hash::new(0xb09e99dc94091d1c),
};

/// The function to implement for the multiply assign operation.
pub const MUL_ASSIGN: Protocol = Protocol {
    name: "mul_assign",
    hash: Hash::new(0x29a54b727f980ebf),
};

/// The function to implement for the division operation.
pub const DIV: Protocol = Protocol {
    name: "div",
    hash: Hash::new(0xf26d6eea1afca6e8),
};

/// The function to implement for the division assign operation.
pub const DIV_ASSIGN: Protocol = Protocol {
    name: "div_assign",
    hash: Hash::new(0x4dd087a8281c04e6),
};

/// The function to implement for the modulo operation.
pub const REM: Protocol = Protocol {
    name: "mod",
    hash: Hash::new(0x5c6293639c74e671),
};

/// Protocol function used by template strings.
pub const STRING_DISPLAY: Protocol = Protocol {
    name: "string_display",
    hash: Hash::new(0x811b62957ea9d9f9),
};

/// Function used to convert an argument into an iterator.
pub const INTO_ITER: Protocol = Protocol {
    name: "into_iter",
    hash: Hash::new(0x15a85c8d774b4065),
};

/// The function to call to continue iteration.
pub const NEXT: Protocol = Protocol {
    name: "next",
    hash: Hash::new(0xc3cde069de2ba320),
};

/// Function used to convert an argument into a future.
pub const INTO_FUTURE: Protocol = Protocol {
    name: "into_future",
    hash: Hash::new(0x596e6428deabfda2),
};
