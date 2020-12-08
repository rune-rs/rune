use thiserror::Error;

/// Error when parsing configuration.
#[derive(Debug, Clone, Error)]
pub enum ConfigurationError {
    /// Tried to configure the compiler with an unsupported optimzation option.
    #[error("unsupported optimization option `{option}`")]
    UnsupportedOptimizationOption {
        /// The unsupported option.
        option: String,
    },
}

/// Compiler options.
#[derive(Debug, Clone, Copy)]
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
    pub bytecode: bool,

    /// Compile for and enable test features
    pub cfg_test: bool,
    /// Use the second version of the compiler in parallel.
    pub v2: bool,
}

impl Options {
    /// Parse a compiler option. This is the function which parses the
    /// `<option>[=<value>]` syntax, which is used by among other things the
    /// Rune CLI with the `-O <option>[=<value>]` option.
    ///
    /// It can be used to consistenly parse a collection of options by other
    /// programs as well.
    pub fn parse_option(&mut self, option: &str) -> Result<(), ConfigurationError> {
        let mut it = option.split('=');

        match it.next() {
            Some("memoize-instance-fn") => {
                self.memoize_instance_fn = it.next() != Some("false");
            }
            Some("debug-info") => {
                self.debug_info = it.next() != Some("false");
            }
            Some("link-checks") => {
                self.link_checks = it.next() != Some("false");
            }
            Some("macros") => {
                self.macros = it.next() != Some("false");
            }
            Some("bytecode") => {
                self.bytecode = it.next() != Some("false");
            }
            Some("test") => {
                self.cfg_test = it.next() != Some("false");
            }
            Some("v2") => {
                self.v2 = it.next() != Some("false");
            }
            _ => {
                return Err(ConfigurationError::UnsupportedOptimizationOption {
                    option: option.to_owned(),
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
        }
    }
}
