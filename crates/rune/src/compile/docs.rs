/// The documentation for a function.
#[derive(Clone, Default)]
pub struct Docs {
    /// Lines of documentation.
    docs: Box<[String]>,
    /// Names of arguments.
    arguments: Option<Box<[String]>>,
}

impl Docs {
    /// Get arguments associated with documentation.
    pub fn args(&self) -> Option<&[String]> {
        self.arguments.as_ref().map(AsRef::as_ref)
    }

    /// Get lines of documentation.
    pub fn lines(&self) -> &[String] {
        &self.docs
    }

    /// Test if documentation is empty.
    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }

    /// Update documentation.
    pub(crate) fn set_docs<S>(&mut self, docs: S)
    where
        S: IntoIterator,
        S::Item: AsRef<str>,
    {
        let mut out = Vec::new();

        for line in docs {
            out.push(line.as_ref().to_owned());
        }

        self.docs = out.into();
    }

    /// Update arguments.
    pub(crate) fn set_arguments<S>(&mut self, arguments: S)
    where
        S: IntoIterator,
        S::Item: AsRef<str>,
    {
        let mut out = Vec::new();

        for argument in arguments {
            out.push(argument.as_ref().to_owned());
        }

        self.arguments = Some(out.into());
    }
}
