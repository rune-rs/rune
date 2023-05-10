#[cfg(feature = "doc")]
use crate::no_std::prelude::*;

/// The documentation for a function.
///
/// If the `doc` feature is disabled, this is a zero-sized type.
#[derive(Debug, Clone)]
pub(crate) struct Docs {
    /// Lines of documentation.
    #[cfg(feature = "doc")]
    docs: Vec<String>,
    /// Names of arguments.
    #[cfg(feature = "doc")]
    arguments: Option<Vec<String>>,
}

impl Docs {
    pub(crate) const EMPTY: Docs = Docs {
        #[cfg(feature = "doc")]
        docs: Vec::new(),
        #[cfg(feature = "doc")]
        arguments: None,
    };

    /// Get arguments associated with documentation.
    #[cfg(feature = "doc")]
    pub(crate) fn args(&self) -> Option<&[String]> {
        self.arguments.as_deref()
    }

    /// Get lines of documentation.
    #[cfg(feature = "doc")]
    pub(crate) fn lines(&self) -> &[String] {
        &self.docs
    }

    /// Update documentation.
    #[cfg(feature = "doc")]
    pub(crate) fn set_docs<S>(&mut self, docs: S)
    where
        S: IntoIterator,
        S::Item: AsRef<str>,
    {
        self.docs.clear();

        for line in docs {
            self.docs.push(line.as_ref().to_owned());
        }
    }

    #[cfg(not(feature = "doc"))]
    pub(crate) fn set_docs<S>(&mut self, _: S)
    where
        S: IntoIterator,
        S::Item: AsRef<str>,
    {
    }

    /// Update arguments.
    #[cfg(feature = "doc")]
    pub(crate) fn set_arguments<S>(&mut self, arguments: S)
    where
        S: IntoIterator,
        S::Item: AsRef<str>,
    {
        let mut out = self.arguments.take().unwrap_or_default();
        out.clear();

        for argument in arguments {
            out.push(argument.as_ref().to_owned());
        }

        self.arguments = Some(out);
    }

    #[cfg(not(feature = "doc"))]
    pub(crate) fn set_arguments<S>(&mut self, _: S)
    where
        S: IntoIterator,
        S::Item: AsRef<str>,
    {
    }
}

impl Default for Docs {
    #[inline]
    fn default() -> Self {
        Self::EMPTY
    }
}
