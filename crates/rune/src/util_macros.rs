/// A peek helper macro.
macro_rules! peek {
    ($expr:expr) => {
        peek!($expr, false)
    };

    ($expr:expr, $default:expr) => {
        match $expr {
            Some(value) => value,
            None => return $default,
        }
    };
}
