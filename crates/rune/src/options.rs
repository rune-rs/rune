use crate::error::ConfigurationError;

/// Compiler options.
pub struct Options {
    /// Memoize the instance function in a loop.
    pub(super) memoize_instance_fn: bool,
}

impl Options {
    /// Parse the given option.
    pub fn parse_option(&mut self, option: &str) -> Result<(), ConfigurationError> {
        let mut it = option.split('=');

        match it.next() {
            Some("memoize-instance-fn") => {
                self.memoize_instance_fn = it.next() != Some("false");
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
            memoize_instance_fn: true,
        }
    }
}
