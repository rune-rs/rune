/// Allocator indirection to simplify lifetime management.
#[rustfmt::skip]
macro_rules! alloc_with {
    ($cx:expr, $span:expr) => {
        #[allow(unused)]
        macro_rules! alloc {
            ($value:expr) => {
                $cx.arena.alloc($value).map_err(|e| {
                    compile::Error::new(
                        $span,
                        ErrorKind::ArenaAllocError {
                            requested: e.requested,
                        },
                    )
                })?
            };
        }

        #[allow(unused)]
        macro_rules! option {
            ($value:expr, |$pat:pat_param| $closure:expr) => {
                match $value {
                    Some($pat) => {
                        Some(&*alloc!($closure))
                    }
                    None => {
                        None
                    }
                }
            };
        }

        #[allow(unused)]
        macro_rules! iter {
            ($iter:expr) => {
                iter!($iter, |value| value)
            };

            ($iter:expr, |$pat:pat_param| $closure:expr) => {
                iter!($iter, it, ExactSizeIterator::len(&it), |$pat| $closure)
            };

            ($iter:expr, $len:expr, |$pat:pat_param| $closure:expr) => {
                iter!($iter, it, $len, |$pat| $closure)
            };

            ($iter:expr, $it:ident, $len:expr, |$pat:pat_param| $closure:expr) => {{
                let mut $it = IntoIterator::into_iter($iter);

                let mut writer = match $cx.arena.alloc_iter($len) {
                    Ok(writer) => writer,
                    Err(e) => {
                        return Err(compile::Error::new(
                            $span,
                            ErrorKind::ArenaAllocError {
                                requested: e.requested,
                            },
                        ));
                    }
                };
        
                while let Some($pat) = $it.next() {
                    if let Err(e) = writer.write($closure) {
                        return Err(compile::Error::new(
                            $span,
                            ErrorKind::ArenaWriteSliceOutOfBounds { index: e.index },
                        ));
                    }
                }

                writer.finish()
            }};
        }

        #[allow(unused)]
        macro_rules! alloc_str {
            ($value:expr) => {
                match $cx.arena.alloc_str($value) {
                    Ok(string) => string,
                    Err(e) => return Err(compile::Error::new(
                        $span,
                        ErrorKind::ArenaAllocError {
                            requested: e.requested,
                        },
                    )),
                }
            };
        }

        #[allow(unused)]
        macro_rules! alloc_bytes {
            ($value:expr) => {
                match $cx.arena.alloc_bytes($value) {
                    Ok(bytes) => bytes,
                    Err(e) => return Err(compile::Error::new(
                        $span,
                        ErrorKind::ArenaAllocError {
                            requested: e.requested,
                        },
                    )),
                }
            };
        }
    };
}
