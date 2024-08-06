//! Helper to format Rune code.

#[cfg(test)]
mod tests;

macro_rules! object_key {
    () => {
        K![ident] | K![str]
    };
}

mod format;
mod output;

use core::fmt;

use crate::alloc;
use crate::alloc::prelude::*;
use crate::ast::Span;
use crate::compile::{FmtOptions, Result, WithSpan};
use crate::grammar::{Node, Parser, Remaining, Stream, Tree};
use crate::{Diagnostics, Options, SourceId, Sources};

use self::output::Comments;
pub(crate) use self::output::Output;

const WS: &str = " ";
const NL: &str = "\n";
const NL_CHAR: char = '\n';
const INDENT: &str = "    ";

#[derive(Debug)]
enum FormatErrorKind {
    Build,
    Alloc(alloc::Error),
}

/// Error during formatting.
#[non_exhaustive]
pub struct FormatError {
    kind: FormatErrorKind,
}

impl fmt::Debug for FormatError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl fmt::Display for FormatError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.kind {
            FormatErrorKind::Build => write!(f, "Failed to format source"),
            FormatErrorKind::Alloc(error) => error.fmt(f),
        }
    }
}

impl From<alloc::Error> for FormatError {
    fn from(value: alloc::Error) -> Self {
        Self {
            kind: FormatErrorKind::Alloc(value),
        }
    }
}

cfg_std! {
    impl std::error::Error for FormatError {
    }
}

/// Format the given source.
pub fn prepare(sources: &Sources) -> Prepare<'_> {
    static OPTIONS: Options = Options::DEFAULT;

    Prepare {
        sources,
        options: &OPTIONS,
        diagnostics: None,
    }
}

/// A prepared formatting operation.
///
/// See [prepare].
pub struct Prepare<'a> {
    sources: &'a Sources,
    options: &'a Options,
    diagnostics: Option<&'a mut Diagnostics>,
}

impl<'a> Prepare<'a> {
    /// Associate diagnostics with the build.
    pub fn with_diagnostics(mut self, diagnostics: &'a mut Diagnostics) -> Self {
        self.diagnostics = Some(diagnostics);
        self
    }

    /// Associate options with the build.
    pub fn with_options(mut self, options: &'a Options) -> Self {
        self.options = options;
        self
    }

    /// Format the given sources.
    pub fn format(self) -> Result<Vec<(SourceId, String)>, FormatError> {
        let mut local;

        let diagnostics = match self.diagnostics {
            Some(diagnostics) => diagnostics,
            None => {
                local = Diagnostics::new();
                &mut local
            }
        };

        let options = self.options;
        let mut files = Vec::new();

        let mut has_errors = false;

        for id in self.sources.source_ids() {
            let Some(source) = self.sources.get(id) else {
                continue;
            };

            match layout_source_with(source.as_str(), &options.fmt) {
                Ok(output) => {
                    files.try_push((id, output))?;
                }
                Err(error) => {
                    has_errors = true;
                    diagnostics.error(id, error)?;
                }
            }
        }

        if has_errors {
            return Err(FormatError {
                kind: FormatErrorKind::Build,
            });
        }

        Ok(files)
    }
}

/// Format the given source with the specified options.
pub(crate) fn layout_source_with(source: &str, options: &FmtOptions) -> Result<String> {
    let mut p = Parser::new(source);
    crate::grammar::root(&mut p)?;

    let tree = p.build()?;

    if options.print_tree {
        let o = std::io::stdout();
        let mut o = o.lock();
        tree.print_with_source(&mut o, source)?;
    }

    let mut o = String::new();

    {
        let mut o = Output::new(Span::new(0, 0), source, &mut o, options);
        o.flush_prefix_comments(&tree)?;
        format::root(&mut o, &tree)?;
        o.comments(Comments::Line)?;
    }

    if options.force_newline && !o.ends_with(NL) {
        o.try_push_str(NL)
            .with_span(Span::new(source.len(), source.len()))?;
    }

    Ok(o)
}
