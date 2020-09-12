use crate::internals;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::spanned::Spanned as _;

/// An internal call to the macro.
pub struct InternalCall {
    path: syn::Path,
    name: Option<(syn::Token![,], syn::LitStr)>,
}

impl syn::parse::Parse for InternalCall {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let path = input.parse()?;

        let name = if input.peek(syn::Token![,]) {
            Some((input.parse()?, input.parse()?))
        } else {
            None
        };

        Ok(Self { path, name })
    }
}

impl InternalCall {
    pub fn expand(self) -> Result<TokenStream, Vec<syn::Error>> {
        let name = match self.name {
            Some((_, name)) => quote!(#name),
            None => match self.path.segments.last() {
                Some(last) if last.arguments.is_empty() => quote!(stringify!(#last)),
                _ => {
                    return Err(vec![syn::Error::new(
                        self.path.span(),
                        "expected last component in path to be without parameters,
                        give it an explicit name instead with `, \"Type\"`",
                    )])
                }
            },
        };

        expand_internal(&quote!(crate), &self.path, &name)
    }
}

pub(super) fn expand_derive(input: &syn::DeriveInput) -> Result<TokenStream, Vec<syn::Error>> {
    let name = syn::LitStr::new(&input.ident.to_string(), input.ident.span());
    let name = &quote!(#name);
    expand_internal(&internals::RUNESTICK, &input.ident, &name)
}

pub(super) fn expand_internal<M, T>(
    module: M,
    ident: T,
    name: &TokenStream,
) -> Result<TokenStream, Vec<syn::Error>>
where
    M: Copy + ToTokens,
    T: Copy + ToTokens,
{
    let any = &quote!(#module::Any);
    let named = &quote!(#module::Named);
    let raw_into_mut = &quote!(#module::RawMut);
    let raw_into_ref = &quote!(#module::RawRef);
    let shared = &quote!(#module::Shared);
    let pointer_guard = &quote!(#module::SharedPointerGuard);
    let ty = &quote!(#module::Type);
    let hash = &quote!(#module::Hash);
    let type_info = &quote!(#module::TypeInfo);
    let unsafe_from_value = &quote!(#module::UnsafeFromValue);
    let unsafe_to_value = &quote!(#module::UnsafeToValue);
    let value = &quote!(#module::Value);
    let type_of = &quote!(#module::TypeOf);
    let vm_error = &quote!(#module::VmError);
    let raw_str = &quote!(#module::RawStr);

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
