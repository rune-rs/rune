use crate::{
    add_trait_bounds,
    context::{Context, Tokens},
};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::Token;

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
    pub(super) fn expand(self, is_option_spanned: bool) -> Result<TokenStream, Vec<syn::Error>> {
        let cx = Context::new();
        let tokens = cx.tokens_with_module(None);

        let mut expander = Expander { cx, tokens };

        if expander.cx.type_attrs(&self.input.attrs).is_err() {
            return Err(expander.cx.errors.into_inner());
        }

        let inner = match &self.input.data {
            syn::Data::Struct(st) => {
                let Ok(inner) = expander.expand_struct_fields(
                    &st.fields,
                    |member| quote!(&self.#member),
                    is_option_spanned,
                ) else {
                    return Err(expander.cx.errors.into_inner());
                };

                inner
            }
            syn::Data::Enum(enum_) => {
                let Ok(inner) = expander.expand_enum(enum_, is_option_spanned) else {
                    return Err(expander.cx.errors.into_inner());
                };

                inner
            }
            syn::Data::Union(un) => {
                expander.cx.error(syn::Error::new_spanned(
                    un.union_token,
                    "not supported on unions",
                ));

                return Err(expander.cx.errors.into_inner());
            }
        };

        let ident = &self.input.ident;

        let Tokens {
            spanned,
            option_spanned,
            span,
            option,
            ..
        } = &expander.tokens;

        let mut generics = self.input.generics.clone();

        let (trait_t, ret) = if is_option_spanned {
            add_trait_bounds(&mut generics, option_spanned);
            (option_spanned, quote!(#option<#span>))
        } else {
            add_trait_bounds(&mut generics, spanned);
            (spanned, quote!(#span))
        };

        let (impl_gen, type_gen, where_gen) = generics.split_for_impl();

        let name = if is_option_spanned {
            syn::Ident::new("option_span", Span::call_site())
        } else {
            syn::Ident::new("span", Span::call_site())
        };

        let implementation = quote! {
            #[automatically_derived]
            impl #impl_gen #trait_t for #ident #type_gen #where_gen {
                fn #name(&self) -> #ret {
                    #inner
                }
            }
        };

        let option_spanned = (!is_option_spanned).then(|| {
            quote! {
                #[automatically_derived]
                impl #impl_gen #option_spanned for #ident #type_gen #where_gen {
                    fn option_span(&self) -> #option<#span> {
                        #option::Some(#spanned::span(self))
                    }
                }
            }
        });

        Ok(quote! {
            #implementation
            #option_spanned
        })
    }
}

struct Expander {
    cx: Context,
    tokens: Tokens,
}

impl Expander {
    /// Expand on a struct.
    fn expand_enum(
        &mut self,
        enum_: &syn::DataEnum,
        is_option_spanned: bool,
    ) -> Result<TokenStream, ()> {
        let mut variants = Vec::new();

        for variant in &enum_.variants {
            let ident = &variant.ident;

            if matches!(&variant.fields, syn::Fields::Unit if !is_option_spanned) {
                self.cx.error(syn::Error::new_spanned(
                    variant,
                    "Spanned cannot be implemented for unit variants",
                ));
                continue;
            }

            let mut assign = Vec::new();

            for (index, field) in variant.fields.iter().enumerate() {
                let member = match &field.ident {
                    Some(ident) => syn::Member::Named(ident.clone()),
                    None => syn::Member::Unnamed(syn::Index::from(index)),
                };

                let to = match &field.ident {
                    Some(ident) => ident.clone(),
                    None => format_ident!("_{}", index),
                };

                assign.push(syn::FieldValue {
                    attrs: Vec::new(),
                    member,
                    colon_token: Some(<Token![:]>::default()),
                    expr: syn::Expr::Path(syn::ExprPath {
                        attrs: Vec::new(),
                        qself: None,
                        path: syn::Path::from(to),
                    }),
                });
            }

            if let Ok(body) = self.expand_struct_fields(
                &variant.fields,
                |member| match member {
                    syn::Member::Named(field) => quote!(#field),
                    syn::Member::Unnamed(index) => format_ident!("_{}", index).into_token_stream(),
                },
                is_option_spanned,
            ) {
                variants.push(quote! {
                    Self::#ident { #(#assign),* } => { #body }
                });
            }
        }

        if self.cx.has_errors() {
            return Err(());
        }

        Ok(quote! {
            match self {
                #(#variants,)*
            }
        })
    }

    /// Expand field decoding.
    fn expand_struct_fields(
        &mut self,
        fields: &syn::Fields,
        access_member: fn(&syn::Member) -> TokenStream,
        is_option_spanned: bool,
    ) -> Result<TokenStream, ()> {
        let mut explicit_span = None;

        let Tokens {
            spanned,
            into_iterator,
            span,
            option,
            option_spanned,
            iterator,
            double_ended_iterator,
            ..
        } = &self.tokens;

        let mut out = None;
        let mut definite_span = false;

        for (index, field) in fields.iter().enumerate() {
            let attr = self.cx.field_attrs(&field.attrs)?;

            if attr.id.is_some() || attr.skip.is_some() {
                continue;
            }

            let member = match &field.ident {
                Some(ident) => syn::Member::Named(ident.clone()),
                None => syn::Member::Unnamed(syn::Index::from(index)),
            };

            if let Some(span) = attr.span {
                if explicit_span.is_some() {
                    self.cx.error(syn::Error::new(
                        span,
                        "Only one field can be marked `#[rune(span)]`",
                    ));
                    return Err(());
                }

                explicit_span = Some(member.clone());
            }

            let access = access_member(&member);

            let next = if attr.iter.is_some() {
                quote! {
                    #iterator::map(#into_iterator::into_iter(#access), #spanned::span)
                }
            } else if attr.option.is_some() {
                quote! {
                    #iterator::flat_map(#into_iterator::into_iter([#access]), #option_spanned::option_span)
                }
            } else {
                definite_span = true;

                quote! {
                    #iterator::map(#into_iterator::into_iter([#access]), #spanned::span)
                }
            };

            out = Some(match out.take() {
                Some(out) => quote!(#iterator::chain(#out, #next)),
                None => next,
            });
        }

        if let Some(explicit_span) = explicit_span {
            let access = access_member(&explicit_span);

            if is_option_spanned {
                return Ok(quote!(#option::Some(#spanned::span(#access))));
            } else {
                return Ok(quote!(#spanned::span(#access)));
            }
        }

        let match_head_back = if is_option_spanned {
            quote! {
                match (head, back) {
                    (#option::Some(head), #option::Some(back)) => #option::Some(#span::join(head, back)),
                    (#option::Some(head), #option::None) => #option::Some(head),
                    (#option::None, #option::Some(back)) => #option::Some(back),
                    _ => None,
                }
            }
        } else {
            if !definite_span {
                self.cx.error(syn::Error::new_spanned(
                    fields,
                    "No field available that can definitely produce a `Span` from",
                ));

                return Err(());
            }

            quote! {
                match (head, back) {
                    (#option::Some(head), #option::Some(back)) => #span::join(head, back),
                    (#option::Some(head), #option::None) => head,
                    (#option::None, #option::Some(back)) => back,
                    _ => unreachable!(),
                }
            }
        };

        let Some(out) = out else {
            return Ok(quote!(#option::None));
        };

        Ok(quote! {
            let mut iter = #out;
            let head: #option<#span> = #iterator::next(&mut iter);
            let back: #option<#span> = #double_ended_iterator::next_back(&mut iter);
            #match_head_back
        })
    }
}
