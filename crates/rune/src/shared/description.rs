/// Helper trait to get description.
pub(crate) trait Description {
    /// Get the description for the thing.
    fn description(self) -> &'static str;
}

impl Description for &'static str {
    fn description(self) -> Self {
        self
    }
}
