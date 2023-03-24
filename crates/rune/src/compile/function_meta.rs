use std::sync::Arc;

use crate::compile::module::{AssocKind, AssocType, AsyncFunction, AsyncInstFn, Function, InstFn};
use crate::compile::{IntoComponent, ItemBuf};
use crate::runtime::FunctionHandler;
use crate::{InstFnInfo, InstFnName};

/// Type used to collect and store function metadata through the
/// `#[rune::function]` macro.
///
/// This is the argument type for
/// [`Module::function2`][crate::compile::Module::function2], and is from a
/// public API perspective completely opaque and might change for any release.
///
/// Calling and making use of `FunctionMeta` manually despite this warning might
/// lead to future breakage.
pub type FunctionMeta = fn() -> FunctionMetaData;

/// Runtime data for a function.
#[derive(Clone)]
pub struct FunctionData {
    pub(crate) name: ItemBuf,
    pub(crate) handler: Arc<FunctionHandler>,
    pub(crate) args: Option<usize>,
}

impl FunctionData {
    #[inline]
    pub(crate) fn new<Func, Args, N>(name: N, f: Func) -> Self
    where
        Func: Function<Args>,
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        Self {
            name: ItemBuf::with_item(name),
            handler: Arc::new(move |stack, args| f.fn_call(stack, args)),
            args: Some(Func::args()),
        }
    }

    #[inline]
    pub(crate) fn new_async<Func, Args, N>(name: N, f: Func) -> Self
    where
        Func: AsyncFunction<Args>,
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        Self {
            name: ItemBuf::with_item(name),
            handler: Arc::new(move |stack, args| f.fn_call(stack, args)),
            args: Some(Func::args()),
        }
    }
}

/// Runtime data for an associated function.
#[derive(Clone)]
pub struct AssocFnData {
    pub(crate) name: InstFnInfo,
    pub(crate) handler: Arc<FunctionHandler>,
    pub(crate) ty: AssocType,
    pub(crate) args: Option<usize>,
    pub(crate) kind: AssocKind,
}

impl AssocFnData {
    #[inline]
    pub(crate) fn new<Func, Args>(name: InstFnInfo, f: Func, kind: AssocKind) -> Self
    where
        Func: InstFn<Args>,
    {
        Self {
            name,
            handler: Arc::new(move |stack, args| f.fn_call(stack, args)),
            ty: Func::ty(),
            args: Some(Func::args()),
            kind,
        }
    }

    #[inline]
    pub(crate) fn new_async<Func, Args>(name: InstFnInfo, f: Func, kind: AssocKind) -> Self
    where
        Func: AsyncInstFn<Args>,
    {
        Self {
            name,
            handler: Arc::new(move |stack, args| f.fn_call(stack, args)),
            ty: Func::ty(),
            args: Some(Func::args()),
            kind,
        }
    }
}

/// The kind of a [`FunctionMeta`].
///
/// Even though this is marked as `pub`, this is private API. If you use this it
/// might cause breakage.
#[derive(Clone)]
#[doc(hidden)]
pub enum FunctionMetaKind {
    #[doc(hidden)]
    Function(FunctionData),
    #[doc(hidden)]
    AssocFn(AssocFnData),
}

impl FunctionMetaKind {
    #[doc(hidden)]
    #[inline]
    pub fn function<N, Func, Args>(name: N, f: Func) -> Self
    where
        N: IntoIterator,
        N::Item: IntoComponent,
        Func: Function<Args>,
    {
        Self::Function(FunctionData::new(name, f))
    }

    #[doc(hidden)]
    #[inline]
    pub fn async_function<N, Func, Args>(name: N, f: Func) -> Self
    where
        N: IntoIterator,
        N::Item: IntoComponent,
        Func: AsyncFunction<Args>,
    {
        Self::Function(FunctionData::new_async(name, f))
    }

    #[doc(hidden)]
    #[inline]
    pub fn instance<N, Func, Args>(name: N, f: Func) -> Self
    where
        N: InstFnName,
        Func: InstFn<Args>,
    {
        Self::AssocFn(AssocFnData::new(name.info(), f, AssocKind::Instance))
    }

    #[doc(hidden)]
    #[inline]
    pub fn async_instance<N, Func, Args>(name: N, f: Func) -> Self
    where
        N: InstFnName,
        Func: AsyncInstFn<Args>,
    {
        Self::AssocFn(AssocFnData::new_async(name.info(), f, AssocKind::Instance))
    }
}

/// The data of a [`FunctionMeta`].
///
/// Even though this is marked as `pub`, this is private API. If you use this it
/// might cause breakage.
#[doc(hidden)]
#[derive(Clone)]
pub struct FunctionMetaData {
    #[doc(hidden)]
    pub kind: FunctionMetaKind,
    #[doc(hidden)]
    pub name: &'static str,
    #[doc(hidden)]
    pub docs: &'static [&'static str],
    #[doc(hidden)]
    pub arguments: &'static [&'static str],
}
