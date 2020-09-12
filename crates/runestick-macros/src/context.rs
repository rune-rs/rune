use crate::internals::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::Meta::*;
use syn::NestedMeta::*;
use syn::{Lit, MetaNameValue};

/// Parsed field attributes.
#[derive(Default)]
pub(crate) struct FieldAttrs {}

/// Parsed field attributes.
#[derive(Default)]
pub(crate) struct DeriveAttrs {
    /// `#[rune(name = "TypeName")]` to override the default type name.
    pub(crate) name: Option<syn::LitStr>,
}

pub(crate) struct Context {
    pub(crate) errors: Vec<syn::Error>,
    pub(crate) value: TokenStream,
    pub(crate) vm_error: TokenStream,
    pub(crate) vm_error_kind: TokenStream,
    pub(crate) object: TokenStream,
    pub(crate) tuple: TokenStream,
    pub(crate) from_value: TokenStream,
    pub(crate) to_value: TokenStream,
    pub(crate) any: TokenStream,
    pub(crate) named: TokenStream,
    pub(crate) raw_into_mut: TokenStream,
    pub(crate) raw_into_ref: TokenStream,
    pub(crate) shared: TokenStream,
    pub(crate) pointer_guard: TokenStream,
    pub(crate) ty: TokenStream,
    pub(crate) hash: TokenStream,
    pub(crate) type_info: TokenStream,
    pub(crate) unsafe_from_value: TokenStream,
    pub(crate) unsafe_to_value: TokenStream,
    pub(crate) type_of: TokenStream,
    pub(crate) raw_str: TokenStream,
}

impl Context {
    /// Construct a new context.
    pub fn new() -> Self {
        Self::with_module(RUNESTICK)
    }

    /// Construct a new context.
    pub fn with_module<M>(module: M) -> Self
    where
        M: Copy + ToTokens,
    {
        Self {
            errors: Vec::new(),
            value: quote!(#module::Value),
            vm_error: quote!(#module::VmError),
            vm_error_kind: quote!(#module::VmErrorKind),
            object: quote!(#module::Object),
            tuple: quote!(#module::Tuple),
            from_value: quote!(#module::FromValue),
            to_value: quote!(#module::ToValue),
            any: quote!(#module::Any),
            named: quote!(#module::Named),
            raw_into_mut: quote!(#module::RawMut),
            raw_into_ref: quote!(#module::RawRef),
            shared: quote!(#module::Shared),
            pointer_guard: quote!(#module::SharedPointerGuard),
            ty: quote!(#module::Type),
            hash: quote!(#module::Hash),
            type_info: quote!(#module::TypeInfo),
            unsafe_from_value: quote!(#module::UnsafeFromValue),
            unsafe_to_value: quote!(#module::UnsafeToValue),
            type_of: quote!(#module::TypeOf),
            raw_str: quote!(#module::RawStr),
        }
    }

    /// Parse the toplevel component of the attribute, which must be `#[rune(..)]`.
    pub fn get_rune_meta_items(&mut self, attr: &syn::Attribute) -> Option<Vec<syn::NestedMeta>> {
        if attr.path != RUNE {
            return Some(Vec::new());
        }

        match attr.parse_meta() {
            Ok(List(meta)) => Some(meta.nested.into_iter().collect()),
            Ok(other) => {
                self.errors
                    .push(syn::Error::new_spanned(other, "expected #[rune(...)]"));
                None
            }
            Err(error) => {
                self.errors.push(syn::Error::new(Span::call_site(), error));
                None
            }
        }
    }

    /// Parse field attributes.
    pub(crate) fn parse_field_attrs(&mut self, attrs: &[syn::Attribute]) -> Option<FieldAttrs> {
        let output = FieldAttrs::default();

        for attr in attrs {
            #[allow(clippy::never_loop)] // I guess this is on purpose?
            for meta in self.get_rune_meta_items(attr)? {
                match meta {
                    meta => {
                        self.errors
                            .push(syn::Error::new_spanned(meta, "unsupported attribute"));

                        return None;
                    }
                }
            }
        }

        Some(output)
    }

    /// Parse field attributes.
    pub(crate) fn parse_derive_attrs(&mut self, attrs: &[syn::Attribute]) -> Option<DeriveAttrs> {
        let mut output = DeriveAttrs::default();

        for attr in attrs {
            for meta in self.get_rune_meta_items(attr)? {
                match meta {
                    Meta(NameValue(MetaNameValue {
                        path,
                        lit: Lit::Str(name),
                        ..
                    })) if path == NAME => {
                        output.name = Some(name);
                    }
                    meta => {
                        self.errors
                            .push(syn::Error::new_spanned(meta, "unsupported attribute"));

                        return None;
                    }
                }
            }
        }

        Some(output)
    }

    /// Expand the necessary implementation details for `Any`.
    pub(super) fn expand_any<T>(
        &self,
        ident: T,
        name: &TokenStream,
    ) -> Result<TokenStream, Vec<syn::Error>>
    where
        T: Copy + ToTokens,
    {
        let any = &self.any;
        let named = &self.named;
        let raw_into_mut = &self.raw_into_mut;
        let raw_into_ref = &self.raw_into_ref;
        let shared = &self.shared;
        let pointer_guard = &self.pointer_guard;
        let ty = &self.ty;
        let hash = &self.hash;
        let type_info = &self.type_info;
        let unsafe_from_value = &self.unsafe_from_value;
        let unsafe_to_value = &self.unsafe_to_value;
        let value = &self.value;
        let type_of = &self.type_of;
        let vm_error = &self.vm_error;
        let raw_str = &self.raw_str;

        Ok(quote! {
            impl #any for #ident {
                fn type_hash() -> #hash {
                    // Safety: `Hash` asserts that it is layout compatible with `TypeId`.
                    // TODO: remove this once we can have transmute-like functionality in a const fn.
                    #hash::from_type_id(std::any::TypeId::of::<#ident>())
                }
            }

            impl #named for #ident {
                const NAME: #raw_str = #raw_str::from_str(#name);
            }

            impl #type_of for #ident {
                fn type_of() -> #ty {
                    #ty::from_type_hash(<Self as #any>::type_hash())
                }

                fn type_info() -> #type_info {
                    #type_info::Any(<Self as #named>::NAME)
                }
            }

            impl #unsafe_from_value for &#ident {
                type Output = *const #ident;
                type Guard = #raw_into_ref;

                unsafe fn unsafe_from_value(
                    value: #value,
                ) -> Result<(Self::Output, Self::Guard), #vm_error> {
                    Ok(value.unsafe_into_any_ref()?)
                }

                unsafe fn to_arg(output: Self::Output) -> Self {
                    &*output
                }
            }

            impl #unsafe_from_value for &mut #ident {
                type Output = *mut #ident;
                type Guard = #raw_into_mut;

                unsafe fn unsafe_from_value(
                    value: #value,
                ) -> Result<(Self::Output, Self::Guard), #vm_error> {
                    Ok(value.unsafe_into_any_mut()?)
                }

                unsafe fn to_arg(output: Self::Output) -> Self {
                    &mut *output
                }
            }

            impl #unsafe_to_value for &#ident {
                type Guard = #pointer_guard;

                unsafe fn unsafe_to_value(self) -> Result<(#value, Self::Guard), #vm_error> {
                    let (shared, guard) = #shared::from_ref(self);
                    Ok((#value::from(shared), guard))
                }
            }

            impl #unsafe_to_value for &mut #ident {
                type Guard = #pointer_guard;

                unsafe fn unsafe_to_value(self) -> Result<(#value, Self::Guard), #vm_error> {
                    let (shared, guard) = #shared::from_mut(self);
                    Ok((#value::from(shared), guard))
                }
            }
        })
    }
}
