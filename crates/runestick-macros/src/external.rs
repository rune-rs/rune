use proc_macro2::TokenStream;
use quote::{quote, ToTokens};

pub(super) fn expand<M, T>(module: M, ident: T) -> Result<TokenStream, Vec<syn::Error>>
where
    M: Copy + ToTokens,
    T: Copy + ToTokens,
{
    let any = &quote!(#module::Any);
    let from_value = &quote!(#module::FromValue);
    let hash = &quote!(#module::Hash);
    let raw_owned_mut = &quote!(#module::RawOwnedMut);
    let raw_owned_ref = &quote!(#module::RawOwnedRef);
    let shared = &quote!(#module::Shared);
    let pointer_guard = &quote!(#module::SharedPointerGuard);
    let to_value = &quote!(#module::ToValue);
    let ty = &quote!(#module::Type);
    let type_info = &quote!(#module::TypeInfo);
    let unsafe_from_value = &quote!(#module::UnsafeFromValue);
    let unsafe_to_value = &quote!(#module::UnsafeToValue);
    let value = &quote!(#module::Value);
    let value_type = &quote!(#module::ValueType);
    let vm_error = &quote!(#module::VmError);

    Ok(quote! {
        impl #value_type for #ident {
            fn value_type() -> #ty {
                #ty::from(#hash::from_type_id(
                    std::any::TypeId::of::<#ident>(),
                ))
            }

            fn type_info() -> #type_info {
                #type_info::Any(std::any::type_name::<#ident>())
            }
        }

        impl #from_value for #ident {
            fn from_value(value: #value) -> Result<Self, #vm_error> {
                let any = value.into_any()?;
                let any = any.take_downcast::<#ident>()?;
                Ok(any)
            }
        }

        impl #to_value for #ident {
            fn to_value(self) -> Result<#value, #vm_error> {
                Ok(#value::from(#shared::new(#any::new(self))))
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
