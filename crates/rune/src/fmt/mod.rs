//! Helper to format Rune code.

#[cfg(test)]
mod tests;

mod format;
mod output;

use core::fmt;

use crate::alloc;
use crate::alloc::prelude::*;
use crate::ast::{Delimiter, Kind, Span};
use crate::compile::{Error, ParseOptionError, Result, WithSpan};
use crate::grammar::{Node, Remaining, Stream, Tree};
use crate::{Diagnostics, Options, SourceId, Sources};

use self::output::Comments;
pub(crate) use self::output::Formatter;

const WS: &str = " ";
const NL: &str = "\n";
const NL_CHAR: char = '\n';
const TAB: &str = "\t";

#[derive(Debug)]
enum FormatErrorKind {
    Build,
    ParseOptionError(ParseOptionError),
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
            FormatErrorKind::ParseOptionError(error) => error.fmt(f),
            FormatErrorKind::Alloc(error) => error.fmt(f),
        }
    }
}

impl From<ParseOptionError> for FormatError {
    fn from(value: ParseOptionError) -> Self {
        Self {
            kind: FormatErrorKind::ParseOptionError(value),
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

impl core::error::Error for FormatError {}

/// Format the given source.
pub fn prepare(sources: &Sources) -> Prepare<'_> {
    Prepare {
        sources,
        options: None,
        diagnostics: None,
    }
}

/// A prepared formatting operation.
///
/// See [prepare].
pub struct Prepare<'a> {
    sources: &'a Sources,
    options: Option<&'a Options>,
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
        self.options = Some(options);
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

        let default_options;

        let options = match self.options {
            Some(options) => options,
            None => {
                default_options = Options::from_default_env()?;
                &default_options
            }
        };

        let mut files = Vec::new();

        for id in self.sources.source_ids() {
            let Some(source) = self.sources.get(id) else {
                continue;
            };

            let declaration = source.is_declaration();

            match layout_source_as(source.as_str(), id, declaration, options, diagnostics) {
                Ok(output) => {
                    files.try_push((id, output))?;
                }
                Err(error) => {
                    diagnostics.error(id, error)?;
                }
            }
        }

        if diagnostics.has_error() {
            return Err(FormatError {
                kind: FormatErrorKind::Build,
            });
        }

        Ok(files)
    }
}

/// Format the given source with the specified options.
#[cfg(test)]
pub(crate) fn layout_source_with(
    source: &str,
    source_id: SourceId,
    options: &Options,
    diagnostics: &mut Diagnostics,
) -> Result<String> {
    layout_source_as(source, source_id, false, options, diagnostics)
}

/// Format the given source with the specified options, as a declaration
/// source (`.d.rn`) if `declaration` is set.
pub(crate) fn layout_source_as(
    source: &str,
    source_id: SourceId,
    declaration: bool,
    options: &Options,
    diagnostics: &mut Diagnostics,
) -> Result<String> {
    let tree = crate::grammar::text(source_id, source)
        .declaration(declaration)
        .without_processing()
        .include_whitespace()
        .max_nesting(options.max_depth)
        .root()?;

    check_delimiters(&tree)?;

    #[cfg(feature = "std")]
    if options.print_tree {
        tree.print_with_source(
            &Span::empty(),
            format_args!("Formatting source #{source_id}"),
            source,
        )?;
    }

    let mut o = String::new();

    {
        let mut o = Formatter::new(
            Span::new(0, 0),
            source,
            source_id,
            &mut o,
            &options.fmt,
            diagnostics,
        );

        o.flush_prefix_comments(&tree)?;
        format::root(&mut o, &tree)?;
        o.comments(Comments::Line)?;
        o.trim_line_end();
    }

    if options.fmt.force_newline && !o.ends_with(NL) {
        o.try_push_str(NL)
            .with_span(Span::new(source.len(), source.len()))?;
    }

    Ok(o)
}

/// Require that every delimiter in the parsed source is matched.
///
/// The grammar recovers from a delimiter which is never closed by closing it
/// where the input ends, and the formatter writes out the delimiters a node
/// expects whether or not they were there. Formatting `(((` would then produce
/// `((()))`, which says something the source did not. So the source is refused
/// instead, the way the compiler refuses it.
fn check_delimiters(tree: &Tree) -> Result<()> {
    let mut stack = Vec::<(Delimiter, Span)>::new();

    for node in tree.walk().filter(|node| node.is_token()) {
        match node.kind() {
            Kind::Open(delimiter) if delimiter != Delimiter::Empty => {
                stack.try_push((delimiter, node.span()))?;
            }
            Kind::Close(delimiter) if delimiter != Delimiter::Empty => match stack.pop() {
                Some((open, _)) if open == delimiter => {}
                Some((open, span)) => {
                    return Err(Error::msg(
                        span.join(node.span()),
                        format_args!(
                            "Mismatched delimiter `{}`, expected `{}` to close `{}`",
                            delimiter.close(),
                            open.close(),
                            open.open()
                        ),
                    ));
                }
                None => {
                    return Err(Error::msg(
                        node.span(),
                        format_args!("Unexpected closing delimiter `{}`", delimiter.close()),
                    ));
                }
            },
            _ => {}
        }
    }

    if let Some((open, span)) = stack.pop() {
        return Err(Error::msg(
            span,
            format_args!("Unclosed delimiter `{}`", open.open()),
        ));
    }

    Ok(())
}
