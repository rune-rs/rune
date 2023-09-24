pub mod crash_test;
pub mod ord_chaos;
pub mod rng;

use core::convert::Infallible;
use core::fmt;

use crate::alloc::AllocError;
use crate::error::{CustomError, Error};

pub(crate) trait CustomTestExt<T, E> {
    fn custom_result(self) -> Result<T, E>;
}

impl<T, E> CustomTestExt<T, E> for Result<T, CustomError<E>> {
    fn custom_result(self) -> Result<T, E> {
        match self {
            Ok(value) => Ok(value),
            Err(CustomError::Custom(error)) => Err(error),
            Err(CustomError::Error(error)) => handle_error(error),
        }
    }
}

pub(crate) trait TestExt<T> {
    fn abort(self) -> T;
}

impl<T> TestExt<T> for Result<T, Infallible> {
    fn abort(self) -> T {
        match self {
            Ok(value) => value,
            Err(error) => match error {},
        }
    }
}

impl<T> TestExt<T> for Result<T, Error> {
    fn abort(self) -> T {
        match self {
            Ok(value) => value,
            Err(error) => handle_error(error),
        }
    }
}

impl<T> TestExt<T> for Result<T, AllocError> {
    fn abort(self) -> T {
        match self {
            Ok(value) => value,
            Err(error) => ::rust_alloc::alloc::handle_alloc_error(error.layout),
        }
    }
}

impl<T, E> TestExt<T> for Result<T, CustomError<E>>
where
    E: fmt::Display,
{
    fn abort(self) -> T {
        match self {
            Ok(value) => value,
            Err(error) => match error {
                CustomError::Custom(error) => {
                    panic!("{}", error)
                }
                CustomError::Error(error) => handle_error(error),
            },
        }
    }
}

fn handle_error(error: Error) -> ! {
    match error {
        Error::AllocError { error } => ::rust_alloc::alloc::handle_alloc_error(error.layout),
        error => {
            panic!("{}", error)
        }
    }
}
