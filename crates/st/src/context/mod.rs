use crate::collections::HashMap;
use crate::hash::Hash;
use crate::value::{ValueType, ValueTypeInfo};
use crate::vm::{Vm, VmError};
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
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
        name: String,
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
    #[error("type with name `{name}` ({hash}) already exists with `{existing}`")]
    ConflictingType {
        /// The name we tried to register.
        name: Item,
        /// The hash of the conflicting type.
        hash: Hash,
        /// The type information for the type that already existed.
        existing: ValueTypeInfo,
    },
}

/// The handler of a function.
type Handler = dyn for<'vm> Fn(&'vm mut Vm, usize) -> BoxFuture<'vm, Result<(), VmError>> + Sync;

/// Helper alias for boxed futures.
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

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
pub struct FnSignature {
    path: Arc<Item>,
    instance: Option<ValueTypeInfo>,
    name: String,
    args: Option<usize>,
}

impl FnSignature {
    /// Construct a new function signature.
    pub fn new_instance(path: Arc<Item>, instance: ValueTypeInfo, name: &str, args: usize) -> Self {
        Self {
            path,
            instance: Some(instance),
            name: name.to_owned(),
            args: Some(args),
        }
    }

    /// Construct a new raw signature.
    pub fn new_raw(path: Arc<Item>, name: &str) -> Self {
        Self {
            path,
            instance: None,
            name: name.to_owned(),
            args: None,
        }
    }

    /// Construct a new global function signature.
    pub fn new_global(path: Arc<Item>, name: &str, args: usize) -> Self {
        Self {
            path,
            instance: None,
            name: name.to_owned(),
            args: Some(args),
        }
    }
}

impl fmt::Display for FnSignature {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(instance) = self.instance {
            write!(fmt, "<{}>::{}(self", instance, self.name)?;

            if let Some(args) = self.args {
                for n in 0..args {
                    write!(fmt, ", #{}", n)?;
                }
            } else {
                write!(fmt, ", ...")?;
            }

            write!(fmt, ")")?;
        } else {
            write!(fmt, "{}::{}(", self.path, self.name)?;

            if let Some(args) = self.args {
                let mut it = 0..args;
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
    functions: HashMap<Hash, Box<Handler>>,
    /// Information on functions.
    functions_info: HashMap<Hash, FnSignature>,
    /// Registered types.
    types: HashMap<Hash, TypeInfo>,
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
        this.install(crate::packages::test::module()?)?;
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

    /// Lookup the given function.
    pub fn lookup(&self, hash: Hash) -> Option<&Handler> {
        let handler = self.functions.get(&hash)?;
        Some(&*handler)
    }

    /// Lookup a type by hash.
    pub fn lookup_type(&self, hash: Hash) -> Option<&TypeInfo> {
        self.types.get(&hash)
    }

    /// Install the specified module.
    pub fn install(&mut self, module: Module) -> Result<(), ContextError> {
        let base = module.path;

        for (name, (handler, signature)) in module.functions.into_iter() {
            let hash = Hash::function(base.into_iter().chain(Some(&name)));

            if let Some(old) = self.functions_info.insert(hash, signature) {
                return Err(ContextError::ConflictingFunction {
                    signature: old,
                    hash,
                });
            }

            self.functions.insert(hash, handler);
        }

        for ((ty, name), (handler, signature)) in module.instance_functions.into_iter() {
            let hash = Hash::instance_function(ty, Hash::of(name));

            if let Some(old) = self.functions_info.insert(hash, signature) {
                return Err(ContextError::ConflictingFunction {
                    signature: old,
                    hash,
                });
            }

            self.functions.insert(hash, handler);
        }

        for (value_type, (type_info, name)) in module.types.into_iter() {
            let name = base.extended(name);
            let hash = Hash::of_type(&name);

            let type_info = TypeInfo {
                name,
                value_type,
                type_info,
            };

            if let Some(existing) = self.types.insert(hash, type_info) {
                return Err(ContextError::ConflictingType {
                    name: existing.name,
                    hash,
                    existing: existing.type_info,
                });
            }
        }

        Ok(())
    }
}
