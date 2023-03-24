use std::ffi::c_void;
#[repr(C)]
pub struct StaticType {
    pub(crate) inner: *const c_void,
}

unsafe impl Sync for StaticType {}

macro_rules! decl {
    ($name:ident, $type:ident, $doc:literal) => {
        /// Handle for the integer type.
        #[no_mangle]
        #[doc = $doc]
        pub static $name: StaticType = StaticType {
            inner: rune::runtime::$type as *const _ as *const c_void,
        };
    };
}

decl!(
    RUNE_BOOL_TYPE,
    BOOL_TYPE,
    "The specialized type information for a bool type."
);
decl!(
    RUNE_BYTES_TYPE,
    BYTES_TYPE,
    "The specialized type information for a bytes type."
);
decl!(
    RUNE_BYTE_TYPE,
    BYTE_TYPE,
    "The specialized type information for a byte type."
);
decl!(
    RUNE_CHAR_TYPE,
    CHAR_TYPE,
    "The specialized type information for a char type."
);
decl!(
    RUNE_FLOAT_TYPE,
    FLOAT_TYPE,
    "The specialized type information for a float type."
);
decl!(
    RUNE_FORMAT_TYPE,
    FORMAT_TYPE,
    "The specialized type information for a fmt spec types."
);
decl!(
    RUNE_FUNCTION_TYPE,
    FUNCTION_TYPE,
    "The specialized type information for a function pointer type."
);
decl!(
    RUNE_FUTURE_TYPE,
    FUTURE_TYPE,
    "The specialized type information for a future type."
);
decl!(
    RUNE_GENERATOR_STATE_TYPE,
    GENERATOR_STATE_TYPE,
    "The specialized type information for a generator state type."
);
decl!(
    RUNE_GENERATOR_TYPE,
    GENERATOR_TYPE,
    "The specialized type information for a generator type."
);
decl!(
    RUNE_INTEGER_TYPE,
    INTEGER_TYPE,
    "The specialized type information for a integer type."
);
decl!(
    RUNE_ITERATOR_TYPE,
    ITERATOR_TYPE,
    "The specialized type information for the iterator type."
);
decl!(
    RUNE_OBJECT_TYPE,
    OBJECT_TYPE,
    "The specialized type information for an anonymous object type."
);
decl!(
    RUNE_OPTION_TYPE,
    OPTION_TYPE,
    "The specialized type information for a option type."
);
decl!(
    RUNE_RANGE_TYPE,
    RANGE_TYPE,
    "The specialized type information for the range type."
);
decl!(
    RUNE_RESULT_TYPE,
    RESULT_TYPE,
    "The specialized type information for a result type."
);
decl!(
    RUNE_STREAM_TYPE,
    STREAM_TYPE,
    "The specialized type information for the Stream type."
);
decl!(
    RUNE_STRING_TYPE,
    STRING_TYPE,
    "The specialized type information for a string type."
);
decl!(
    RUNE_TUPLE_TYPE,
    TUPLE_TYPE,
    "The specialized type information for an anonymous tuple type."
);
decl!(
    RUNE_TYPE,
    TYPE,
    "The specialized type information for type objects."
);
decl!(
    RUNE_UNIT_TYPE,
    UNIT_TYPE,
    "The specialized type information for a unit."
);
decl!(
    RUNE_VEC_TYPE,
    VEC_TYPE,
    "The specialized type information for a vector type."
);
