use crate::context::Context;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned as _;

struct ExpandedVariant {
    into_tokens: TokenStream,
    spanned: Option<TokenStream>,
}

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
        let _ = self.ctx.parse_ast_derive(&input.attrs)?;
        let inner = self.expand_struct_fields(input, &st.fields)?;

        Some(quote! {
            #inner
        })
    }

    /// Expand on a struct.
    fn expand_enum(&mut self, input: &syn::DeriveInput, st: &syn::DataEnum) -> Option<TokenStream> {
        let _ = self.ctx.parse_ast_derive(&input.attrs)?;

        let mut impl_into_tokens = Vec::new();
        let mut impl_spanned = Option::Some::<Vec<TokenStream>>(vec![]);

        for variant in &st.variants {
            let expanded = self.expand_variant_fields(variant, &variant.fields)?;
            impl_into_tokens.push(expanded.into_tokens);

            if let Some(s) = &mut impl_spanned {
                if let Some(out) = expanded.spanned {
                    s.push(out);
                } else {
                    // NB: if one variant doesn't support implementing `Spanned`, it
                    // won't be implemented.
                    impl_spanned = None;
                }
            }
        }

        let ident = &input.ident;
        let into_tokens = &self.ctx.into_tokens;
        let spanned = &self.ctx.spanned;
        let span = &self.ctx.span;
        let macro_context = &self.ctx.macro_context;
        let token_stream = &self.ctx.token_stream;

        let spanned_impl = match impl_spanned {
            Some(impl_spanned) => Some(quote_spanned! { input.span() =>
                impl #spanned for #ident {
                    fn span(&self) -> #span {
                        match self {
                            #(#impl_spanned,)*
                        }
                    }
                }
            }),
            None => None,
        };

        Some(quote_spanned! { input.span() =>
            impl #into_tokens for #ident {
                fn into_tokens(&self, context: &mut #macro_context, stream: &mut #token_stream) {
                    match self {
                        #(#impl_into_tokens,)*
                    }
                }
            }

            #spanned_impl
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
    ) -> Option<ExpandedVariant> {
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
            let ident = self.ctx.field_ident(&field)?;
            let output = self.ctx.parse_ast_fields(&field.attrs)?;

            if !output.skip {
                fields.push(
                    quote_spanned! { field.span() => self.#ident.into_tokens(context, stream) },
                )
            }
        }

        let ident = &input.ident;

        let into_tokens = &self.ctx.into_tokens;
        let macro_context = &self.ctx.macro_context;
        let token_stream = &self.ctx.token_stream;

        let into_tokens_impl = quote_spanned! {
            named.span() => impl #into_tokens for #ident {
                fn into_tokens(&self, context: &mut #macro_context, stream: &mut #token_stream) {
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
    ) -> Option<ExpandedVariant> {
        let mut fields = Vec::new();
        let mut idents = Vec::new();

        for field in &named.named {
            let ident = self.ctx.field_ident(&field)?;
            let output = self.ctx.parse_ast_fields(&field.attrs)?;

            if !output.skip {
                fields.push(quote_spanned! { field.span() => #ident.into_tokens(context, stream) })
            }

            idents.push(ident);
        }

        let ident = &variant.ident;

        Some(ExpandedVariant {
            into_tokens: quote_spanned! { named.span() =>
                Self::#ident { #(#idents,)* } => { #(#fields;)* }
            },
            spanned: None,
        })
    }

    /// Expand named variant fields.
    fn expand_variant_unnamed(
        &mut self,
        variant: &syn::Variant,
        named: &syn::FieldsUnnamed,
    ) -> Option<ExpandedVariant> {
        let mut field_into_tokens = Vec::new();
        let mut idents = Vec::new();

        for (n, field) in named.unnamed.iter().enumerate() {
            let ident = syn::Ident::new(&format!("f{}", n), field.span());
            let output = self.ctx.parse_ast_fields(&field.attrs)?;

            if !output.skip {
                let ident = &ident;

                field_into_tokens
                    .push(quote_spanned! { field.span() => #ident.into_tokens(context, stream) })
            }

            idents.push(ident);
        }

        let ident = &variant.ident;

        let spanned = if named.unnamed.len() == 1 {
            Some(quote_spanned! { named.span() =>
                Self::#ident(v) => v.span()
            })
        } else {
            None
        };

        Some(ExpandedVariant {
            into_tokens: quote_spanned! { named.span() =>
                Self::#ident(#(#idents,)*) => { #(#field_into_tokens;)* }
            },
            spanned,
        })
    }

    /// Expand unit variant.
    fn expand_variant_unit(&mut self, variant: &syn::Variant) -> Option<ExpandedVariant> {
        let ident = &variant.ident;

        Some(ExpandedVariant {
            into_tokens: quote_spanned! { variant.span() =>
                Self::#ident => ()
            },
            spanned: None,
        })
    }
}
