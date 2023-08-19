//! The `std::ops` module.

use crate::runtime::{
    Function, Protocol, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive,
    Value,
};
use crate::{ContextError, Module};

/// Construct the `std::ops` module.
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::with_crate_item("std", ["ops"]);

    {
        m.ty::<RangeFrom>()?;
        m.type_meta::<RangeFrom>()?
            .make_named_struct(&["start"])?
            .docs([
                "Type for a from range expression `start..`.",
                "",
                "# Examples",
                "",
                "```rune",
                "let range = 0..;",
                "",
                "assert!(!range.contains::<i64>(-10));",
                "assert!(range.contains::<i64>(5));",
                "assert!(range.contains::<i64>(10));",
                "assert!(range.contains::<i64>(20));",
                "",
                "assert!(range is std::ops::RangeFrom);",
                "```",
                "",
                "Ranges can contain any type:",
                "",
                "```rune",
                "let range = 'a'..;",
                "assert_eq!(range.start, 'a');",
                "range.start = 'b';",
                "assert_eq!(range.start, 'b');",
                "```",
                "",
                "Certain ranges can be used as iterators:",
                "",
                "```rune",
                "let range = 'a'..;",
                "assert_eq!(range.iter().take(5).collect::<Vec>(), ['a', 'b', 'c', 'd', 'e']);",
                "```",
            ]);
        m.field_function(Protocol::GET, "start", |r: &RangeFrom| r.start.clone())?;
        m.field_function(Protocol::SET, "start", |r: &mut RangeFrom, value: Value| {
            r.start = value;
        })?;
        m.associated_function(Protocol::INTO_ITER, RangeFrom::iter)?;
        m.function_meta(RangeFrom::contains)?;
        m.function_meta(RangeFrom::iter__meta)?;
    }

    {
        m.ty::<RangeFull>()?;
        m.type_meta::<RangeFull>()?.make_empty_struct()?.docs([
            "Type for a full range expression `..`.",
            "",
            "# Examples",
            "",
            "```rune",
            "let range = ..;",
            "",
            "assert!(range.contains::<i64>(-10));",
            "assert!(range.contains::<i64>(5));",
            "assert!(range.contains::<i64>(10));",
            "assert!(range.contains::<i64>(20));",
            "",
            "assert!(range is std::ops::RangeFull);",
            "```",
        ]);
        m.function_meta(RangeFull::contains)?;
    }

    {
        m.ty::<RangeInclusive>()?;
        m.type_meta::<RangeInclusive>()?
            .make_named_struct(&["start", "end"])?
            .docs([
                "Type for an inclusive range expression `start..=end`.",
                "",
                "# Examples",
                "",
                "```rune",
                "let range = 0..=10;",
                "",
                "assert!(!range.contains::<i64>(-10));",
                "assert!(range.contains::<i64>(5));",
                "assert!(range.contains::<i64>(10));",
                "assert!(!range.contains::<i64>(20));",
                "",
                "assert!(range is std::ops::RangeInclusive);",
                "```",
                "",
                "Ranges can contain any type:",
                "",
                "```rune",
                "let range = 'a'..='f';",
                "assert_eq!(range.start, 'a');",
                "range.start = 'b';",
                "assert_eq!(range.start, 'b');",
                "assert_eq!(range.end, 'f');",
                "range.end = 'g';",
                "assert_eq!(range.end, 'g');",
                "```",
                "",
                "Certain ranges can be used as iterators:",
                "",
                "```rune",
                "let range = 'a'..='e';",
                "assert_eq!(range.iter().collect::<Vec>(), ['a', 'b', 'c', 'd', 'e']);",
                "```",
            ]);
        m.field_function(Protocol::GET, "start", |r: &RangeInclusive| r.start.clone())?;
        m.field_function(
            Protocol::SET,
            "start",
            |r: &mut RangeInclusive, value: Value| {
                r.start = value;
            },
        )?;
        m.field_function(Protocol::GET, "end", |r: &RangeInclusive| r.end.clone())?;
        m.field_function(
            Protocol::SET,
            "end",
            |r: &mut RangeInclusive, value: Value| {
                r.end = value;
            },
        )?;
        m.associated_function(Protocol::INTO_ITER, RangeInclusive::iter)?;
        m.function_meta(RangeInclusive::contains)?;
        m.function_meta(RangeInclusive::iter__meta)?.docs([
            "Iterate over the range.",
            "",
            "This panics if the range is not a well-defined range.",
        ]);
    }

    {
        m.ty::<RangeToInclusive>()?;
        m.type_meta::<RangeToInclusive>()?
            .make_named_struct(&["end"])?
            .docs([
                "Type for an inclusive range expression `..=end`.",
                "",
                "# Examples",
                "",
                "```rune",
                "let range = ..=10;",
                "assert!(range.contains::<i64>(-10));",
                "assert!(range.contains::<i64>(5));",
                "assert!(range.contains::<i64>(10));",
                "assert!(!range.contains::<i64>(20));",
                "",
                "assert!(range is std::ops::RangeToInclusive);",
                "```",
                "",
                "Ranges can contain any type:",
                "",
                "```rune",
                "let range = ..='f';",
                "assert_eq!(range.end, 'f');",
                "range.end = 'g';",
                "assert_eq!(range.end, 'g');",
                "```",
            ]);
        m.field_function(Protocol::GET, "end", |r: &RangeToInclusive| r.end.clone())?;
        m.field_function(
            Protocol::SET,
            "end",
            |r: &mut RangeToInclusive, value: Value| {
                r.end = value;
            },
        )?;
        m.function_meta(RangeToInclusive::contains)?;
    }

    {
        m.ty::<RangeTo>()?;
        m.type_meta::<RangeTo>()?
            .make_named_struct(&["end"])?
            .docs([
                "Type for an inclusive range expression `..end`.",
                "",
                "# Examples",
                "",
                "```rune",
                "let range = ..10;",
                "assert!(range.contains::<i64>(-10));",
                "assert!(range.contains::<i64>(5));",
                "assert!(!range.contains::<i64>(10));",
                "assert!(!range.contains::<i64>(20));",
                "",
                "assert!(range is std::ops::RangeTo);",
                "```",
                "",
                "Ranges can contain any type:",
                "",
                "```rune",
                "let range = ..'f';",
                "assert_eq!(range.end, 'f');",
                "range.end = 'g';",
                "assert_eq!(range.end, 'g');",
                "```",
            ]);
        m.field_function(Protocol::GET, "end", |r: &RangeTo| r.end.clone())?;
        m.field_function(Protocol::SET, "end", |r: &mut RangeTo, value: Value| {
            r.end = value;
        })?;
        m.function_meta(RangeTo::contains)?;
    }

    {
        m.ty::<Range>()?;
        m.type_meta::<Range>()?
            .make_named_struct(&["start", "end"])?
            .docs([
                "Type for a range expression `start..end`.",
                "",
                "# Examples",
                "",
                "```rune",
                "let range = 0..10;",
                "assert!(!range.contains::<i64>(-10));",
                "assert!(range.contains::<i64>(5));",
                "assert!(!range.contains::<i64>(10));",
                "assert!(!range.contains::<i64>(20));",
                "",
                "assert!(range is std::ops::Range);",
                "```",
                "",
                "Ranges can contain any type:",
                "",
                "```rune",
                "let range = 'a'..'f';",
                "assert_eq!(range.start, 'a');",
                "range.start = 'b';",
                "assert_eq!(range.start, 'b');",
                "assert_eq!(range.end, 'f');",
                "range.end = 'g';",
                "assert_eq!(range.end, 'g');",
                "```",
                "",
                "Certain ranges can be used as iterators:",
                "",
                "```rune",
                "let range = 'a'..'e';",
                "assert_eq!(range.iter().collect::<Vec>(), ['a', 'b', 'c', 'd']);",
                "```",
            ]);
        m.field_function(Protocol::GET, "start", |r: &Range| r.start.clone())?;
        m.field_function(Protocol::SET, "start", |r: &mut Range, value: Value| {
            r.start = value;
        })?;
        m.field_function(Protocol::GET, "end", |r: &Range| r.end.clone())?;
        m.field_function(Protocol::SET, "end", |r: &mut Range, value: Value| {
            r.end = value;
        })?;
        m.associated_function(Protocol::INTO_ITER, Range::iter)?;
        m.function_meta(Range::contains)?;
        m.function_meta(Range::iter__meta)?;
    }

    m.ty::<Function>()?.docs([
        "The type of a function in Rune.",
        "",
        "Functions can be called using call expression syntax, such as `<expr>()`.",
        "",
        "There are multiple different kind of things which can be coerced into a function in Rune:",
        "* Regular functions.",
        "* Closures (which might or might not capture their environment).",
        "* Built-in constructors for tuple types (tuple structs, tuple variants).",
        "",
        "# Examples",
        "",
        "```rune",
        "// Captures the constructor for the `Some(<value>)` tuple variant.",
        "let build_some = Some;",
        "assert_eq!(build_some(42), Some(42));",
        "",
        "fn build(value) {",
        "    Some(value)",
        "}",
        "",
        "// Captures the function previously defined.",
        "let build_some = build;",
        "assert_eq!(build_some(42), Some(42));",
        "```",
    ]);

    Ok(m)
}
