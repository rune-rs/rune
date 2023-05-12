//! The `std::string` module.

use core::char;
use core::fmt::{self, Write};
use core::num;

use crate::no_std::prelude::*;

use crate::runtime::{Bytes, Iterator, Panic, Protocol, Value, VmErrorKind, VmResult};
use crate::{Any, ContextError, Module};

/// Construct the `std::string` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["string"]);

    module.ty::<String>()?;

    module.function(["String", "from_str"], <String as From<&str>>::from)?;
    module.function(["String", "new"], String::new)?;
    module.function(["String", "with_capacity"], String::with_capacity)?;

    module.associated_function("cmp", str::cmp)?;
    module.associated_function("len", String::len)?;
    module.associated_function("starts_with", str::starts_with::<&str>)?;
    module.associated_function("ends_with", str::ends_with::<&str>)?;
    module.associated_function("capacity", String::capacity)?;
    module.associated_function("clear", String::clear)?;
    module.associated_function("contains", str::contains::<&str>)?;
    module.associated_function("push", String::push)?;
    module.associated_function("push_str", String::push_str)?;
    module.associated_function("reserve", String::reserve)?;
    module.associated_function("reserve_exact", String::reserve_exact)?;
    module.associated_function("into_bytes", into_bytes)?;
    module.associated_function("clone", String::clone)?;
    module.associated_function("shrink_to_fit", String::shrink_to_fit)?;
    module.associated_function("char_at", char_at)?;
    module.associated_function("split", string_split)?;
    module.associated_function("trim", string_trim)?;
    module.associated_function("trim_end", string_trim_end)?;
    module.associated_function("replace", str::replace::<&str>)?;
    // TODO: deprecate this variant.
    module.associated_function("split_str", string_split)?;
    module.associated_function("is_empty", str::is_empty)?;
    module.associated_function("chars", string_chars)?;
    module.associated_function(Protocol::ADD, add)?;
    module.associated_function(Protocol::ADD_ASSIGN, String::push_str)?;
    module.associated_function(Protocol::INDEX_GET, string_index_get)?;
    module.associated_function("get", string_get)?;

    // TODO: parameterize once generics are available.
    module.function(["parse_int"], parse_int)?;
    module.function(["parse_char"], parse_char)?;

    Ok(module)
}

#[derive(Any, Debug, Clone, Copy)]
#[rune(module = crate, item = ::std::string, install_with = NotCharBoundary::install)]
struct NotCharBoundary(());

impl NotCharBoundary {
    fn string_display(&self, s: &mut String) -> fmt::Result {
        write!(s, "index outside of character boundary")
    }

    fn install(m: &mut Module) -> Result<(), ContextError> {
        m.associated_function(Protocol::STRING_DISPLAY, Self::string_display)?;
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

fn string_split(this: &str, value: Value) -> VmResult<Iterator> {
    let lines = match value {
        Value::String(s) => this
            .split(vm_try!(s.borrow_ref()).as_str())
            .map(String::from)
            .collect::<Vec<String>>(),
        Value::StaticString(s) => this
            .split(s.as_str())
            .map(String::from)
            .collect::<Vec<String>>(),
        Value::Char(pat) => this.split(pat).map(String::from).collect::<Vec<String>>(),
        value => return VmResult::err(vm_try!(VmErrorKind::bad_argument::<String>(0, &value))),
    };

    VmResult::Ok(Iterator::from_double_ended(
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

fn parse_int(s: &str) -> Result<i64, num::ParseIntError> {
    str::parse::<i64>(s)
}

fn parse_char(s: &str) -> Result<char, char::ParseCharError> {
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
fn string_get(s: &str, key: Value) -> VmResult<Option<String>> {
    use crate::runtime::{FromValue, RangeLimits, TypeOf};

    match key {
        Value::Range(range) => {
            let range = vm_try!(range.borrow_ref());

            let start = match range.start.clone() {
                Some(value) => Some(vm_try!(<usize>::from_value(value))),
                None => None,
            };

            let end = match range.end.clone() {
                Some(value) => Some(vm_try!(<usize>::from_value(value))),
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
                    _ => return VmResult::err(VmErrorKind::UnsupportedRange),
                },
            };

            VmResult::Ok(out.map(|out| out.to_owned()))
        }
        index => VmResult::err(VmErrorKind::UnsupportedIndexGet {
            target: String::type_info(),
            index: vm_try!(index.type_info()),
        }),
    }
}

/// Get a specific string index.
fn string_index_get(s: &str, key: Value) -> VmResult<String> {
    match vm_try!(string_get(s, key)) {
        Some(slice) => VmResult::Ok(slice),
        None => VmResult::err(Panic::custom("missing string slice")),
    }
}
