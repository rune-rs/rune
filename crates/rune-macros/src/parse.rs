use crate::{
    add_trait_bounds,
    context::{Context, ParseKind, Tokens},
};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned as _;

/// Derive implementation of the Parse macro.
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
        self.expand_struct_fields(input, &st.fields)
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
                    "Tuple structs are not supported",
                ));
                Err(())
            }
            syn::Fields::Unit => {
                self.cx.error(syn::Error::new_spanned(
                    fields,
                    "Unit structs are not supported",
                ));
                Err(())
            }
        }
    }

    /// Expand named fields.
    fn expand_struct_named(
        &mut self,
        input: &syn::DeriveInput,
        named: &syn::FieldsNamed,
    ) -> Result<TokenStream, ()> {
        let ident = &input.ident;
        let mut fields = Vec::new();

        let mut meta_args = Vec::new();
        let mut meta_parse = Vec::new();
        let mut meta_fields = Vec::new();

        let ty_attrs = self.cx.type_attrs(&input.attrs)?;
        let mut skipped = 0;

        for (i, field) in named.named.iter().enumerate() {
            let field_attrs = self.cx.field_attrs(&field.attrs)?;
            let ident = self.cx.field_ident(field)?;

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
                self.cx.error(syn::Error::new_spanned(
                    field,
                    format!(
                        "The first sequence of fields may have `#[rune({})]`, \
                        but field is outside of that sequence.",
                        crate::internals::META,
                    ),
                ));
                return Err(());
            }

            let ident = self.cx.field_ident(field)?;
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
        let compile_error = &self.tokens.compile_error;
        let result = &self.tokens.result;

        let mut generics = input.generics.clone();

        add_trait_bounds(&mut generics, parse);

        let (impl_generics, type_generics, where_generics) = generics.split_for_impl();

        let inner = if let ParseKind::MetaOnly = ty_attrs.parse {
            None
        } else {
            Some(quote_spanned! {
                named.span() =>
                #[automatically_derived]
                impl #impl_generics #parse for #ident #type_generics #where_generics {
                    fn parse(parser: &mut #parser<'_>) -> #result<Self, #compile_error> {
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
                        -> #result<Self, #compile_error>
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
                impl #impl_generics #parse for #ident #type_generics #where_generics {
                    fn parse(#parser_ident: &mut #parser<'_>) -> #result<Self, #compile_error> {
                        Ok(Self {
                            #(#fields,)*
                        })
                    }
                }
            }
        };

        Ok(output)
    }
}
