use crate::LoadError;

/// A collection of errors.
#[derive(Debug)]
pub struct Errors {
    errors: Vec<LoadError>,
}

impl Errors {
    /// Construct a new collection of errors.
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Push an error to the collection.
    pub fn push(&mut self, error: LoadError) {
        self.errors.push(error);
    }

    /// Test if the collection of errors is empty.
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }
}

impl IntoIterator for Errors {
    type Item = LoadError;
    type IntoIter = std::vec::IntoIter<LoadError>;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.into_iter()
    }
}

impl<'a> IntoIterator for &'a Errors {
    type Item = &'a LoadError;
    type IntoIter = std::slice::Iter<'a, LoadError>;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.iter()
    }
}
