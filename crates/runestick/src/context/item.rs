use std::fmt;

/// The name of an item.
///
/// This is made up of a collection of strings, like `["foo", "bar"]`.
/// This is indicated in rune as `foo::bar`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Item {
    path: Box<[String]>,
}

impl Item {
    /// Construct a new item path.
    pub fn new(path: Vec<String>) -> Self {
        Self {
            path: path.into_boxed_slice(),
        }
    }

    /// If the item only contains one element, return that element.
    pub fn as_local(&self) -> Option<&str> {
        match self.path.last() {
            Some(last) if self.path.len() == 1 => Some(&*last),
            _ => None,
        }
    }

    /// Construct a new item path.
    pub fn of<I>(iter: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        Self {
            path: iter
                .into_iter()
                .map(|s| s.as_ref().to_owned())
                .collect::<Box<[String]>>(),
        }
    }

    /// Join this path with another.
    pub fn join<I>(&self, other: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut path = self.path.to_vec();

        for part in other {
            path.push(part.as_ref().to_owned());
        }

        Self::new(path)
    }

    /// Clone and extend the item path.
    pub fn extended<S>(&self, part: S) -> Self
    where
        S: AsRef<str>,
    {
        let mut path = self.path.clone().into_vec();
        path.push(part.as_ref().to_owned());
        Self::new(path)
    }

    /// Access the last component in the path.
    pub fn last(&self) -> Option<&str> {
        self.path.last().map(String::as_str)
    }
}

impl fmt::Display for Item {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut it = self.path.iter().peekable();

        while let Some(part) = it.next() {
            write!(fmt, "{}", part)?;

            if it.peek().is_some() {
                write!(fmt, "::")?;
            }
        }

        Ok(())
    }
}

impl<'a> IntoIterator for &'a Item {
    type IntoIter = std::slice::Iter<'a, String>;
    type Item = &'a String;

    fn into_iter(self) -> Self::IntoIter {
        self.path.iter()
    }
}
