//! Public API for extracting type information from compiled units.
//!
//! This module provides types for embedders to query function signatures
//! and type annotations from compiled Rune code. Part of Phase 3 of gradual typing.
//!
//! Note: This module uses `AnnotatedType` instead of `TypeInfo` to avoid
//! naming conflicts with `runtime::TypeInfo` which is used for runtime type
//! introspection.
//!
//! # Example
//!
//! ```ignore
//! let unit = rune::prepare(&mut sources).build()?;
//!
//! // Query a specific function
//! if let Some(sig) = unit.function_signature_by_name("add") {
//!     println!("Function: {}", sig.name);
//!     for param in &sig.parameters {
//!         if let Some(ty) = &param.type_info {
//!             println!("  {}: {}", param.name, ty.to_type_string());
//!         }
//!     }
//! }
//! ```

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{String, Vec};
use crate::Hash;

/// Type annotation extracted from source code.
///
/// Represents the type of a parameter or return value as written in source code.
/// For untyped values, the corresponding `Option<AnnotatedType>` will be `None`.
///
/// Note: Named `AnnotatedType` instead of `TypeInfo` to avoid conflicts with
/// `runtime::TypeInfo` which is used for runtime type introspection.
#[derive(Debug, TryClone)]
#[non_exhaustive]
pub enum AnnotatedType {
    /// A named type (e.g., `i64`, `String`, `foo::Bar`)
    Named {
        /// Full path of the type as written in source
        path: String,
    },
    /// A tuple type (e.g., `(i64, String)`)
    Tuple(Vec<AnnotatedType>),
    /// The never type `!`
    Never,
}

impl AnnotatedType {
    /// Check if this represents a primitive type.
    ///
    /// Returns `true` for: `i64`, `f64`, `bool`, `String`, `char`, `u8`
    pub fn is_primitive(&self) -> bool {
        matches!(self, AnnotatedType::Named { path, .. }
            if matches!(path.as_str(), "i64" | "f64" | "bool" | "String" | "char" | "u8"))
    }

    /// Convert to a human-readable type string.
    ///
    /// # Examples
    ///
    /// - `AnnotatedType::Named { path: "i64" }` → `"i64"`
    /// - `AnnotatedType::Tuple([i64, String])` → `"(i64, String)"`
    /// - `AnnotatedType::Never` → `"!"`
    pub fn to_type_string(&self) -> String {
        match self {
            AnnotatedType::Named { path, .. } => path.try_clone().unwrap_or_default(),
            AnnotatedType::Tuple(types) => {
                let mut result = String::new();
                let _ = result.try_push('(');
                for (i, ty) in types.iter().enumerate() {
                    if i > 0 {
                        let _ = result.try_push_str(", ");
                    }
                    let _ = result.try_push_str(&ty.to_type_string());
                }
                let _ = result.try_push(')');
                result
            }
            AnnotatedType::Never => String::try_from("!").unwrap_or_default(),
        }
    }
}

/// Type information for a function parameter.
#[derive(Debug, TryClone)]
#[non_exhaustive]
pub struct ParameterType {
    /// Parameter name
    pub name: String,
    /// Type annotation if present, `None` for untyped parameters
    pub type_info: Option<AnnotatedType>,
    /// Position in parameter list (0-indexed)
    pub position: usize,
}

/// Complete signature information for a function.
///
/// Contains all available metadata about a function including its name,
/// path, parameters, and return type.
#[derive(Debug, TryClone)]
#[non_exhaustive]
pub struct FunctionSignature {
    /// Function name (last component of path)
    pub name: String,
    /// Full item path
    pub path: String,
    /// Function hash for lookup
    pub hash: Hash,
    /// Whether the function is async
    pub is_async: bool,
    /// Parameter types in order
    pub parameters: Vec<ParameterType>,
    /// Return type if annotated, `None` for untyped returns
    pub return_type: Option<AnnotatedType>,
}
