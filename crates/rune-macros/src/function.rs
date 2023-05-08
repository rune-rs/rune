use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Error, Token};

#[derive(Default)]
enum Path {
    #[default]
    None,
    Path(syn::Path),
}

#[derive(Default)]
pub(crate) struct FunctionAttrs {
    instance: bool,
    /// Keep the existing function in place, and generate a separate hidden meta function.
    keep: bool,
    path: Path,
    /// Looks like an associated type.
    assoc: bool,
}

impl FunctionAttrs {
    /// Parse the given parse stream.
    pub(crate) fn parse(input: ParseStream) -> Result<Self, Error> {
        let mut out = Self::default();

        while !input.is_empty() {
            let ident = input.parse::<syn::Ident>()?;

            if ident == "instance" {
                out.instance = true;
            } else if ident == "keep" {
                out.keep = true;
            } else if ident == "path" {
                input.parse::<Token![=]>()?;

                if input.peek(Token![Self]) {
                    out.assoc = true;
                }

                out.path = Path::Path(input.parse()?);
            } else {
                return Err(syn::Error::new_spanned(ident, "Unsupported option"));
            }

            if input.parse::<Option<Token![,]>>()?.is_none() {
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

pub(crate) struct Function {
    attributes: Vec<syn::Attribute>,
    vis: syn::Visibility,
    sig: syn::Signature,
    remainder: TokenStream,
    name_string: syn::LitStr,
    docs: syn::ExprArray,
    arguments: syn::ExprArray,
    takes_self: bool,
}

impl Function {
    /// Parse the given parse stream.
    pub(crate) fn parse(input: ParseStream) -> Result<Self, Error> {
        let parsed_attributes = input.call(syn::Attribute::parse_outer)?;
        let vis = input.parse::<syn::Visibility>()?;
        let sig = input.parse::<syn::Signature>()?;
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
        })
    }

    /// Expand the function declaration.
    pub(crate) fn expand(mut self, attrs: FunctionAttrs) -> Result<TokenStream, Error> {
        let (meta_fn, real_fn, sig, real_fn_mangled) = if attrs.keep {
            let meta_fn = syn::Ident::new(
                &format!("__{}__meta", self.sig.ident),
                self.sig.ident.span(),
            );
            let real_fn = self.sig.ident.clone();
            (meta_fn, real_fn, self.sig.clone(), false)
        } else {
            let meta_fn = self.sig.ident.clone();
            let real_fn = syn::Ident::new(
                &format!("__rune_fn__{}", self.sig.ident),
                self.sig.ident.span(),
            );
            let mut sig = self.sig.clone();
            sig.ident = real_fn.clone();
            (meta_fn, real_fn, sig, true)
        };

        let self_type = if !attrs.instance {
            match &attrs.path {
                Path::Path(path) if path.segments.len() == 2 => match path.segments.first() {
                    Some(segment) => Some(&segment.ident),
                    None => {
                        return Err(syn::Error::new_spanned(
                            path,
                            "First segment required for associated functions",
                        ))
                    }
                },
                _ => None,
            }
        } else {
            None
        };

        let real_fn_path = if self.takes_self || attrs.assoc {
            let mut path = syn::Path {
                leading_colon: None,
                segments: Punctuated::default(),
            };

            path.segments.push(syn::PathSegment::from(syn::Ident::new(
                "Self",
                self.sig.span(),
            )));
            path.segments.push(syn::PathSegment::from(real_fn));

            syn::TypePath { qself: None, path }
        } else {
            let mut path = syn::Path {
                leading_colon: None,
                segments: Punctuated::default(),
            };

            path.segments.push(syn::PathSegment::from(real_fn));

            syn::TypePath { qself: None, path }
        };

        let (instance, meta_name) = if attrs.instance || self.takes_self {
            (true, self.name_string.to_token_stream())
        } else {
            let stream = match &attrs.path {
                Path::None => {
                    let name_string = &self.name_string;
                    quote!([#name_string])
                }
                Path::Path(path) => {
                    let mut out = syn::ExprArray {
                        attrs: Vec::new(),
                        bracket_token: syn::token::Bracket::default(),
                        elems: Punctuated::default(),
                    };

                    let skip = if path.segments.len() <= 1 { 0 } else { 1 };

                    for segment in path.segments.iter().skip(skip) {
                        let ident =
                            syn::LitStr::new(&segment.ident.to_string(), segment.ident.span());

                        out.elems.push(syn::Expr::Lit(syn::ExprLit {
                            attrs: Vec::new(),
                            lit: syn::Lit::Str(ident),
                        }));
                    }

                    out.into_token_stream()
                }
            };

            (false, stream)
        };

        let function = match (instance, self_type.is_some(), self.sig.asyncness.is_some()) {
            (true, _, false) => "instance",
            (true, _, true) => "async_instance",
            (_, true, false) => "function_with",
            (_, true, true) => "async_function_with",
            (_, _, false) => "function",
            (_, _, true) => "async_function",
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

        if real_fn_mangled {
            stream.extend(quote!(#[allow(non_snake_case)]));
        }

        stream.extend(self.vis.to_token_stream());
        stream.extend(sig.into_token_stream());
        stream.extend(self.remainder);

        let arguments = &self.arguments;
        let docs = &self.docs;
        let name_string = self.name_string;

        let meta_kind = if let Some(self_type) = self_type {
            quote!(#meta_kind::<#self_type, _, _, _>)
        } else {
            meta_kind.into_token_stream()
        };

        let meta_vis = &self.vis;

        let attr = (!real_fn_mangled).then(|| quote!(#[allow(non_snake_case)] #[doc(hidden)]));

        stream.extend(quote! {
            /// Get function metadata.
            #[automatically_derived]
            #attr
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
