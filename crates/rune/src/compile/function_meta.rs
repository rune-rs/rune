use std::fmt;
use std::sync::Arc;

use crate::compile::module::{
    AssocKey, AssocKind, AssocType, AsyncFunction, AsyncInstFn, Function, InstFn,
};
use crate::compile::{IntoComponent, ItemBuf, Named};
use crate::hash::{Hash, IntoHash, Params};
use crate::runtime::{FunctionHandler, Protocol};

mod sealed {
    use crate::hash::Params;
    use crate::runtime::Protocol;

    pub trait Sealed {}

    impl Sealed for Protocol {}
    impl Sealed for &str {}
    impl<T, P> Sealed for Params<T, P> {}
}

/// Type used to collect and store function metadata through the
/// `#[rune::function]` macro.
///
/// This is the argument type for
/// [`Module::function_meta`][crate::compile::Module::function_meta], and is from a
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

/// An instance function name.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum InstFnKind {
    /// The instance function refers to the given protocol.
    Protocol(Protocol),
    /// The instance function refers to the given named instance fn.
    Instance(Box<str>),
}

impl fmt::Display for InstFnKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstFnKind::Protocol(protocol) => write!(f, "<{}>", protocol.name),
            InstFnKind::Instance(name) => write!(f, "{}", name),
        }
    }
}

/// Trait used to determine what can be used as an instance function name.
pub trait InstFnName: self::sealed::Sealed {
    /// Get information on the naming of the instance function.
    #[doc(hidden)]
    fn info(self) -> InstFnInfo;
}

impl InstFnName for &str {
    #[inline]
    fn info(self) -> InstFnInfo {
        InstFnInfo {
            hash: self.into_hash(),
            kind: InstFnKind::Instance(self.into()),
            parameters: Hash::EMPTY,
        }
    }
}

impl<T, P> InstFnName for Params<T, P>
where
    T: InstFnName,
    P: IntoIterator,
    P::Item: std::hash::Hash,
{
    fn info(self) -> InstFnInfo {
        let info = self.name.info();

        InstFnInfo {
            hash: info.hash,
            kind: info.kind,
            parameters: Hash::parameters(self.parameters),
        }
    }
}

/// A descriptor for an instance function.
#[derive(Clone)]
#[non_exhaustive]
#[doc(hidden)]
pub struct InstFnInfo {
    /// The hash of the instance function.
    pub hash: Hash,
    /// The name of the instance function.
    pub kind: InstFnKind,
    /// Parameters hash.
    pub parameters: Hash,
}

impl InstFnInfo {
    pub(crate) fn index(protocol: Protocol, index: usize) -> Self {
        Self {
            hash: Hash::index(index),
            kind: InstFnKind::Protocol(protocol),
            parameters: Hash::EMPTY,
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

    /// Get associated key.
    pub(crate) fn assoc_key(&self) -> AssocKey {
        AssocKey {
            type_hash: self.ty.hash,
            hash: self.name.hash,
            kind: self.kind,
            parameters: self.name.parameters,
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
    pub fn function_with<T, N, Func, Args>(name: N, f: Func) -> Self
    where
        T: Named,
        N: IntoIterator,
        N::Item: IntoComponent,
        Func: Function<Args>,
    {
        let name = [IntoComponent::into_component(T::BASE_NAME)]
            .into_iter()
            .chain(name.into_iter().map(IntoComponent::into_component));
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
    pub fn async_function_with<T, N, Func, Args>(name: N, f: Func) -> Self
    where
        T: Named,
        N: IntoIterator,
        N::Item: IntoComponent,
        Func: AsyncFunction<Args>,
    {
        let name = [IntoComponent::into_component(T::BASE_NAME)]
            .into_iter()
            .chain(name.into_iter().map(IntoComponent::into_component));
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
