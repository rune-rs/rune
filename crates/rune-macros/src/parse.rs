use crate::context::{Context, ParseKind, Tokens};
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
        let ctx = Context::with_crate();
        let tokens = ctx.tokens_with_module(None);

        let mut expander = Expander { ctx, tokens };

        match &self.input.data {
            syn::Data::Struct(st) => {
                if let Some(stream) = expander.expand_struct(&self.input, st) {
                    return Ok(stream);
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
                self.ctx.error(syn::Error::new_spanned(
                    fields,
                    "tuple structs are not supported",
                ));
                None
            }
            syn::Fields::Unit => {
                self.ctx.error(syn::Error::new_spanned(
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
        let ident = &input.ident;
        let mut fields = Vec::new();

        let mut meta_args = Vec::new();
        let mut meta_parse = Vec::new();
        let mut meta_fields = Vec::new();

        let ty_attrs = self.ctx.type_attrs(&input.attrs)?;
        let mut skipped = 0;

        for (i, field) in named.named.iter().enumerate() {
            let field_attrs = self.ctx.field_attrs(&field.attrs)?;
            let ident = self.ctx.field_ident(field)?;

            if field_attrs.id.is_some() {
                fields.push(quote_spanned! { field.span() => #ident: Default::default() });
                skipped += 1;
                continue;
            }

            let parse_impl = if let Some(parse_with) = field_attrs.parse_with {
                quote_spanned!(field.span() => #parse_with(parser)?)
            } else {
                quote_spanned!(field.span() => parser.parse()?)
            };

            if field_attrs.meta.is_none() {
                fields.push(quote_spanned! { field.span() => #ident: #parse_impl });
                continue;
            }

            if i - skipped != meta_fields.len() {
                self.ctx.error(syn::Error::new_spanned(
                    field,
                    format!(
                        "The first sequence of fields may have `#[rune({})]`, \
                        but field is outside of that sequence.",
                        crate::internals::META,
                    ),
                ));
                return None;
            }

            let ident = self.ctx.field_ident(field)?;
            let ty = &field.ty;
            meta_args.push(quote_spanned!(field.span() => #ident: #ty));
            meta_parse.push(quote_spanned!(field.span() => let #ident: #ty = #parse_impl));
            fields.push(quote_spanned! { field.span() => #ident });
            meta_fields.push(ident);
        }

        let parser_ident = &if fields.is_empty() {
            quote!(_parser)
        } else {
            quote!(parser)
        };

        let parse = &self.tokens.parse;
        let parser = &self.tokens.parser;
        let parse_error = &self.tokens.parse_error;

        let inner = if let ParseKind::MetaOnly = ty_attrs.parse {
            None
        } else {
            Some(quote_spanned! {
                named.span() =>
                #[automatically_derived]
                impl #parse for #ident {
                    fn parse(parser: &mut #parser<'_>) -> Result<Self, #parse_error> {
                        #(#meta_parse;)*
                        Self::parse_with_meta(parser, #(#meta_fields,)*)
                     }
                }
            })
        };

        let output = if !meta_args.is_empty() {
            quote_spanned! { named.span() =>
                #[automatically_derived]
                impl #ident {
                    #[doc = "Parse #ident and attach the given meta"]
                    pub fn parse_with_meta(#parser_ident: &mut #parser<'_>, #(#meta_args,)*)
                        -> Result<Self, #parse_error>
                    {
                        Ok(Self {
                            #(#fields,)*
                        })
                    }
                }

                #inner
            }
        } else {
            quote_spanned! { named.span() =>
                #[automatically_derived]
                impl #parse for #ident {
                    fn parse(#parser_ident: &mut #parser<'_>) -> Result<Self, #parse_error> {
                        Ok(Self {
                            #(#fields,)*
                        })
                    }
                }
            }
        };

        Some(output)
    }
}
