use core::fmt;

use ::rust_alloc::boxed::Box;

/// Error raised when trying to parse an invalid option.
#[derive(Debug, Clone)]
pub struct ParseOptionError {
    option: Box<str>,
}

impl fmt::Display for ParseOptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unsupported compile option `{}`", self.option)
    }
}

cfg_std! {
    impl std::error::Error for ParseOptionError {}
}

/// Options specific to formatting.
#[derive(Debug, Clone)]
pub(crate) struct FmtOptions {
    /// Print source tree.
    pub(crate) print_tree: bool,
    /// Attempt to format even when faced with syntax errors.
    pub(crate) error_recovery: bool,
    /// Force newline at end of document.
    pub(crate) force_newline: bool,
}

impl FmtOptions {
    /// The default format option.
    pub(crate) const DEFAULT: Self = Self {
        print_tree: false,
        error_recovery: false,
        force_newline: true,
    };

    /// Parse a rune-fmt option.
    pub(crate) fn parse_option(&mut self, option: &str) -> Result<(), ParseOptionError> {
        let (head, tail) = if let Some((head, tail)) = option.trim().split_once('=') {
            (head.trim(), Some(tail.trim()))
        } else {
            (option.trim(), None)
        };

        match head {
            "print-tree" => {
                self.print_tree = tail.map_or(true, |s| s == "true");
            }
            "error-recovery" => {
                self.error_recovery = tail.map_or(true, |s| s == "true");
            }
            "force-newline" => {
                self.force_newline = tail.map_or(true, |s| s == "true");
            }
            _ => {
                return Err(ParseOptionError {
                    option: option.into(),
                });
            }
        }

        Ok(())
    }
}

impl Default for FmtOptions {
    #[inline]
    fn default() -> Self {
        FmtOptions::DEFAULT
    }
}

/// Documentation for a single compiler option.
#[non_exhaustive]
pub struct OptionMeta {
    /// The key.
    pub key: &'static str,
    /// Whether the option is unstable or not.
    pub unstable: bool,
    /// The documentation for the option.
    pub doc: &'static [&'static str],
    /// The default value for the option.
    pub default: &'static str,
    /// Available options.
    pub options: &'static str,
}

/// Options that can be provided to the compiler.
///
/// See [Build::with_options][crate::Build::with_options].
#[derive(Debug, Clone)]
pub struct Options {
    /// Perform link-time checks.
    pub(crate) link_checks: bool,
    /// Memoize the instance function in a loop.
    pub(crate) memoize_instance_fn: bool,
    /// Include debug information when compiling.
    pub(crate) debug_info: bool,
    /// Support macros.
    pub(crate) macros: bool,
    /// Support bytecode caching.
    pub(crate) bytecode: bool,
    /// Build sources as function bodies.
    ///
    /// The function to run will be named 0, which can be constructed with
    /// `Hash::EMPTY`.
    pub(crate) function_body: bool,
    /// When running tests, include std tests.
    pub(crate) test_std: bool,
    /// Enable lowering optimizations.
    pub(crate) lowering: u8,
    /// Rune format options.
    pub(crate) fmt: FmtOptions,
}

impl Options {
    /// The default options.
    pub(crate) const DEFAULT: Options = Options {
        link_checks: true,
        memoize_instance_fn: true,
        debug_info: true,
        macros: true,
        bytecode: false,
        function_body: false,
        test_std: false,
        lowering: 0,
        fmt: FmtOptions::DEFAULT,
    };

    /// Get a list and documentation for all available compiler options.
    pub fn available() -> &'static [OptionMeta] {
        static BOOL: &str = "true, false";
        static VALUES: &[OptionMeta] = &[
            OptionMeta {
                key: "link-checks",
                unstable: false,
                doc: &docstring! {
                    /// Perform link-time checks to ensure that
                    /// function hashes which are referenced during
                    /// compilation exist.
                },
                default: "true",
                options: BOOL,
            },
            OptionMeta {
                key: "memoize-instance-fn",
                unstable: false,
                doc: &docstring! {
                    /// Memoize the instance function in a loop.
                },
                default: "true",
                options: BOOL,
            },
            OptionMeta {
                key: "debug-info",
                unstable: false,
                doc: &docstring! {
                    /// Include debug information when compiling.
                    ///
                    /// This provides better diagnostics, but also
                    /// increases memory usage.
                },
                default: "true",
                options: BOOL,
            },
            OptionMeta {
                key: "macros",
                unstable: false,
                doc: &docstring! {
                    /// Support macro expansion.
                },
                default: "true",
                options: BOOL,
            },
            OptionMeta {
                key: "bytecode",
                unstable: true,
                doc: &docstring! {
                    /// Make use of bytecode, which might make
                    /// compilation units smaller.
                },
                default: "false",
                options: BOOL,
            },
            OptionMeta {
                key: "function-body",
                unstable: true,
                doc: &docstring! {
                    /// Causes sources to be treated as-if they were
                    /// function bodies, rather than modules.
                },
                default: "false",
                options: BOOL,
            },
            OptionMeta {
                key: "test-std",
                unstable: true,
                doc: &docstring! {
                    /// When running tests, includes tests found in the
                    /// standard library.
                },
                default: "false",
                options: BOOL,
            },
            OptionMeta {
                key: "lowering",
                unstable: true,
                doc: &docstring! {
                    /// Enable lowering optimizations.
                    ///
                    /// Supports a value of 0-3 with increasingly higher
                    /// levels of optimizations applied.
                    ///
                    /// Enabling a higher level results in better code
                    /// generation, but contributes to compilation times.
                },
                default: "0",
                options: "0-3",
            },
            OptionMeta {
                key: "fmt.print-tree",
                unstable: false,
                doc: &docstring! {
                    /// Print the parsed source tree when formatting to
                    /// standard output.
                },
                default: "false",
                options: BOOL,
            },
            OptionMeta {
                key: "fmt.error-recovery",
                unstable: true,
                doc: &docstring! {
                    /// Perform error recovery when formatting.
                    ///
                    /// This allows code to be formatted even if it
                    /// contains invalid syntax.
                },
                default: "false",
                options: BOOL,
            },
            OptionMeta {
                key: "fmt.force-newline",
                unstable: true,
                doc: &docstring! {
                    /// Force newline at end of document.
                },
                default: "true",
                options: BOOL,
            },
        ];

        VALUES
    }

    /// Parse a compiler option. This is the function which parses the
    /// `<option>[=<value>]` syntax, which is used by among other things the
    /// Rune CLI with the `-O <option>[=<value>]` option.
    ///
    /// It can be used to consistenly parse a collection of options by other
    /// programs as well.
    pub fn parse_option(&mut self, option: &str) -> Result<(), ParseOptionError> {
        let (head, tail) = if let Some((head, tail)) = option.trim().split_once('=') {
            (head.trim(), Some(tail.trim()))
        } else {
            (option.trim(), None)
        };

        match head {
            "memoize-instance-fn" => {
                self.memoize_instance_fn = tail.map_or(true, |s| s == "true");
            }
            "debug-info" => {
                self.debug_info = tail.map_or(true, |s| s == "true");
            }
            "link-checks" => {
                self.link_checks = tail.map_or(true, |s| s == "true");
            }
            "macros" => {
                self.macros = tail.map_or(true, |s| s == "true");
            }
            "bytecode" => {
                self.bytecode = tail.map_or(true, |s| s == "true");
            }
            "function-body" => {
                self.function_body = tail.map_or(true, |s| s == "true");
            }
            "test-std" => {
                self.test_std = tail.map_or(true, |s| s == "true");
            }
            "lowering" => {
                self.lowering = match tail {
                    Some("0") | None => 0,
                    Some("1") => 1,
                    _ => {
                        return Err(ParseOptionError {
                            option: option.into(),
                        })
                    }
                };
            }
            other => {
                let Some((head, tail)) = other.split_once('.') else {
                    return Err(ParseOptionError {
                        option: option.into(),
                    });
                };

                let head = head.trim();
                let tail = tail.trim();

                match head {
                    "fmt" => {
                        self.fmt.parse_option(tail)?;
                    }
                    _ => {
                        return Err(ParseOptionError {
                            option: option.into(),
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Enable the test configuration flag.
    pub fn test(&mut self, _enabled: bool) {
        // ignored
    }

    /// Set if debug info is enabled or not. Defaults to `true`.
    pub fn debug_info(&mut self, enabled: bool) {
        self.debug_info = enabled;
    }

    /// Set if link checks are enabled or not. Defaults to `true`. This will
    /// cause compilation to fail if an instruction references a function which
    /// does not exist.
    pub fn link_checks(&mut self, enabled: bool) {
        self.link_checks = enabled;
    }

    /// Set if macros are enabled or not. Defaults to `false`.
    pub fn macros(&mut self, enabled: bool) {
        self.macros = enabled;
    }

    /// Set if bytecode caching is enabled or not. Defaults to `false`.
    pub fn bytecode(&mut self, enabled: bool) {
        self.bytecode = enabled;
    }

    /// Memoize the instance function in a loop. Defaults to `false`.
    pub fn memoize_instance_fn(&mut self, enabled: bool) {
        self.memoize_instance_fn = enabled;
    }
}

impl Default for Options {
    fn default() -> Self {
        Options::DEFAULT
    }
}
