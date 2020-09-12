use crate::internals;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::spanned::Spanned as _;

pub(super) fn expand_derive(input: &syn::DeriveInput) -> Result<TokenStream, Vec<syn::Error>> {
    let name = syn::LitStr::new(&input.ident.to_string(), input.ident.span());
    let name = &quote!(#name);
    expand_internal(&internals::RUNESTICK, &input.ident, &name)
}

pub(super) fn expand_type_path(ty: &syn::TypePath) -> Result<TokenStream, Vec<syn::Error>> {
    let name = match ty.path.segments.last() {
        Some(last) => quote!(stringify!(#last)),
        None => {
            return Err(vec![syn::Error::new(
                ty.span(),
                "expected segments in path",
            )])
        }
    };

    expand_internal(&quote!(crate), &ty, &name)
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
    let hash = &quote!(#module::Hash);
    let raw_owned_mut = &quote!(#module::RawOwnedMut);
    let raw_owned_ref = &quote!(#module::RawOwnedRef);
    let shared = &quote!(#module::Shared);
    let pointer_guard = &quote!(#module::SharedPointerGuard);
    let ty = &quote!(#module::Type);
    let type_info = &quote!(#module::TypeInfo);
    let unsafe_from_value = &quote!(#module::UnsafeFromValue);
    let unsafe_to_value = &quote!(#module::UnsafeToValue);
    let value = &quote!(#module::Value);
    let type_of = &quote!(#module::TypeOf);
    let vm_error = &quote!(#module::VmError);

    Ok(quote! {
        impl #any for #ident {
            const NAME: &'static str = #name;
        }

        impl #type_of for #ident {
            fn type_of() -> #ty {
                #ty::from(#hash::from_type_id(
                    std::any::TypeId::of::<#ident>(),
                ))
            }

            fn type_info() -> #type_info {
                #type_info::Any(#name)
            }
        }

        impl<'a> #unsafe_from_value for &'a #ident {
            type Output = *const #ident;
            type Guard = #raw_owned_ref;

            unsafe fn unsafe_from_value(
                value: #value,
            ) -> Result<(Self::Output, Self::Guard), #vm_error> {
                Ok(value.unsafe_into_any_ref()?)
            }

            unsafe fn to_arg(output: Self::Output) -> Self {
                &*output
            }
        }

        impl<'a> #unsafe_from_value for &'a mut #ident {
            type Output = *mut #ident;
            type Guard = #raw_owned_mut;

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
