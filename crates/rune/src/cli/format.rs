use anyhow::{Context, Result};
use clap::Parser;
use codespan_reporting::term::termcolor::WriteColor;
use std::io::Write;
use std::path::PathBuf;

use crate::cli::{ExitCode, Io, SharedFlags};
use crate::Source;

#[derive(Parser, Debug, Clone)]
pub(super) struct Flags {
    /// Exit with a non-zero exit-code even for warnings
    #[arg(long)]
    warnings_are_errors: bool,

    #[command(flatten)]
    pub(super) shared: SharedFlags,

    #[arg(long)]
    check: bool,
}

pub(super) fn run(io: &mut Io<'_>, paths: &[PathBuf], flags: &Flags) -> Result<ExitCode> {
    let mut red = codespan_reporting::term::termcolor::ColorSpec::new();
    red.set_fg(Some(codespan_reporting::term::termcolor::Color::Red));

    let mut green = codespan_reporting::term::termcolor::ColorSpec::new();
    green.set_fg(Some(codespan_reporting::term::termcolor::Color::Green));

    let mut yellow = codespan_reporting::term::termcolor::ColorSpec::new();
    yellow.set_fg(Some(codespan_reporting::term::termcolor::Color::Yellow));

    let mut succeeded = 0;
    let mut failed = 0;
    let mut unchanged = 0;
    for path in paths {
        let source =
            Source::from_path(path).with_context(|| format!("reading file: {}", path.display()))?;

        match crate::fmt::layout_source(&source) {
            Ok(val) => {
                if val == source.as_str() {
                    io.stdout.set_color(&yellow)?;
                    write!(io.stdout, "== ")?;
                    io.stdout.reset()?;
                    writeln!(io.stdout, "{}", path.display())?;

                    unchanged += 1;
                } else {
                    succeeded += 1;
                    io.stdout.set_color(&green)?;
                    write!(io.stdout, "++ ")?;
                    io.stdout.reset()?;
                    writeln!(io.stdout, "{}", path.display())?;
                    if !flags.check {
                        std::fs::write(path, &val)?;
                    }
                }
            }
            Err(err) => {
                failed += 1;
                io.stdout.set_color(&red)?;
                write!(io.stdout, "!! ")?;
                io.stdout.reset()?;
                writeln!(io.stdout, "{}: {}", path.display(), err)?;
            }
        }
    }

    io.stdout.set_color(&yellow)?;
    write!(io.stdout, "{}", unchanged)?;
    io.stdout.reset()?;
    writeln!(io.stdout, " unchanged")?;
    io.stdout.set_color(&green)?;
    write!(io.stdout, "{}", succeeded)?;
    io.stdout.reset()?;
    writeln!(io.stdout, " succeeded")?;
    io.stdout.set_color(&red)?;
    write!(io.stdout, "{}", failed)?;
    io.stdout.reset()?;
    writeln!(io.stdout, " failed")?;

    if flags.check && succeeded > 0 {
        return Ok(ExitCode::Failure);
    }

    if failed > 0 {
        return Ok(ExitCode::Failure);
    }

    Ok(ExitCode::Success)
}
