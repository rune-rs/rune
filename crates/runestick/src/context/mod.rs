use crate::collections::HashMap;
use crate::hash::Hash;
use crate::value::{ValueType, ValueTypeInfo};
use crate::vm::{Vm, VmError};
use std::fmt;
use thiserror::Error;

mod item;
mod module;

pub use self::item::Item;
pub use self::module::Module;

/// An error raised when building the context.
#[derive(Debug, Error)]
pub enum ContextError {
    /// Error raised when attempting to register a conflicting function.
    #[error("function `{signature}` ({hash}) already exists")]
    ConflictingFunction {
        /// The signature of the conflicting function.
        signature: FnSignature,
        /// The hash of the conflicting function.
        hash: Hash,
    },
    /// Error raised when attempting to register a conflicting function.
    #[error("function with name `{name}` already exists")]
    ConflictingFunctionName {
        /// The name of the conflicting function.
        name: Item,
    },
    /// Error raised when attempting to register a conflicting instance function.
    #[error("instance function `{name}` for type `{type_info}` already exists")]
    ConflictingInstanceFunction {
        /// Type that we register the instance function for.
        type_info: ValueTypeInfo,
        /// The name of the conflicting function.
        name: String,
    },
    /// Tried to insert a module that conflicted with an already existing one.
    #[error("module `{name}` with hash `{hash}` already exists")]
    ConflictingModule {
        /// The name of the module that conflicted.
        name: Item,
        /// The hash of the module that conflicted.
        hash: Hash,
    },
    /// Raised when we try to register a conflicting type.
    #[error("type with name `{name}` already exists `{existing}`")]
    ConflictingType {
        /// The name we tried to register.
        name: Item,
        /// The type information for the type that already existed.
        existing: ValueTypeInfo,
    },
    /// Error raised when attempting to register an instance function on an
    /// instance which does not exist.
    #[error("instance `{instance_type}` does not exist in module")]
    MissingInstance {
        /// The instance type.
        instance_type: ValueTypeInfo,
    },
}

/// A function handler.
pub(crate) type Handler = Box<dyn Fn(&mut Vm, usize) -> Result<(), VmError>>;

/// Information on a specific type.
#[derive(Debug, Clone)]
pub struct TypeInfo {
    /// The name of the type.
    pub name: Item,
    /// The value type of the type.
    pub value_type: ValueType,
    /// Information on the type.
    pub type_info: ValueTypeInfo,
}

impl fmt::Display for TypeInfo {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{} => {}", self.name, self.type_info)?;
        Ok(())
    }
}

/// A description of a function signature.
#[derive(Debug, Clone)]
pub enum FnSignature {
    Free {
        /// Path to the function.
        path: Item,
        /// Arguments.
        args: Option<usize>,
    },
    Instance {
        /// Path to the instance function.
        path: Item,
        /// Name of the instance function.
        name: String,
        /// Arguments.
        args: Option<usize>,
        /// Information on the self type.
        self_type_info: ValueTypeInfo,
    },
}

impl FnSignature {
    /// Construct a new global function signature.
    pub fn new_free(path: Item, args: Option<usize>) -> Self {
        Self::Free { path, args }
    }

    /// Construct a new function signature.
    pub fn new_inst(
        path: Item,
        name: String,
        args: Option<usize>,
        self_type_info: ValueTypeInfo,
    ) -> Self {
        Self::Instance {
            path,
            name,
            args,
            self_type_info,
        }
    }
}

impl fmt::Display for FnSignature {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Free { path, args } => {
                write!(fmt, "{}(", path)?;

                if let Some(args) = args {
                    let mut it = 0..*args;
                    let last = it.next_back();

                    for n in it {
                        write!(fmt, "#{}, ", n)?;
                    }

                    if let Some(n) = last {
                        write!(fmt, "#{}", n)?;
                    }
                } else {
                    write!(fmt, "...")?;
                }

                write!(fmt, ")")?;
            }
            Self::Instance {
                path,
                name,
                self_type_info,
                args,
            } => {
                write!(fmt, "{}::{}(self: {}", path, name, self_type_info)?;

                if let Some(args) = args {
                    for n in 0..*args {
                        write!(fmt, ", #{}", n)?;
                    }
                } else {
                    write!(fmt, ", ...")?;
                }

                write!(fmt, ")")?;
            }
        }

        Ok(())
    }
}

/// Static run context visible to the virtual machine.
///
/// This contains:
/// * Declared functions.
/// * Declared instance functions.
/// * Type definitions.
#[derive(Default)]
pub struct Context {
    /// Free functions.
    functions: HashMap<Hash, Handler>,
    /// Information on functions.
    functions_info: HashMap<Hash, FnSignature>,
    /// Registered types.
    types: HashMap<Hash, TypeInfo>,
    /// Reverse lookup for types.
    types_rev: HashMap<ValueType, Hash>,
}

impl Context {
    /// Construct a new empty collection of functions.
    pub fn new() -> Self {
        Context::default()
    }

    /// Construct a new collection of functions with default packages installed.
    pub fn with_default_packages() -> Result<Self, ContextError> {
        let mut this = Self::new();
        this.install(crate::packages::core::module()?)?;
        this.install(crate::packages::bytes::module()?)?;
        this.install(crate::packages::string::module()?)?;
        this.install(crate::packages::int::module()?)?;
        this.install(crate::packages::float::module()?)?;
        this.install(crate::packages::test::module()?)?;
        this.install(crate::packages::iter::module()?)?;
        this.install(crate::packages::array::module()?)?;
        this.install(crate::packages::object::module()?)?;
        Ok(this)
    }

    /// Iterate over all available functions
    pub fn iter_functions(&self) -> impl Iterator<Item = (Hash, &FnSignature)> {
        let mut it = self.functions_info.iter();

        std::iter::from_fn(move || {
            let (hash, signature) = it.next()?;
            Some((*hash, signature))
        })
    }

    /// Iterate over all available types.
    pub fn iter_types(&self) -> impl Iterator<Item = (Hash, &TypeInfo)> {
        let mut it = self.types.iter();

        std::iter::from_fn(move || {
            let (hash, ty) = it.next()?;
            Some((*hash, ty))
        })
    }

    /// Install the specified module.
    pub fn install(&mut self, module: Module) -> Result<(), ContextError> {
        for (value_type, (type_info, name)) in module.types.into_iter() {
            let name = module.path.join(&name);
            let hash = Hash::of_type(&name);

            let type_info = TypeInfo {
                name,
                value_type,
                type_info,
            };

            if let Some(existing) = self.types.insert(hash, type_info) {
                return Err(ContextError::ConflictingType {
                    name: existing.name,
                    existing: existing.type_info,
                });
            }

            // reverse lookup for types.
            self.types_rev.insert(value_type, hash);
        }

        for (name, (handler, args)) in module.functions.into_iter() {
            let name = module.path.join(&name);
            let hash = Hash::function(&name);
            let signature = FnSignature::new_free(name, args);

            if let Some(old) = self.functions_info.insert(hash, signature) {
                return Err(ContextError::ConflictingFunction {
                    signature: old,
                    hash,
                });
            }

            self.functions.insert(hash, handler);
        }

        for ((ty, hash), (handler, args, instance_type, name)) in
            module.instance_functions.into_iter()
        {
            let type_info = match self
                .types_rev
                .get(&ty)
                .and_then(|hash| self.types.get(&hash))
            {
                Some(type_info) => type_info,
                None => {
                    return Err(ContextError::MissingInstance { instance_type });
                }
            };

            let hash = Hash::instance_function(ty, hash);
            let signature =
                FnSignature::new_inst(type_info.name.clone(), name, args, type_info.type_info);

            if let Some(old) = self.functions_info.insert(hash, signature) {
                return Err(ContextError::ConflictingFunction {
                    signature: old,
                    hash,
                });
            }

            self.functions.insert(hash, handler);
        }

        Ok(())
    }

    /// Lookup the given function.
    pub(crate) fn lookup(&self, hash: Hash) -> Option<&Handler> {
        let handler = self.functions.get(&hash)?;
        Some(&*handler)
    }

    /// Lookup a type by hash.
    pub(crate) fn lookup_type(&self, hash: Hash) -> Option<&TypeInfo> {
        self.types.get(&hash)
    }
}

/// Trait used to determine what can be used as an instance function name.
pub trait IntoInstFnHash: Copy {
    /// Generate a locally unique hash to check for conflicts.
    fn to_hash(self) -> Hash;

    /// Get a human readable name for the function.
    fn to_name(self) -> String;
}

impl<'a> IntoInstFnHash for &'a str {
    fn to_hash(self) -> Hash {
        Hash::of(self)
    }

    fn to_name(self) -> String {
        self.to_owned()
    }
}

/// The hash helper for a function.
#[derive(Debug, Clone, Copy)]
pub struct FnHash {
    pub(crate) hash: Hash,
    name: &'static str,
}

impl IntoInstFnHash for FnHash {
    fn to_hash(self) -> Hash {
        self.hash
    }

    fn to_name(self) -> String {
        String::from(self.name)
    }
}

impl std::ops::Deref for FnHash {
    type Target = Hash;

    fn deref(&self) -> &Self::Target {
        &self.hash
    }
}

/// The function to call to continue iteration.
pub const NEXT: FnHash = FnHash {
    name: "next",
    hash: Hash(0xc3cde069de2ba320),
};

/// The function to access an index.
pub const INDEX_GET: FnHash = FnHash {
    name: "index_get",
    hash: Hash(0xadb5b27e2a4d2dec),
};

/// The function to set an index.
pub const INDEX_SET: FnHash = FnHash {
    name: "index_set",
    hash: Hash(0x162943f7bd03ad36),
};

/// The function to implement for the addition operation.
pub const ADD: FnHash = FnHash {
    name: "add",
    hash: Hash(0xe4ecf51fa0bf1076),
};

/// The function to implement for the addition assign operation.
pub const ADD_ASSIGN: FnHash = FnHash {
    name: "add_assign",
    hash: Hash(0x42451ccb0a2071a9),
};

/// The function to implement for the subtraction operation.
pub const SUB: FnHash = FnHash {
    name: "sub",
    hash: Hash(0x6fa86a5f18d0bf71),
};

/// The function to implement for the subtraction assign operation.
pub const SUB_ASSIGN: FnHash = FnHash {
    name: "sub_assign",
    hash: Hash(0x5939bb56a1415284),
};

/// The function to implement for the multiply operation.
pub const MUL: FnHash = FnHash {
    name: "mul",
    hash: Hash(0xb09e99dc94091d1c),
};

/// The function to implement for the multiply assign operation.
pub const MUL_ASSIGN: FnHash = FnHash {
    name: "mul_assign",
    hash: Hash(0x29a54b727f980ebf),
};

/// The function to implement for the division operation.
pub const DIV: FnHash = FnHash {
    name: "div",
    hash: Hash(0xf26d6eea1afca6e8),
};

/// The function to implement for the division assign operation.
pub const DIV_ASSIGN: FnHash = FnHash {
    name: "div_assign",
    hash: Hash(0x4dd087a8281c04e6),
};
