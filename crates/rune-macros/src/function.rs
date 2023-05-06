use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::Error;

#[derive(Default)]
enum Path {
    #[default]
    None,
    Path(Span, Option<syn::token::SelfType>, Vec<syn::Ident>),
}

impl Path {
    fn is_self(&self) -> bool {
        match self {
            Path::Path(_, self_type, _) => self_type.is_some(),
            _ => false,
        }
    }
}

#[derive(Default)]
pub(crate) struct FunctionAttrs {
    instance: bool,
    path: Path,
}

impl FunctionAttrs {
    /// Parse the given parse stream.
    pub(crate) fn parse(input: ParseStream) -> Result<Self, Error> {
        let span = input.span();
        let mut out = Self::default();

        while !input.is_empty() {
            let ident = input.parse::<syn::Ident>()?;

            if ident == "instance" {
                out.instance = true;
            } else if ident == "path" {
                input.parse::<syn::Token![=]>()?;

                let (self_type, head) = if input.peek(syn::token::SelfType) {
                    let self_type = input.parse()?;
                    (Some(self_type), None)
                } else {
                    (None, Some(input.parse()?))
                };

                out.path = Path::Path(span, self_type, parse_path(input, head)?);
            } else {
                return Err(syn::Error::new_spanned(ident, "Unsupported option"));
            }

            if input.parse::<Option<syn::Token![,]>>()?.is_none() {
                break;
            }
        }

        let stream = input.parse::<TokenStream>()?;

        if !stream.is_empty() {
            return Err(syn::Error::new_spanned(stream, "Unexpected input"));
        }

        Ok(out)
    }
}

/// Parse `#[rune::function(path = <value>)]`.
fn parse_path(input: ParseStream, head: Option<syn::Ident>) -> Result<Vec<syn::Ident>, Error> {
    let mut array = Vec::new();
    array.extend(head);

    while input.peek(syn::Token![::]) {
        input.parse::<syn::Token![::]>()?;
        array.push(input.parse()?);
    }

    if input.peek(syn::Ident) {
        array.push(input.parse()?);
    }

    Ok(array)
}

pub(crate) struct Function {
    attributes: Vec<syn::Attribute>,
    vis: syn::Visibility,
    sig: syn::Signature,
    remainder: TokenStream,
    name_string: syn::LitStr,
    docs: syn::ExprArray,
    arguments: syn::ExprArray,
    takes_self: bool,
    meta_vis: syn::Visibility,
    real_fn: syn::Ident,
    meta_fn: syn::Ident,
}

impl Function {
    /// Parse the given parse stream.
    pub(crate) fn parse(input: ParseStream) -> Result<Self, Error> {
        let parsed_attributes = input.call(syn::Attribute::parse_outer)?;
        let vis = input.parse::<syn::Visibility>()?;
        let mut sig = input.parse::<syn::Signature>()?;
        let ident = sig.ident.clone();

        let mut attributes = Vec::new();

        let mut docs = syn::ExprArray {
            attrs: Vec::new(),
            bracket_token: syn::token::Bracket::default(),
            elems: Punctuated::default(),
        };

        for attr in parsed_attributes {
            if attr.path().is_ident("doc") {
                if let syn::Meta::NameValue(name_value) = &attr.meta {
                    docs.elems.push(name_value.value.clone());
                }
            }

            attributes.push(attr);
        }

        let name_string = syn::LitStr::new(&ident.to_string(), ident.span());

        let mut arguments = syn::ExprArray {
            attrs: Vec::new(),
            bracket_token: syn::token::Bracket::default(),
            elems: Punctuated::default(),
        };

        let mut takes_self = false;

        for arg in &sig.inputs {
            let argument_name = match arg {
                syn::FnArg::Typed(ty) => argument_ident(&ty.pat),
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

        let remainder = input.parse::<TokenStream>()?;

        Ok(Self {
            attributes,
            vis,
            sig,
            remainder,
            name_string,
            docs,
            arguments,
            takes_self,
            meta_vis,
            real_fn,
            meta_fn,
        })
    }

    /// Expand the function declaration.
    pub(crate) fn expand(mut self, attrs: FunctionAttrs) -> Result<TokenStream, Error> {
        let real_fn_path = if attrs.path.is_self() || self.takes_self {
            let mut segments = Punctuated::default();

            segments.push(syn::PathSegment {
                ident: syn::Ident::new("Self", self.sig.span()),
                arguments: syn::PathArguments::None,
            });

            let mut path = syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            };

            path.path.segments.push(syn::PathSegment {
                ident: self.real_fn,
                arguments: syn::PathArguments::None,
            });

            path
        } else {
            let mut segments = Punctuated::default();

            segments.push(syn::PathSegment {
                ident: self.real_fn,
                arguments: syn::PathArguments::None,
            });

            syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            }
        };

        let (instance, meta_name, self_type) = if attrs.instance || self.takes_self {
            match attrs.path {
                Path::None => (true, self.name_string.to_token_stream(), None),
                Path::Path(span, self_type, array) => {
                    if array.is_empty() {
                        return Err(syn::Error::new(
                            span,
                            "paths for instance functions must only take `Self`",
                        ));
                    }

                    (true, self.name_string.to_token_stream(), self_type)
                }
            }
        } else {
            match attrs.path {
                Path::None => {
                    let name_string = self.name_string.clone();
                    (false, quote!([#name_string]), None)
                }
                Path::Path(_, self_type, array) => {
                    let mut out = syn::ExprArray {
                        attrs: Vec::new(),
                        bracket_token: syn::token::Bracket::default(),
                        elems: Punctuated::default(),
                    };

                    for ident in array {
                        let ident = syn::LitStr::new(&ident.to_string(), ident.span());

                        out.elems.push(syn::Expr::Lit(syn::ExprLit {
                            attrs: Vec::new(),
                            lit: syn::Lit::Str(ident),
                        }));
                    }

                    (false, out.into_token_stream(), self_type)
                }
            }
        };

        let function = match (instance, self_type, self.sig.asyncness.is_some()) {
            (true, _, false) => "instance",
            (true, _, true) => "async_instance",
            (_, Some(_), false) => "function_with",
            (_, Some(_), true) => "async_function_with",
            (_, None, false) => "function",
            (_, None, true) => "async_function",
        };

        if instance && !self.takes_self {
            // Ensure that the first argument is called `self`.
            if let Some(argument) = self.arguments.elems.first_mut() {
                let span = argument.span();

                *argument = syn::Expr::Lit(syn::ExprLit {
                    attrs: Vec::new(),
                    lit: syn::Lit::Str(syn::LitStr::new("self", span)),
                });
            }
        }

        let meta_kind = syn::Ident::new(function, self.sig.span());
        let mut stream = TokenStream::new();

        for attr in self.attributes {
            stream.extend(attr.into_token_stream());
        }

        stream.extend(quote!(#[allow(non_snake_case)]));
        stream.extend(self.vis.into_token_stream());
        stream.extend(self.sig.into_token_stream());
        stream.extend(self.remainder);

        let meta_vis = &self.meta_vis;
        let meta_fn = &self.meta_fn;
        let arguments = &self.arguments;
        let docs = &self.docs;
        let name_string = self.name_string;

        let meta_kind = if let Some(self_type) = self_type {
            quote!(#meta_kind::<#self_type, _, _, _>)
        } else {
            meta_kind.into_token_stream()
        };

        stream.extend(quote! {
            /// Get function metadata.
            #[automatically_derived]
            #meta_vis fn #meta_fn() -> rune::__private::FunctionMetaData {
                rune::__private::FunctionMetaData {
                    kind: rune::__private::FunctionMetaKind::#meta_kind(#meta_name, #real_fn_path),
                    name: #name_string,
                    docs: &#docs[..],
                    arguments: &#arguments[..],
                }
            }
        });

        Ok(stream)
    }
}

/// The identifier of an argument.
fn argument_ident(pat: &syn::Pat) -> syn::LitStr {
    match pat {
        syn::Pat::Type(pat) => argument_ident(&pat.pat),
        syn::Pat::Path(pat) => argument_path_ident(&pat.path),
        syn::Pat::Ident(pat) => syn::LitStr::new(&pat.ident.to_string(), pat.span()),
        _ => syn::LitStr::new(&pat.to_token_stream().to_string(), pat.span()),
    }
}

/// Argument path identifier.
fn argument_path_ident(path: &syn::Path) -> syn::LitStr {
    match path.get_ident() {
        Some(ident) => syn::LitStr::new(&ident.to_string(), path.span()),
        None => syn::LitStr::new(&path.to_token_stream().to_string(), path.span()),
    }
}
