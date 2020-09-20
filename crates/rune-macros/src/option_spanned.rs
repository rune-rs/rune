use crate::context::Context;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned as _;

/// Derive implementation of `rune::OptionSpanned`.
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
        let inner = self.expand_struct_fields(&st.fields)?;

        let ident = &input.ident;
        let option_spanned = &self.ctx.option_spanned;
        let span = &self.ctx.span;

        Some(quote! {
            impl #option_spanned for #ident {
                fn span(&self) -> #span {
                    #inner
                }
            }
        })
    }

    /// Expand on a struct.
    fn expand_enum(&mut self, input: &syn::DeriveInput, st: &syn::DataEnum) -> Option<TokenStream> {
        let _ = self.ctx.pase_derive_attributes(&input.attrs)?;

        let mut impl_spanned = Vec::new();

        for variant in &st.variants {
            let expanded = self.expand_variant_fields(variant, &variant.fields)?;
            impl_spanned.push(expanded);
        }

        let ident = &input.ident;
        let option_spanned = &self.ctx.option_spanned;
        let span = &self.ctx.span;

        Some(quote_spanned! { input.span() =>
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
    fn expand_struct_fields(&mut self, fields: &syn::Fields) -> Option<TokenStream> {
        match fields {
            syn::Fields::Named(named) => self.expand_struct_named(named),
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
    fn expand_struct_named(&mut self, named: &syn::FieldsNamed) -> Option<TokenStream> {
        let it = named
            .named
            .iter()
            .map(|f| {
                let var = self.ctx.field_ident(f).map(|n| quote!(&self.#n));
                (var, f)
            })
            .collect::<Vec<_>>();

        let it = it.into_iter();
        self.build_inner(named, it)
    }

    fn build_inner<'a>(
        &mut self,
        tokens: &(impl quote::ToTokens + syn::spanned::Spanned),
        mut it: impl DoubleEndedIterator<Item = (Option<TokenStream>, &'a syn::Field)>,
    ) -> Option<TokenStream> {
        let (begin_term, begin) = self.ctx.build_spanned_iter(false, &mut it)?;

        let begin = match (begin_term, begin) {
            (false, Some(begin)) => begin,
            (true, Some(begin)) => {
                return Some(quote_spanned!(tokens.span() => #begin));
            }
            (_, None) => {
                return Some(quote_spanned!(tokens.span() => None));
            }
        };

        let mut it = it.rev();
        let (end_term, end) = self.ctx.build_spanned_iter(true, &mut it)?;

        Some(if end_term {
            if let Some(end) = end {
                quote_spanned! { tokens.span() => {
                    let begin = #begin;
                    let end = #end;

                    match end {
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
    ) -> Option<TokenStream> {
        match fields {
            syn::Fields::Named(..) => {
                self.ctx.errors.push(syn::Error::new_spanned(
                    fields,
                    "named enum variants are not supported",
                ));
                return None;
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
    ) -> Option<TokenStream> {
        let it = unnamed
            .unnamed
            .iter()
            .enumerate()
            .map(|(n, f)| {
                let ident = syn::Ident::new(&format!("f{}", n), f.span());
                (Some(quote!(#ident)), f)
            })
            .collect::<Vec<_>>();

        let it = it.into_iter();
        let body = self.build_inner(unnamed, it);

        let ident = &variant.ident;
        let vars =
            (0..unnamed.unnamed.len()).map(|n| syn::Ident::new(&format!("f{}", n), variant.span()));

        Some(quote_spanned!(variant.span() => Self::#ident(#(#vars,)*) => #body))
    }

    /// Expand the implementation for a unit variant.
    fn expand_variant_unit(&mut self, variant: &syn::Variant) -> Option<TokenStream> {
        let ident = &variant.ident;
        Some(quote_spanned!(variant.span() => Self::#ident => None))
    }
}
