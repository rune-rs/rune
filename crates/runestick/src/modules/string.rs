//! The `std::string` module.

use crate::{Bytes, ContextError, Iterator, Module, Protocol};

/// Construct the `std::string` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["string"]);

    module.ty::<String>()?;

    module.function(&["String", "from_str"], <String as From<&str>>::from)?;
    module.function(&["String", "new"], String::new)?;
    module.function(&["String", "with_capacity"], String::with_capacity)?;

    module.inst_fn("len", String::len)?;
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
    module.inst_fn("is_empty", str::is_empty)?;
    module.inst_fn(Protocol::ADD, add)?;
    module.inst_fn(Protocol::ADD_ASSIGN, String::push_str)?;

    // TODO: parameterize once generics are available.
    module.function(&["parse_int"], parse_int)?;

    Ok(module)
}

#[derive(Debug, Clone, Copy)]
struct NotCharBoundary(());

/// into_bytes shim for strings.
fn into_bytes(s: String) -> Bytes {
    Bytes::from_vec(s.into_bytes())
}

fn char_at(s: &str, index: usize) -> Result<Option<char>, NotCharBoundary> {
    if !s.is_char_boundary(index) {
        return Err(NotCharBoundary(()));
    }

    Ok(s[index..].chars().next())
}

fn string_split(s: &str, pat: char) -> Iterator {
    let parts = s.split(pat).map(String::from).collect::<Vec<String>>();
    Iterator::from_double_ended("std::str::Split", parts.into_iter())
}

fn parse_int(s: &str) -> Result<i64, std::num::ParseIntError> {
    str::parse::<i64>(s)
}

/// The add operation for strings.
fn add(a: &str, b: &str) -> String {
    let mut string = String::with_capacity(a.len() + b.len());
    string.push_str(a);
    string.push_str(b);
    string
}

crate::__internal_impl_any!(NotCharBoundary);
