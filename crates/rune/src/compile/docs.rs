use crate as rune;
use crate::alloc;
use crate::alloc::prelude::*;
use crate::alloc::{String, Vec};

/// The documentation for a function.
#[derive(Debug, TryClone)]
pub(crate) struct Docs {
    /// Lines of documentation.
    #[cfg(feature = "doc")]
    docs: Vec<String>,
    /// Names of arguments (always available for type checking).
    arguments: Option<Vec<String>>,
}

impl Docs {
    pub(crate) const EMPTY: Docs = Docs {
        #[cfg(feature = "doc")]
        docs: Vec::new(),
        arguments: None,
    };

    /// Get arguments associated with documentation.
    pub(crate) fn args(&self) -> &[String] {
        self.arguments.as_deref().unwrap_or_default()
    }

    /// Get lines of documentation.
    #[cfg(all(feature = "doc", any(feature = "languageserver", feature = "cli")))]
    pub(crate) fn lines(&self) -> &[String] {
        &self.docs
    }

    /// Update documentation.
    #[cfg(feature = "doc")]
    pub(crate) fn set_docs(
        &mut self,
        docs: impl IntoIterator<Item: AsRef<str>>,
    ) -> alloc::Result<()> {
        self.docs.clear();

        for line in docs {
            self.docs.try_push(line.as_ref().try_to_owned()?)?;
        }

        Ok(())
    }

    #[cfg(not(feature = "doc"))]
    pub(crate) fn set_docs(&mut self, _: impl IntoIterator<Item: AsRef<str>>) -> alloc::Result<()> {
        Ok(())
    }

    /// Update arguments (always available for type checking).
    pub(crate) fn set_arguments(
        &mut self,
        arguments: impl IntoIterator<Item: AsRef<str>>,
    ) -> alloc::Result<()> {
        let mut out = self.arguments.take().unwrap_or_default();
        out.clear();

        for argument in arguments {
            out.try_push(argument.as_ref().try_to_owned()?)?;
        }

        self.arguments = Some(out);
        Ok(())
    }
}

impl Default for Docs {
    #[inline]
    fn default() -> Self {
        Self::EMPTY
    }
}
