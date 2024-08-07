use crate as rune;
use crate::alloc;
use crate::alloc::prelude::*;
#[cfg(feature = "doc")]
use crate::alloc::{String, Vec};

/// The documentation for a function.
///
/// If the `doc` feature is disabled, this is a zero-sized type.
#[derive(Debug, TryClone)]
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

    /// Update arguments.
    #[cfg(feature = "doc")]
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

    #[cfg(not(feature = "doc"))]
    pub(crate) fn set_arguments(
        &mut self,
        _: impl IntoIterator<Item: AsRef<str>>,
    ) -> alloc::Result<()> {
        Ok(())
    }
}

impl Default for Docs {
    #[inline]
    fn default() -> Self {
        Self::EMPTY
    }
}
