use crate::context::{Context, Tokens};
use proc_macro2::TokenStream;
use quote::quote;

struct Expander<'cx> {
    cx: &'cx Context,
    tokens: Tokens,
}

impl Expander<'_> {
    /// Expand on a struct.
    fn expand_struct(
        &mut self,
        input: &syn::DeriveInput,
        st: &syn::DataStruct,
    ) -> Result<TokenStream, ()> {
        let inner = self.expand_fields(&st.fields)?;

        let ident = &input.ident;

        let Tokens {
            value,
            to_value,
            result,
            runtime_error,
            ..
        } = &self.tokens;

        Ok(quote! {
            #[automatically_derived]
            impl #to_value for #ident {
                fn to_value(self) -> #result<#value, #runtime_error> {
                    #inner
                }
            }
        })
    }

    /// Expand field decoding.
    fn expand_fields(&mut self, fields: &syn::Fields) -> Result<TokenStream, ()> {
        match fields {
            syn::Fields::Unnamed(named) => self.expand_unnamed(named),
            syn::Fields::Named(named) => self.expand_named(named),
            syn::Fields::Unit => {
                self.cx.error(syn::Error::new_spanned(
                    fields,
                    "unit structs are not supported",
                ));
                Err(())
            }
        }
    }

    /// Expand unnamed fields.
    fn expand_unnamed(&mut self, unnamed: &syn::FieldsUnnamed) -> Result<TokenStream, ()> {
        let mut to_values = Vec::new();

        let Tokens {
            to_value,
            value,
            owned_tuple,
            result,
            try_from,
            vec,
            ..
        } = &self.tokens;

        for (index, f) in unnamed.unnamed.iter().enumerate() {
            _ = self.cx.field_attrs(&f.attrs);
            let index = syn::Index::from(index);
            to_values.push(quote!(#vec::try_push(&mut tuple, #to_value::to_value(self.#index)?)?));
        }

        let cap = unnamed.unnamed.len();

        Ok(quote! {
            let mut tuple = #vec::try_with_capacity(#cap)?;
            #(#to_values;)*
            let tuple = <#owned_tuple as #try_from<_>>::try_from(tuple)?;
            #result::Ok(<#value as #try_from<_>>::try_from(tuple)?)
        })
    }

    /// Expand named fields.
    fn expand_named(&mut self, named: &syn::FieldsNamed) -> Result<TokenStream, ()> {
        let Tokens {
            to_value,
            value,
            object,
            result,
            string,
            try_from,
            ..
        } = &self.tokens;

        let mut to_values = Vec::new();

        for f in &named.named {
            let ident = self.cx.field_ident(f)?;
            _ = self.cx.field_attrs(&f.attrs);

            let name = syn::LitStr::new(&ident.to_string(), ident.span());

            to_values.push(quote! {
                object.insert(<#string as #try_from<_>>::try_from(#name)?, #to_value::to_value(self.#ident)?)?
            });
        }

        Ok(quote! {
            let mut object = <#object>::new();
            #(#to_values;)*
            #result::Ok(<#value as #try_from<_>>::try_from(object)?)
        })
    }
}

pub(super) fn expand(cx: &Context, input: &syn::DeriveInput) -> Result<TokenStream, ()> {
    let attr = cx.type_attrs(&input.attrs);
    let tokens = cx.tokens_with_module(attr.module.as_ref());

    let mut expander = Expander { cx, tokens };

    match &input.data {
        syn::Data::Struct(st) => {
            if let Ok(expanded) = expander.expand_struct(input, st) {
                return Ok(expanded);
            }
        }
        syn::Data::Enum(en) => {
            expander.cx.error(syn::Error::new_spanned(
                en.enum_token,
                "not supported on enums",
            ));
        }
        syn::Data::Union(un) => {
            expander.cx.error(syn::Error::new_spanned(
                un.union_token,
                "not supported on unions",
            ));
        }
    }

    Err(())
}
