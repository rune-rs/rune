use crate::add_trait_bounds;
use crate::context::{Context, Tokens};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned as _;

/// Derive implementation of the ToTokens macro.
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
        let cx = Context::new();
        let tokens = cx.tokens_with_module(None);

        let mut expander = Expander { cx, tokens };

        match &self.input.data {
            syn::Data::Struct(st) => {
                if let Ok(stream) = expander.expand_struct(&self.input, st) {
                    return Ok(stream);
                }
            }
            syn::Data::Enum(en) => {
                if let Ok(stream) = expander.expand_enum(&self.input, en) {
                    return Ok(stream);
                }
            }
            syn::Data::Union(un) => {
                expander.cx.error(syn::Error::new_spanned(
                    un.union_token,
                    "not supported on unions",
                ));
            }
        }

        Err(expander.cx.errors.into_inner())
    }
}

struct Expander {
    cx: Context,
    tokens: Tokens,
}

impl Expander {
    /// Expand on a struct.
    fn expand_struct(
        &mut self,
        input: &syn::DeriveInput,
        st: &syn::DataStruct,
    ) -> Result<TokenStream, ()> {
        let _ = self.cx.type_attrs(&input.attrs)?;
        self.expand_struct_fields(input, &st.fields)
    }

    /// Expand on a struct.
    fn expand_enum(
        &mut self,
        input: &syn::DeriveInput,
        st: &syn::DataEnum,
    ) -> Result<TokenStream, ()> {
        let _ = self.cx.type_attrs(&input.attrs)?;

        let mut impl_into_tokens = Vec::new();

        for variant in &st.variants {
            let expanded = self.expand_variant_fields(variant, &variant.fields)?;
            impl_into_tokens.push(expanded);
        }

        let ident = &input.ident;
        let Tokens {
            to_tokens,
            macro_context,
            token_stream,
            alloc,
            ..
        } = &self.tokens;

        let mut generics = input.generics.clone();

        add_trait_bounds(&mut generics, to_tokens);

        let (impl_generics, type_generics, where_generics) = generics.split_for_impl();

        Ok(quote! {
            #[automatically_derived]
            impl #impl_generics #to_tokens for #ident #type_generics #where_generics {
                fn to_tokens(&self, context: &mut #macro_context, stream: &mut #token_stream) -> #alloc::Result<()> {
                    match self {
                        #(#impl_into_tokens),*
                    }

                    Ok(())
                }
            }
        })
    }

    /// Expand field decoding.
    fn expand_struct_fields(
        &mut self,
        input: &syn::DeriveInput,
        fields: &syn::Fields,
    ) -> Result<TokenStream, ()> {
        match fields {
            syn::Fields::Named(named) => self.expand_struct_named(input, named),
            syn::Fields::Unnamed(..) => {
                self.cx.error(syn::Error::new_spanned(
                    fields,
                    "tuple structs are not supported",
                ));
                Err(())
            }
            syn::Fields::Unit => {
                self.cx.error(syn::Error::new_spanned(
                    fields,
                    "unit structs are not supported",
                ));
                Err(())
            }
        }
    }

    /// Expand variant ast.
    fn expand_variant_fields(
        &mut self,
        variant: &syn::Variant,
        fields: &syn::Fields,
    ) -> Result<TokenStream, ()> {
        match fields {
            syn::Fields::Named(named) => self.expand_variant_named(variant, named),
            syn::Fields::Unnamed(unnamed) => self.expand_variant_unnamed(variant, unnamed),
            syn::Fields::Unit => Ok(self.expand_variant_unit(variant)),
        }
    }

    /// Expand named fields.
    fn expand_struct_named(
        &mut self,
        input: &syn::DeriveInput,
        named: &syn::FieldsNamed,
    ) -> Result<TokenStream, ()> {
        let mut fields = Vec::new();

        let Tokens {
            to_tokens,
            macro_context,
            token_stream,
            alloc,
            ..
        } = &self.tokens;

        for field in &named.named {
            let ident = self.cx.field_ident(field)?;
            let attrs = self.cx.field_attrs(&field.attrs)?;

            if attrs.skip() {
                continue;
            }

            fields.push(quote! { #to_tokens::to_tokens(&self.#ident, context, stream)? })
        }

        let ident = &input.ident;

        let mut generics = input.generics.clone();

        add_trait_bounds(&mut generics, to_tokens);

        let (impl_generics, type_generics, where_generics) = generics.split_for_impl();

        let into_tokens_impl = quote_spanned! { named.span() =>
            #[automatically_derived]
            impl #impl_generics #to_tokens for #ident #type_generics #where_generics {
                fn to_tokens(&self, context: &mut #macro_context, stream: &mut #token_stream) -> #alloc::Result<()> {
                    #(#fields;)*
                    Ok(())
                }
            }
        };

        Ok(quote_spanned! { named.span() =>
            #into_tokens_impl
        })
    }

    /// Expand named variant fields.
    fn expand_variant_named(
        &mut self,
        variant: &syn::Variant,
        named: &syn::FieldsNamed,
    ) -> Result<TokenStream, ()> {
        let mut fields = Vec::new();
        let mut idents = Vec::new();

        let Tokens { to_tokens, .. } = &self.tokens;

        for field in &named.named {
            let ident = self.cx.field_ident(field)?;
            let attrs = self.cx.field_attrs(&field.attrs)?;
            idents.push(ident);

            if attrs.skip() {
                continue;
            }

            fields.push(quote! { #to_tokens::to_tokens(&#ident, context, stream)? })
        }

        let ident = &variant.ident;

        Ok(quote! {
            Self::#ident { #(#idents,)* } => { #(#fields;)* }
        })
    }

    /// Expand named variant fields.
    fn expand_variant_unnamed(
        &mut self,
        variant: &syn::Variant,
        named: &syn::FieldsUnnamed,
    ) -> Result<TokenStream, ()> {
        let mut field_into_tokens = Vec::new();
        let mut idents = Vec::new();

        let Tokens { to_tokens, .. } = &self.tokens;

        for (n, field) in named.unnamed.iter().enumerate() {
            let ident = syn::Ident::new(&format!("f{}", n), field.span());
            let attrs = self.cx.field_attrs(&field.attrs)?;

            idents.push(ident.clone());

            if attrs.skip() {
                continue;
            }

            field_into_tokens.push(quote! { #to_tokens::to_tokens(#ident, context, stream)? })
        }

        let ident = &variant.ident;

        Ok(quote! {
            Self::#ident(#(#idents,)*) => { #(#field_into_tokens;)* }
        })
    }

    /// Expand unit variant.
    fn expand_variant_unit(&mut self, variant: &syn::Variant) -> TokenStream {
        let ident = &variant.ident;

        quote_spanned! { variant.span() =>
            Self::#ident => ()
        }
    }
}
