use crate::error::ConfigurationError;

/// Compiler options.
pub struct Options {
    /// Perform link-time checks.
    pub(crate) link_checks: bool,
    /// Memoize the instance function in a loop.
    pub(crate) memoize_instance_fn: bool,
    /// Include debug information when compiling.
    pub(crate) debug_info: bool,
    /// Support (experimental) macros.
    pub(crate) macros: bool,
}

impl Options {
    /// Parse the given option.
    pub fn parse_option(&mut self, option: &str) -> Result<(), ConfigurationError> {
        let mut it = option.split('=');

        match it.next() {
            Some("link-checks") => {
                self.link_checks = it.next() != Some("false");
            }
            Some("memoize-instance-fn") => {
                self.memoize_instance_fn = it.next() != Some("false");
            }
            Some("debug-info") => {
                self.debug_info = it.next() != Some("false");
            }
            Some("macros") => {
                self.macros = it.next() != Some("false");
            }
            _ => {
                return Err(ConfigurationError::UnsupportedOptimizationOption {
                    option: option.to_owned(),
                });
            }
        }

        Ok(())
    }
}

impl Default for Options {
    fn default() -> Self {
        Self {
            link_checks: true,
            memoize_instance_fn: true,
            debug_info: true,
            macros: false,
        }
    }
}
