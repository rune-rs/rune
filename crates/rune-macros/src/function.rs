use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::Error;

pub(crate) struct Function {
    stream: TokenStream,
}

impl Function {
    /// Parse the given parse stream.
    pub(crate) fn parse(input: ParseStream) -> Result<Self, Error> {
        let attributes = input.call(syn::Attribute::parse_outer)?;
        let vis = input.parse::<syn::Visibility>()?;
        let mut sig = input.parse::<syn::Signature>()?;
        let ident = sig.ident.clone();

        let mut stream = TokenStream::new();

        let mut docs = syn::ExprArray {
            attrs: Vec::new(),
            bracket_token: syn::token::Bracket::default(),
            elems: Punctuated::default(),
        };

        for attr in attributes {
            if attr.path().is_ident("doc") {
                if let syn::Meta::NameValue(name_value) = &attr.meta {
                    docs.elems.push(name_value.value.clone());
                }
            }

            stream.extend(attr.into_token_stream());
        }

        let name_string = syn::LitStr::new(&ident.to_string(), ident.span());

        let mut arguments = syn::ExprArray {
            attrs: Vec::new(),
            bracket_token: syn::token::Bracket::default(),
            elems: Punctuated::default(),
        };

        let mut takes_self = false;

        let mut segments = Punctuated::default();

        segments.push(syn::PathSegment {
            ident: syn::Ident::new("Self", sig.span()),
            arguments: syn::PathArguments::None,
        });

        let self_elem = syn::TypePath {
            qself: None,
            path: syn::Path {
                leading_colon: None,
                segments,
            },
        };

        for arg in &sig.inputs {
            let argument_name = match arg {
                syn::FnArg::Typed(ty) => {
                    let argument_name = match ty.pat.as_ref() {
                        syn::Pat::Path(path) => match path.path.get_ident() {
                            Some(ident) => syn::LitStr::new(&ident.to_string(), arg.span()),
                            None => syn::LitStr::new("", arg.span()),
                        },
                        _ => syn::LitStr::new("", arg.span()),
                    };

                    argument_name
                }
                syn::FnArg::Receiver(..) => {
                    takes_self = true;
                    syn::LitStr::new("self", arg.span())
                }
            };

            arguments.elems.push(syn::Expr::Lit(syn::ExprLit {
                attrs: Vec::new(),
                lit: syn::Lit::Str(argument_name),
            }));
        }

        let meta_vis = vis.clone();
        let meta_fn = sig.ident.clone();
        let real_fn = syn::Ident::new(&format!("__rune_fn__{}", sig.ident), sig.ident.span());
        sig.ident = real_fn.clone();

        let (real_fn_path, meta_kind, meta_name) = if takes_self {
            let mut path = self_elem;

            path.path.segments.push(syn::PathSegment {
                ident: real_fn,
                arguments: syn::PathArguments::None,
            });

            let meta_kind = if sig.asyncness.is_some() {
                quote!(async_instance)
            } else {
                quote!(instance)
            };

            (path, meta_kind, quote!(#name_string))
        } else {
            let mut segments = Punctuated::default();

            segments.push(syn::PathSegment {
                ident: real_fn,
                arguments: syn::PathArguments::None,
            });

            let path = syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            };

            let meta_kind = if sig.asyncness.is_some() {
                quote!(async_function)
            } else {
                quote!(function)
            };

            (path, meta_kind, quote!([#name_string]))
        };

        stream.extend(quote!(#[allow(non_snake_case)]));
        stream.extend(vis.into_token_stream());
        stream.extend(sig.into_token_stream());
        stream.extend(input.parse::<TokenStream>()?);

        stream.extend(quote! {
            /// Get function metadata.
            #[automatically_derived]
            #meta_vis fn #meta_fn() -> rune::compile::FunctionMetaData {
                rune::compile::FunctionMetaData {
                    kind: rune::compile::FunctionMetaKind::#meta_kind(#meta_name, #real_fn_path),
                    name: #name_string,
                    docs: &#docs[..],
                    arguments: &#arguments[..],
                }
            }
        });

        Ok(Self { stream })
    }

    /// Expand the function declaration.
    pub(crate) fn expand(self) -> Result<TokenStream, Error> {
        Ok(self.stream)
    }
}
