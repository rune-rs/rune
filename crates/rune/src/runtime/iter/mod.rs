#[macro_use]
mod macros;

mod runtime_iterator;
pub use self::runtime_iterator::RuntimeIterator;

mod traits;
pub use self::traits::{Iterator, DoubleEndedIterator};

mod empty;
pub use self::empty::Empty;

mod once;
pub use self::once::Once;

mod map;
pub use self::map::Map;

mod peekable;
pub use self::peekable::Peekable;

mod take;
pub use self::take::Take;

mod skip;
pub use self::skip::Skip;

mod rev;
pub use self::rev::Rev;

mod chain;
pub use self::chain::Chain;

mod enumerate;
pub use self::enumerate::Enumerate;

mod filter;
pub use self::filter::Filter;

mod fuse;
pub use self::fuse::Fuse;

mod flat_map;
pub use self::flat_map::FlatMap;

#[inline]
pub fn empty() -> Empty {
    Empty::new()
}

#[inline]
pub fn once(value: Value) -> Once {
    Once::new(value)
}
