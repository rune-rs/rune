use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Parse;

use crate::context::{Context, Tokens};

pub(super) fn expand(mut input: syn::DeriveInput) -> Result<TokenStream, Vec<syn::Error>> {
    let cx = Context::new();
    let tokens = cx.tokens_with_module(None);

    let attr = parse_type_attr(&cx, &input.attrs);

    if !attr.predicates.is_empty() {
        input
            .generics
            .make_where_clause()
            .predicates
            .extend(attr.predicates);
    }

    let Tokens {
        try_clone, alloc, ..
    } = &tokens;

    let implementation = if attr.copy {
        quote!(*self)
    } else {
        match input.data {
            syn::Data::Struct(st) => {
                let fields = st.fields.into_iter().enumerate().map(|(index, f)| {
                    let member = match &f.ident {
                        Some(ident) => syn::Member::Named(ident.clone()),
                        None => syn::Member::Unnamed(syn::Index::from(index)),
                    };

                    let attr = parse_field_attr(&cx, &f.attrs);

                    let expr = match attr.with {
                        With::Copy => quote! { self.#member },
                        With::None => quote! { #try_clone::try_clone(&self.#member)? },
                        With::With(with) => quote! { #with(&self.#member) },
                        With::TryWith(with) => quote! { #with(&self.#member)? },
                    };

                    syn::FieldValue {
                        attrs: Vec::new(),
                        member,
                        colon_token: Some(<syn::Token![:]>::default()),
                        expr: syn::Expr::Verbatim(expr),
                    }
                });

                quote! {
                    Self { #(#fields),* }
                }
            }
            syn::Data::Enum(en) => {
                let variants = en.variants.into_iter().map(|v| {
                    let name = v.ident;

                    let members = v.fields.iter().enumerate().map(|(index, f)| {
                        let (member, var) = match &f.ident {
                            Some(ident) => (
                                syn::Member::Named(ident.clone()),
                                quote::format_ident!("{}", ident),
                            ),
                            None => (
                                syn::Member::Unnamed(syn::Index::from(index)),
                                quote::format_ident!("_{}", index),
                            ),
                        };

                        let attr = parse_field_attr(&cx, &f.attrs);
                        (index, f, member, var, attr)
                    });

                    let assigns =
                        members
                            .clone()
                            .map(|(index, f, member, var, _)| match &f.ident {
                                Some(..) => syn::FieldValue {
                                    attrs: Vec::new(),
                                    member,
                                    colon_token: None,
                                    expr: syn::Expr::Verbatim(quote!()),
                                },
                                None => {
                                    let member = syn::Member::Unnamed(syn::Index::from(index));

                                    let expr = syn::Expr::Path(syn::ExprPath {
                                        attrs: Vec::new(),
                                        qself: None,
                                        path: syn::Path::from(var),
                                    });

                                    syn::FieldValue {
                                        attrs: Vec::new(),
                                        member,
                                        colon_token: Some(<syn::Token![:]>::default()),
                                        expr,
                                    }
                                }
                            });

                    let fields = members.clone().map(|(_, _, member, var, attr)| {
                        let expr = match attr.with {
                            With::Copy => quote! { *#var },
                            With::None => quote! { #try_clone::try_clone(#var)? },
                            With::With(with) => quote! { #with(#var) },
                            With::TryWith(with) => quote! { #with(#var)? },
                        };

                        syn::FieldValue {
                            attrs: Vec::new(),
                            member,
                            colon_token: Some(<syn::Token![:]>::default()),
                            expr: syn::Expr::Verbatim(expr),
                        }
                    });

                    quote! {
                        Self::#name { #(#assigns),* } => {
                            Self::#name { #(#fields),* }
                        }
                    }
                });

                quote! {
                    match self {
                        #(#variants),*
                    }
                }
            }
            syn::Data::Union(un) => {
                cx.error(syn::Error::new_spanned(
                    un.union_token,
                    "TryClone: Unions are not supported",
                ));
                quote!()
            }
        }
    };

    if cx.has_errors() {
        return Err(cx.into_errors());
    }

    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics #try_clone for #name #ty_generics #where_clause {
            fn try_clone(&self) -> #alloc::Result<Self> {
                Ok(#implementation)
            }
        }
    })
}

#[derive(Default)]
struct TypeAttr {
    predicates: syn::punctuated::Punctuated<syn::WherePredicate, syn::Token![,]>,
    copy: bool,
}

fn parse_type_attr(cx: &Context, input: &[syn::Attribute]) -> TypeAttr {
    let mut attr = TypeAttr::default();

    for a in input {
        if !a.path().is_ident("try_clone") {
            continue;
        }

        let result = a.parse_nested_meta(|parser| {
            if parser.path.is_ident("bound") {
                parser.input.parse::<syn::Token![=]>()?;
                let content;
                syn::braced!(content in parser.input);
                attr.predicates
                    .extend(content.parse_terminated(syn::WherePredicate::parse, syn::Token![,])?);
                return Ok(());
            }

            if parser.path.is_ident("copy") {
                attr.copy = true;
                return Ok(());
            }

            Err(syn::Error::new(
                parser.input.span(),
                "unsupported attribute",
            ))
        });

        if let Err(error) = result {
            cx.error(error);
        }
    }

    attr
}

#[derive(Default, Clone)]
enum With {
    #[default]
    None,
    Copy,
    With(syn::Path),
    TryWith(syn::Path),
}

#[derive(Default, Clone)]
struct FieldAttr {
    with: With,
}

fn parse_field_attr(cx: &Context, input: &[syn::Attribute]) -> FieldAttr {
    let mut attr = FieldAttr::default();

    for a in input {
        if !a.path().is_ident("try_clone") {
            continue;
        }

        let result = a.parse_nested_meta(|parser| {
            if parser.path.is_ident("with") {
                parser.input.parse::<syn::Token![=]>()?;
                attr.with = With::With(parser.input.parse()?);
                return Ok(());
            }

            if parser.path.is_ident("try_with") {
                parser.input.parse::<syn::Token![=]>()?;
                attr.with = With::TryWith(parser.input.parse()?);
                return Ok(());
            }

            if parser.path.is_ident("copy") {
                attr.with = With::Copy;
                return Ok(());
            }

            Err(syn::Error::new(
                parser.input.span(),
                "unsupported attribute",
            ))
        });

        if let Err(error) = result {
            cx.error(error);
        }
    }

    attr
}
