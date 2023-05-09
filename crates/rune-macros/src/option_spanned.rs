use crate::context::{Context, Tokens};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned as _;

/// Derive implementation of `OptionSpanned`.
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
        let ctx = Context::with_crate();
        let tokens = ctx.tokens_with_module(None);

        let mut expander = Expander { ctx, tokens };

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
                expander.ctx.error(syn::Error::new_spanned(
                    un.union_token,
                    "not supported on unions",
                ));
            }
        }

        Err(expander.ctx.errors.into_inner())
    }
}

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
        let inner = self.expand_struct_fields(&st.fields)?;

        let ident = &input.ident;
        let option_spanned = &self.tokens.option_spanned;
        let span = &self.tokens.span;

        Ok(quote! {
            #[automatically_derived]
            impl #option_spanned for #ident {
                fn span(&self) -> #span {
                    #inner
                }
            }
        })
    }

    /// Expand on a struct.
    fn expand_enum(
        &mut self,
        input: &syn::DeriveInput,
        st: &syn::DataEnum,
    ) -> Result<TokenStream, ()> {
        let _ = self.ctx.field_attrs(&input.attrs)?;

        let mut impl_spanned = Vec::new();

        for variant in &st.variants {
            let expanded = self.expand_variant_fields(variant, &variant.fields)?;
            impl_spanned.push(expanded);
        }

        let ident = &input.ident;
        let option_spanned = &self.tokens.option_spanned;
        let span = &self.tokens.span;

        Ok(quote_spanned! { input.span() =>
            #[automatically_derived]
            impl #option_spanned for #ident {
                fn option_span(&self) -> Option<#span> {
                    match self {
                        #(#impl_spanned,)*
                    }
                }
            }
        })
    }

    /// Expand field decoding.
    fn expand_struct_fields(&mut self, fields: &syn::Fields) -> Result<TokenStream, ()> {
        match fields {
            syn::Fields::Named(named) => self.expand_struct_named(named),
            syn::Fields::Unnamed(..) => {
                self.ctx.error(syn::Error::new_spanned(
                    fields,
                    "Tuple structs are not supported",
                ));
                Err(())
            }
            syn::Fields::Unit => {
                self.ctx.error(syn::Error::new_spanned(
                    fields,
                    "Unit structs are not supported",
                ));
                Err(())
            }
        }
    }

    /// Expand named fields.
    fn expand_struct_named(&mut self, named: &syn::FieldsNamed) -> Result<TokenStream, ()> {
        let values = named
            .named
            .iter()
            .map(|f| {
                let var = self.ctx.field_ident(f).map(|n| quote!(&self.#n));
                (var, f)
            })
            .collect::<Vec<_>>();

        self.build_inner(named, values)
    }

    fn build_inner(
        &mut self,
        tokens: &(impl quote::ToTokens + syn::spanned::Spanned),
        values: Vec<(Result<TokenStream, ()>, &syn::Field)>,
    ) -> Result<TokenStream, ()> {
        let (optional, begin) =
            self.ctx
                .build_spanned_iter(&self.tokens, false, values.clone().into_iter())?;

        let begin = match (optional, begin) {
            (false, Some(begin)) => begin,
            (true, Some(begin)) => {
                return Ok(quote_spanned!(tokens.span() => #begin));
            }
            (_, None) => {
                return Ok(quote_spanned!(tokens.span() => None));
            }
        };

        let (end_optional, end) =
            self.ctx
                .build_spanned_iter(&self.tokens, true, values.into_iter().rev())?;

        Ok(if end_optional {
            if let Some(end) = end {
                quote_spanned! { tokens.span() => {
                    match #end {
                        Some(end) => Some(begin.join(end)),
                        None => Some(begin),
                    }
                }}
            } else {
                quote_spanned!(tokens.span() => Some(#begin))
            }
        } else {
            quote_spanned!(tokens.span() => Some(#begin.join(#end)))
        })
    }

    /// Expand variant ast.
    fn expand_variant_fields(
        &mut self,
        variant: &syn::Variant,
        fields: &syn::Fields,
    ) -> Result<TokenStream, ()> {
        match fields {
            syn::Fields::Named(..) => {
                self.ctx.error(syn::Error::new_spanned(
                    fields,
                    "Named enum variants are not supported",
                ));
                Err(())
            }
            syn::Fields::Unnamed(unnamed) => self.expand_variant_unnamed(variant, unnamed),
            syn::Fields::Unit => self.expand_variant_unit(variant),
        }
    }

    /// Expand named variant fields.
    fn expand_variant_unnamed(
        &mut self,
        variant: &syn::Variant,
        unnamed: &syn::FieldsUnnamed,
    ) -> Result<TokenStream, ()> {
        let values = unnamed
            .unnamed
            .iter()
            .enumerate()
            .map(|(n, f)| {
                let ident = syn::Ident::new(&format!("f{}", n), f.span());
                (Ok(quote!(#ident)), f)
            })
            .collect::<Vec<_>>();

        let body = self.build_inner(unnamed, values)?;

        let ident = &variant.ident;
        let vars =
            (0..unnamed.unnamed.len()).map(|n| syn::Ident::new(&format!("f{}", n), variant.span()));

        Ok(quote_spanned!(variant.span() => Self::#ident(#(#vars,)*) => #body))
    }

    /// Expand the implementation for a unit variant.
    fn expand_variant_unit(&mut self, variant: &syn::Variant) -> Result<TokenStream, ()> {
        let ident = &variant.ident;
        Ok(quote_spanned!(variant.span() => Self::#ident => None))
    }
}
