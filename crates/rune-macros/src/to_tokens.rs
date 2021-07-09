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
                if let Some(stream) = expander.expand_enum(&self.input, en) {
                    return Ok(stream);
                }
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
        let _ = self.ctx.parse_derive_attributes(&input.attrs)?;
        let inner = self.expand_struct_fields(input, &st.fields)?;

        Some(quote! {
            #inner
        })
    }

    /// Expand on a struct.
    fn expand_enum(&mut self, input: &syn::DeriveInput, st: &syn::DataEnum) -> Option<TokenStream> {
        let _ = self.ctx.parse_derive_attributes(&input.attrs)?;

        let mut impl_into_tokens = Vec::new();

        for variant in &st.variants {
            let expanded = self.expand_variant_fields(variant, &variant.fields)?;
            impl_into_tokens.push(expanded);
        }

        let ident = &input.ident;
        let to_tokens = &self.ctx.to_tokens;
        let macro_context = &self.ctx.macro_context;
        let token_stream = &self.ctx.token_stream;

        Some(quote_spanned! { input.span() =>
            impl #to_tokens for #ident {
                fn to_tokens(&self, context: &#macro_context, stream: &mut #token_stream) {
                    match self {
                        #(#impl_into_tokens,)*
                    }
                }
            }
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

    /// Expand variant ast.
    fn expand_variant_fields(
        &mut self,
        variant: &syn::Variant,
        fields: &syn::Fields,
    ) -> Option<TokenStream> {
        match fields {
            syn::Fields::Named(named) => self.expand_variant_named(variant, named),
            syn::Fields::Unnamed(unnamed) => self.expand_variant_unnamed(variant, unnamed),
            syn::Fields::Unit => self.expand_variant_unit(variant),
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
            let ident = self.ctx.field_ident(field)?;
            let attrs = self.ctx.parse_field_attributes(&field.attrs)?;

            if attrs.skip() {
                continue;
            }

            fields.push(quote_spanned! { field.span() => self.#ident.to_tokens(context, stream) })
        }

        let ident = &input.ident;

        let to_tokens = &self.ctx.to_tokens;
        let macro_context = &self.ctx.macro_context;
        let token_stream = &self.ctx.token_stream;

        let generics = &input.generics;

        let bounds = generic_bounds(generics, to_tokens);

        let into_tokens_impl = quote_spanned! {
            named.span() => impl #generics #to_tokens for #ident #generics #bounds {
                fn to_tokens(&self, context: &#macro_context, stream: &mut #token_stream) {
                    #(#fields;)*
                }
            }
        };

        Some(quote_spanned! { named.span() =>
            #into_tokens_impl
        })
    }

    /// Expand named variant fields.
    fn expand_variant_named(
        &mut self,
        variant: &syn::Variant,
        named: &syn::FieldsNamed,
    ) -> Option<TokenStream> {
        let mut fields = Vec::new();
        let mut idents = Vec::new();

        for field in &named.named {
            let ident = self.ctx.field_ident(field)?;
            let attrs = self.ctx.parse_field_attributes(&field.attrs)?;
            idents.push(ident);

            if attrs.skip() {
                continue;
            }

            fields.push(quote_spanned! { field.span() => #ident.to_tokens(context, stream) })
        }

        let ident = &variant.ident;

        Some(quote_spanned! { named.span() =>
            Self::#ident { #(#idents,)* } => { #(#fields;)* }
        })
    }

    /// Expand named variant fields.
    fn expand_variant_unnamed(
        &mut self,
        variant: &syn::Variant,
        named: &syn::FieldsUnnamed,
    ) -> Option<TokenStream> {
        let mut field_into_tokens = Vec::new();
        let mut idents = Vec::new();

        for (n, field) in named.unnamed.iter().enumerate() {
            let ident = syn::Ident::new(&format!("f{}", n), field.span());
            let attrs = self.ctx.parse_field_attributes(&field.attrs)?;

            idents.push(ident.clone());

            if attrs.skip() {
                continue;
            }

            field_into_tokens
                .push(quote_spanned! { field.span() => #ident.to_tokens(context, stream) })
        }

        let ident = &variant.ident;

        Some(quote_spanned! { named.span() =>
            Self::#ident(#(#idents,)*) => { #(#field_into_tokens;)* }
        })
    }

    /// Expand unit variant.
    fn expand_variant_unit(&mut self, variant: &syn::Variant) -> Option<TokenStream> {
        let ident = &variant.ident;

        Some(quote_spanned! { variant.span() =>
            Self::#ident => ()
        })
    }
}

fn generic_bounds(generics: &syn::Generics, to_tokens: &TokenStream) -> Option<TokenStream> {
    if generics.params.is_empty() {
        return None;
    }

    let mut bound = Vec::new();

    for param in &generics.params {
        bound.push(quote_spanned!(param.span() => #param: #to_tokens))
    }

    Some(quote_spanned! { generics.span() => where
        #(#bound,)*
    })
}
