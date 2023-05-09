use crate::context::{Context, Tokens};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned as _;

struct Expander {
    ctx: Context,
    tokens: Tokens,
}

impl Expander {
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
            vm_result,
            to_value,
            ..
        } = &self.tokens;

        Ok(quote_spanned! { input.span() =>
            #[automatically_derived]
            impl #to_value for #ident {
                fn to_value(self) -> #vm_result<#value> {
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
                self.ctx.error(syn::Error::new_spanned(
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
            tuple,
            vm_result,
            ..
        } = &self.tokens;

        for (index, f) in unnamed.unnamed.iter().enumerate() {
            let _ = self.ctx.field_attrs(&f.attrs)?;
            let index = syn::Index::from(index);
            let to_value = self.tokens.vm_try(quote!(#to_value::to_value(self.#index)));
            to_values.push(quote_spanned!(f.span() => tuple.push(#to_value)));
        }

        let cap = unnamed.unnamed.len();

        Ok(quote_spanned! {
            unnamed.span() =>
            let mut tuple = Vec::with_capacity(#cap);
            #(#to_values;)*
            #vm_result::Ok(#value::from(#tuple::from(tuple)))
        })
    }

    /// Expand named fields.
    fn expand_named(&mut self, named: &syn::FieldsNamed) -> Result<TokenStream, ()> {
        let Tokens {
            to_value,
            value,
            object,
            vm_result,
            ..
        } = &self.tokens;

        let mut to_values = Vec::new();

        for f in &named.named {
            let ident = self.ctx.field_ident(f)?;
            let _ = self.ctx.field_attrs(&f.attrs)?;

            let name = &syn::LitStr::new(&ident.to_string(), ident.span());
            let to_value = self.tokens.vm_try(quote!(#to_value::to_value(self.#ident)));
            to_values
                .push(quote_spanned!(f.span() => object.insert(String::from(#name), #to_value)));
        }

        Ok(quote_spanned! {
            named.span() =>
            let mut object = <#object>::new();
            #(#to_values;)*
            #vm_result::Ok(#value::from(object))
        })
    }
}

pub(super) fn expand(input: &syn::DeriveInput) -> Result<TokenStream, Vec<syn::Error>> {
    let ctx = Context::new();

    let Ok(attr) = ctx.type_attrs(&input.attrs) else {
        return Err(ctx.errors.into_inner());
    };

    let tokens = ctx.tokens_with_module(attr.module.as_ref());

    let mut expander = Expander {
        ctx: Context::new(),
        tokens,
    };

    match &input.data {
        syn::Data::Struct(st) => {
            if let Ok(expanded) = expander.expand_struct(input, st) {
                return Ok(expanded);
            }
        }
        syn::Data::Enum(en) => {
            expander.ctx.error(syn::Error::new_spanned(
                en.enum_token,
                "not supported on enums",
            ));
        }
        syn::Data::Union(un) => {
            expander.ctx.error(syn::Error::new_spanned(
                un.union_token,
                "not supported on unions",
            ));
        }
    }

    Err(expander.ctx.errors.into_inner())
}
