//! The `std::string` module.

use crate::{Any, Bytes, ContextError, Iterator, Module, Protocol, Value, VmError, VmErrorKind};

/// Construct the `std::string` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["string"]);

    module.ty::<String>()?;

    module.function(&["String", "from_str"], <String as From<&str>>::from)?;
    module.function(&["String", "new"], String::new)?;
    module.function(&["String", "with_capacity"], String::with_capacity)?;

    module.inst_fn("cmp", str::cmp)?;
    module.inst_fn("len", String::len)?;
    module.inst_fn("starts_with", str::starts_with::<&str>)?;
    module.inst_fn("ends_with", str::ends_with::<&str>)?;
    module.inst_fn("capacity", String::capacity)?;
    module.inst_fn("clear", String::clear)?;
    module.inst_fn("push", String::push)?;
    module.inst_fn("push_str", String::push_str)?;
    module.inst_fn("reserve", String::reserve)?;
    module.inst_fn("reserve_exact", String::reserve_exact)?;
    module.inst_fn("into_bytes", into_bytes)?;
    module.inst_fn("clone", String::clone)?;
    module.inst_fn("shrink_to_fit", String::shrink_to_fit)?;
    module.inst_fn("char_at", char_at)?;
    module.inst_fn("split", string_split)?;
    module.inst_fn("trim", string_trim)?;
    module.inst_fn("trim_end", string_trim_end)?;
    module.inst_fn("replace", str::replace::<&str>)?;
    // TODO: deprecate this variant.
    module.inst_fn("split_str", string_split)?;
    module.inst_fn("is_empty", str::is_empty)?;
    module.inst_fn("chars", string_chars)?;
    module.inst_fn(Protocol::ADD, add)?;
    module.inst_fn(Protocol::ADD_ASSIGN, String::push_str)?;
    module.inst_fn(Protocol::INDEX_GET, string_index_get)?;
    module.inst_fn("get", string_get)?;

    // TODO: parameterize once generics are available.
    module.function(&["parse_int"], parse_int)?;
    module.function(&["parse_char"], parse_char)?;

    Ok(module)
}

#[derive(Any, Debug, Clone, Copy)]
#[rune(module = "crate", install_with = "NotCharBoundary::install")]
struct NotCharBoundary(());

impl NotCharBoundary {
    fn string_display(&self, s: &mut String) -> std::fmt::Result {
        use std::fmt::Write as _;
        write!(s, "index outside of character boundary")
    }

    fn install(m: &mut Module) -> Result<(), ContextError> {
        m.inst_fn(crate::Protocol::STRING_DISPLAY, Self::string_display)?;
        Ok(())
    }
}

/// into_bytes shim for strings.
fn into_bytes(s: String) -> Bytes {
    Bytes::from_vec(s.into_bytes())
}

fn char_at(s: &str, index: usize) -> Option<char> {
    if !s.is_char_boundary(index) {
        return None;
    }

    s[index..].chars().next()
}

fn string_split(this: &str, value: Value) -> Result<Iterator, VmError> {
    let lines = match value {
        Value::String(s) => this
            .split(s.borrow_ref()?.as_str())
            .map(String::from)
            .collect::<Vec<String>>(),
        Value::StaticString(s) => this
            .split(s.as_str())
            .map(String::from)
            .collect::<Vec<String>>(),
        Value::Char(pat) => this.split(pat).map(String::from).collect::<Vec<String>>(),
        value => return Err(VmError::bad_argument::<String>(0, &value)?),
    };

    Ok(Iterator::from_double_ended(
        "std::str::Split",
        lines.into_iter(),
    ))
}

fn string_trim(this: &str) -> String {
    this.trim().to_owned()
}

fn string_trim_end(this: &str) -> String {
    this.trim_end().to_owned()
}

fn parse_int(s: &str) -> Result<i64, std::num::ParseIntError> {
    str::parse::<i64>(s)
}

fn parse_char(s: &str) -> Result<char, std::char::ParseCharError> {
    str::parse::<char>(s)
}

/// The add operation for strings.
fn add(a: &str, b: &str) -> String {
    let mut string = String::with_capacity(a.len() + b.len());
    string.push_str(a);
    string.push_str(b);
    string
}

fn string_chars(s: &str) -> Iterator {
    let iter = s.chars().collect::<Vec<_>>().into_iter();
    Iterator::from_double_ended("std::str::Chars", iter)
}

/// Get a specific string index.
fn string_get(s: &str, key: Value) -> Result<Option<String>, VmError> {
    use crate::{FromValue as _, RangeLimits, TypeOf as _};

    match key {
        Value::Range(range) => {
            let range = range.borrow_ref()?;

            let start = match range.start.clone() {
                Some(value) => Some(<usize>::from_value(value)?),
                None => None,
            };

            let end = match range.end.clone() {
                Some(value) => Some(<usize>::from_value(value)?),
                None => None,
            };

            let out = match range.limits {
                RangeLimits::HalfOpen => match (start, end) {
                    (Some(start), Some(end)) => s.get(start..end),
                    (Some(start), None) => s.get(start..),
                    (None, Some(end)) => s.get(..end),
                    (None, None) => s.get(..),
                },
                RangeLimits::Closed => match (start, end) {
                    (Some(start), Some(end)) => s.get(start..=end),
                    (None, Some(end)) => s.get(..=end),
                    _ => return Err(VmError::from(VmErrorKind::UnsupportedRange)),
                },
            };

            Ok(out.map(|out| out.to_owned()))
        }
        index => Err(VmError::from(VmErrorKind::UnsupportedIndexGet {
            target: String::type_info(),
            index: index.type_info()?,
        })),
    }
}

/// Get a specific string index.
fn string_index_get(s: &str, key: Value) -> Result<String, VmError> {
    string_get(s, key)?.ok_or_else(|| VmError::panic("missing string slice"))
}
