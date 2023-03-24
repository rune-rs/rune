//! FFI bindings for Rune.

#![allow(non_camel_case_types)]

macro_rules! test_size {
    ($ty:ty, $rune_ty:ty) => {
        // Compile-time assertion of the size of the value.
        #[cfg(not(test))]
        const _: () = assert!(
            ::std::mem::size_of::<$ty>() == ::std::mem::size_of::<$rune_ty>(),
            "rune value size must match"
        );

        #[test]
        fn test_size() {
            assert_eq!(
                ::std::mem::size_of::<$ty>(),
                ::std::mem::size_of::<$rune_ty>()
            );
        }
    };
}

mod types;
pub use self::types::*;

mod build;
pub use self::build::*;

mod context;
pub use self::context::*;

mod context_error;
pub use self::context_error::*;

mod diagnostics;
pub use self::diagnostics::*;

mod hash;
pub use self::hash::*;

mod source;
pub use self::source::*;

mod sources;
pub use self::sources::*;

mod standard_stream;
pub use self::standard_stream::*;

mod stack;
pub use self::stack::*;

mod unit;
pub use self::unit::*;

mod module;
pub use self::module::*;

mod runtime_context;
pub use self::runtime_context::*;

mod value;
pub use self::value::*;

mod vm;
pub use self::vm::*;

mod vm_error;
pub use self::vm_error::*;
