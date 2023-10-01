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
    /// Support (experimental) macros.
    pub(crate) macros: bool,
    /// Support (experimental) bytecode caching.
    pub(crate) bytecode: bool,
    /// Compile for and enable test features
    pub(crate) cfg_test: bool,
    /// Use the second version of the compiler in parallel.
    pub(crate) v2: bool,
    /// Build sources as function bodies.
    pub(crate) function_body: bool,
}

impl Options {
    /// Parse a compiler option. This is the function which parses the
    /// `<option>[=<value>]` syntax, which is used by among other things the
    /// Rune CLI with the `-O <option>[=<value>]` option.
    ///
    /// It can be used to consistenly parse a collection of options by other
    /// programs as well.
    pub fn parse_option(&mut self, option: &str) -> Result<(), ParseOptionError> {
        let mut it = option.split('=');

        match it.next() {
            Some("memoize-instance-fn") => {
                self.memoize_instance_fn = it.next() == Some("true");
            }
            Some("debug-info") => {
                self.debug_info = it.next() == Some("true");
            }
            Some("link-checks") => {
                self.link_checks = it.next() == Some("true");
            }
            Some("macros") => {
                self.macros = it.next() == Some("true");
            }
            Some("bytecode") => {
                self.bytecode = it.next() == Some("true");
            }
            Some("test") => {
                self.cfg_test = it.next() == Some("true");
            }
            Some("v2") => {
                self.v2 = it.next() == Some("true");
            }
            Some("function-body") => {
                self.function_body = it.next() == Some("true");
            }
            _ => {
                return Err(ParseOptionError {
                    option: option.into(),
                });
            }
        }

        Ok(())
    }

    /// Enable the test configuration flag
    pub fn test(&mut self, enabled: bool) {
        self.cfg_test = enabled;
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
        Self {
            link_checks: true,
            memoize_instance_fn: true,
            debug_info: true,
            macros: true,
            bytecode: false,
            cfg_test: false,
            v2: false,
            function_body: false,
        }
    }
}
