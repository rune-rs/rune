use crate::context::Context;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned as _;

struct Expander {
    ctx: Context,
}

impl Expander {
    /// Expand on a struct.
    fn expand_struct(
        &mut self,
        input: &syn::DeriveInput,
        st: &syn::DataStruct,
    ) -> Option<TokenStream> {
        let inner = self.expand_fields(&st.fields)?;

        let ident = &input.ident;
        let value = &self.ctx.value;
        let vm_error = &self.ctx.vm_error;
        let to_value = &self.ctx.to_value;

        Some(quote! {
            impl #to_value for #ident {
                fn to_value(self) -> Result<#value, #vm_error> {
                    #inner
                }
            }
        })
    }

    /// Expand field decoding.
    fn expand_fields(&mut self, fields: &syn::Fields) -> Option<TokenStream> {
        match fields {
            syn::Fields::Unnamed(named) => self.expand_unnamed(named),
            syn::Fields::Named(named) => self.expand_named(named),
            syn::Fields::Unit => {
                self.ctx.errors.push(syn::Error::new_spanned(
                    fields,
                    "unit structs are not supported",
                ));
                None
            }
        }
    }

    /// Get a field identifier.
    fn field_ident<'a>(&mut self, field: &'a syn::Field) -> Option<&'a syn::Ident> {
        match &field.ident {
            Some(ident) => Some(ident),
            None => {
                self.ctx.errors.push(syn::Error::new_spanned(
                    field,
                    "unnamed fields are not supported",
                ));
                None
            }
        }
    }

    /// Expand unnamed fields.
    fn expand_unnamed(&mut self, unnamed: &syn::FieldsUnnamed) -> Option<TokenStream> {
        let mut to_values = Vec::new();

        for (index, field) in unnamed.unnamed.iter().enumerate() {
            let _ = self.ctx.parse_field_attrs(&field.attrs)?;

            let index = syn::Index::from(index);

            let to_value = &self.ctx.to_value;

            to_values.push(quote_spanned! {
                field.span() =>
                tuple.push(#to_value::to_value(self.#index)?);
            });
        }

        let cap = unnamed.unnamed.len();
        let value = &self.ctx.value;
        let tuple = &self.ctx.tuple;

        Some(quote_spanned! {
            unnamed.span() =>
            let mut tuple = Vec::with_capacity(#cap);
            #(#to_values)*
            Ok(#value::from(#tuple::from(tuple)))
        })
    }

    /// Expand named fields.
    fn expand_named(&mut self, named: &syn::FieldsNamed) -> Option<TokenStream> {
        let mut to_values = Vec::new();

        for field in &named.named {
            let ident = self.field_ident(&field)?;
            let _ = self.ctx.parse_field_attrs(&field.attrs)?;

            let name = &syn::LitStr::new(&ident.to_string(), ident.span());

            let to_value = &self.ctx.to_value;

            to_values.push(quote_spanned! {
                field.span() =>
                object.insert(String::from(#name), #to_value::to_value(self.#ident)?);
            });
        }

        let value = &self.ctx.value;
        let object = &self.ctx.object;

        Some(quote_spanned! {
            named.span() =>
            let mut object = <#object>::new();
            #(#to_values)*
            Ok(#value::from(object))
        })
    }
}

pub(super) fn expand(input: &syn::DeriveInput) -> Result<TokenStream, Vec<syn::Error>> {
    let mut expander = Expander {
        ctx: Context::new(),
    };

    match &input.data {
        syn::Data::Struct(st) => {
            if let Some(expanded) = expander.expand_struct(input, st) {
                return Ok(expanded);
            }
        }
        syn::Data::Enum(en) => {
            expander.ctx.errors.push(syn::Error::new_spanned(
                en.enum_token,
                "not supported on enums",
            ));
        }
        syn::Data::Union(un) => {
            expander.ctx.errors.push(syn::Error::new_spanned(
                un.union_token,
                "not supported on unions",
            ));
        }
    }

    Err(expander.ctx.errors)
}
