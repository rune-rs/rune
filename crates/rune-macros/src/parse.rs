use crate::context::Context;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned as _;

/// Derive implementation of the AST macro.
pub struct Derive {
    input: syn::DeriveInput,
}

impl syn::parse::Parse for Derive {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            input: input.parse()?,
        })
    }
}

impl Derive {
    pub(super) fn expand(self) -> Result<TokenStream, Vec<syn::Error>> {
        let mut expander = Expander {
            ctx: Context::new(),
        };

        match &self.input.data {
            syn::Data::Struct(st) => {
                if let Some(stream) = expander.expand_struct(&self.input, st) {
                    return Ok(stream);
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
}

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
        let inner = self.expand_struct_fields(input, &st.fields)?;

        Some(quote! {
            #inner
        })
    }

    /// Expand field decoding.
    fn expand_struct_fields(
        &mut self,
        input: &syn::DeriveInput,
        fields: &syn::Fields,
    ) -> Option<TokenStream> {
        match fields {
            syn::Fields::Named(named) => self.expand_struct_named(input, named),
            syn::Fields::Unnamed(..) => {
                self.ctx.errors.push(syn::Error::new_spanned(
                    fields,
                    "tuple structs are not supported",
                ));
                None
            }
            syn::Fields::Unit => {
                self.ctx.errors.push(syn::Error::new_spanned(
                    fields,
                    "unit structs are not supported",
                ));
                None
            }
        }
    }

    /// Expand named fields.
    fn expand_struct_named(
        &mut self,
        input: &syn::DeriveInput,
        named: &syn::FieldsNamed,
    ) -> Option<TokenStream> {
        let mut fields = Vec::new();

        for field in &named.named {
            let _ = self.ctx.parse_field_attributes(&field.attrs)?;
            let ident = self.ctx.field_ident(&field)?;
            fields.push(quote_spanned! { field.span() => #ident: parser.parse()? })
        }

        let ident = &input.ident;

        let parse = &self.ctx.parse;
        let parser = &self.ctx.parser;
        let parse_error = &self.ctx.parse_error;

        Some(quote_spanned! {
            named.span() => impl #parse for #ident {
                fn parse(parser: &mut #parser<'_>) -> Result<Self, #parse_error> {
                    Ok(Self {
                        #(#fields,)*
                    })
                }
            }
        })
    }
}
