use crate::RawStr;
use serde::{Deserialize, Serialize};
use std::convert;
use std::fmt;

/// The name of an item.
///
/// This is made up of a collection of strings, like `["foo", "bar"]`.
/// This is indicated in rune as `foo::bar`.
#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Item {
    path: Vec<Component>,
}

impl Item {
    /// Construct an empty item.
    pub const fn empty() -> Self {
        Self { path: Vec::new() }
    }

    /// Construct a new item path.
    pub fn new(path: Vec<Component>) -> Self {
        Self { path }
    }

    /// Construct a new item path.
    pub fn of<I>(iter: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Component>,
    {
        Self {
            path: iter.into_iter().map(Into::into).collect::<Vec<Component>>(),
        }
    }

    /// Check if the item is empty.
    pub fn is_empty(&self) -> bool {
        self.path.is_empty()
    }

    /// Push the given component to the current item.
    pub fn push<C>(&mut self, component: C)
    where
        C: Into<Component>,
    {
        self.path.push(component.into());
    }

    /// Push the given component to the current item.
    pub fn pop(&mut self) -> Option<Component> {
        self.path.pop()
    }

    /// Construct a new vector from the current item.
    pub fn as_vec(&self) -> Vec<Component> {
        self.path.clone()
    }

    /// Convert into a vector from the current item.
    pub fn into_vec(self) -> Vec<Component> {
        self.path
    }

    /// If the item only contains one element, return that element.
    pub fn as_local(&self) -> Option<&str> {
        match self.path.last() {
            Some(Component::String(last)) if self.path.len() == 1 => Some(&*last),
            _ => None,
        }
    }

    /// Join this path with another.
    pub fn join<I>(&self, other: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Component>,
    {
        let mut path = self.path.to_vec();

        for part in other {
            path.push(part.into());
        }

        Self::new(path)
    }

    /// Clone and extend the item path.
    pub fn extended<C>(&self, part: C) -> Self
    where
        C: Into<Component>,
    {
        let mut path = self.path.clone();
        path.push(part.into());
        Self::new(path)
    }

    /// Access the last component in the path.
    pub fn last(&self) -> Option<&Component> {
        self.path.last()
    }
}

/// Format implementation for item.
///
/// An empty item is formatted as `{empty}`.
///
/// # Examples
///
/// ```rust
/// use runestick::{Item, Component::*};
///
/// assert_eq!("{empty}", Item::empty().to_string());
/// assert_eq!("hello::$block0", Item::of(&[String("hello".into()), Block(0)]).to_string());
/// ```
impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut it = self.path.iter();

        if let Some(last) = it.next_back() {
            for p in it {
                write!(f, "{}::", p)?;
            }

            write!(f, "{}", last)
        } else {
            write!(f, "{{empty}}")
        }
    }
}

impl<'a> IntoIterator for Item {
    type IntoIter = std::vec::IntoIter<Component>;
    type Item = Component;

    fn into_iter(self) -> Self::IntoIter {
        self.path.to_vec().into_iter()
    }
}

impl<'a> IntoIterator for &'a Item {
    type IntoIter = std::slice::Iter<'a, Component>;
    type Item = &'a Component;

    fn into_iter(self) -> Self::IntoIter {
        self.path.iter()
    }
}

/// The component of an item.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Component {
    /// A regular string component.
    String(String),
    /// A nested block with an index.
    ///
    /// The block for the current function is always `0`.
    Block(usize),
    /// A closure component.
    Closure(usize),
    /// An async block, like `async {  }`.
    AsyncBlock(usize),
    /// An expanded macro.
    Macro(usize),
}

impl fmt::Display for Component {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(s) => write!(fmt, "{}", s),
            Self::Block(n) => write!(fmt, "$block{}", n),
            Self::Closure(n) => write!(fmt, "$closure{}", n),
            Self::AsyncBlock(n) => write!(fmt, "$async{}", n),
            Self::Macro(n) => write!(fmt, "$macro{}", n),
        }
    }
}

impl convert::AsRef<Component> for Component {
    fn as_ref(&self) -> &Component {
        self
    }
}

impl From<RawStr> for Component {
    fn from(value: RawStr) -> Self {
        Self::String((&*value).to_owned())
    }
}

impl From<&RawStr> for Component {
    fn from(value: &RawStr) -> Self {
        Self::String((&**value).to_owned())
    }
}

impl From<&str> for Component {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<&&str> for Component {
    fn from(value: &&str) -> Self {
        Self::String((*value).to_owned())
    }
}

impl From<String> for Component {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&String> for Component {
    fn from(value: &String) -> Self {
        Self::String(value.clone())
    }
}

impl From<&Component> for Component {
    fn from(value: &Component) -> Self {
        value.clone()
    }
}
