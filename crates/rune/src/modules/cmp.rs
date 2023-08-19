//! The `std::cmp` module.

use core::cmp::Ordering;

use crate::runtime::Protocol;
use crate::{ContextError, Module};

/// Construct the `std::cmp` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["cmp"]);

    let ty = module.ty::<Ordering>()?.docs([
        "An `Ordering` is the result of a comparison between two values.",
        "",
        "# Examples",
        "",
        "```",
        "use std::cmp::Ordering;",
        "",
        "let result = 1.cmp(2);",
        "assert_eq!(Ordering::Less, result);",
        "",
        "let result = 1.cmp(1);",
        "assert_eq!(Ordering::Equal, result);",
        "",
        "let result = 2.cmp(1);",
        "assert_eq!(Ordering::Greater, result);",
        "```",
    ]);

    let mut ty = ty.make_enum(&["Less", "Equal", "Greater"])?;

    ty.variant_mut(0)?
        .make_empty()?
        .constructor(|| Ordering::Less)?
        .docs(["An ordering where a compared value is less than another."]);

    ty.variant_mut(1)?
        .make_empty()?
        .constructor(|| Ordering::Equal)?
        .docs(["An ordering where a compared value is equal to another."]);

    ty.variant_mut(2)?
        .make_empty()?
        .constructor(|| Ordering::Greater)?
        .docs(["An ordering where a compared value is greater than another."]);

    module.associated_function(Protocol::PARTIAL_EQ, |lhs: Ordering, rhs: Ordering| {
        lhs == rhs
    })?;
    module.associated_function(Protocol::EQ, |lhs: Ordering, rhs: Ordering| lhs == rhs)?;
    Ok(module)
}
