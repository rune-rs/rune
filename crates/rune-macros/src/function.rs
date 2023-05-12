use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Error, Token};

#[derive(Default)]
enum Path {
    #[default]
    None,
    Instance(syn::Ident, syn::PathSegment),
    Rename(syn::PathSegment),
}

#[derive(Default)]
pub(crate) struct FunctionAttrs {
    instance: bool,
    /// Keep the existing function in place, and generate a separate hidden meta function.
    keep: bool,
    /// Path to register in.
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

                let path: syn::Path = input.parse()?;

                if path.segments.len() > 2 {
                    return Err(syn::Error::new_spanned(
                        path,
                        "Expected at most two path segments",
                    ));
                }

                let mut it = path.segments.into_iter();

                let Some(first) = it.next() else {
                    return Err(syn::Error::new(input.span(), "Expected at least one path segment"));
                };

                if let Some(second) = it.next() {
                    let syn::PathArguments::None = &first.arguments else {
                        return Err(syn::Error::new_spanned(first.arguments, "Unsupported arguments"));
                    };

                    out.path = Path::Instance(first.ident, second);
                } else {
                    out.path = Path::Rename(first);
                }
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
                Path::Instance(self_type, _) => Some(self_type.clone()),
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

        let name_string = syn::LitStr::new(&self.sig.ident.to_string(), self.sig.ident.span());

        let (instance, mut name, arguments) = if attrs.instance || self.takes_self {
            let (name, arguments) = match &attrs.path {
                Path::None => (name_string.clone(), Punctuated::default()),
                Path::Rename(last) | Path::Instance(_, last) => {
                    let name = syn::LitStr::new(&last.ident.to_string(), last.ident.span());

                    let arguments = match &last.arguments {
                        syn::PathArguments::AngleBracketed(arguments) => arguments.args.clone(),
                        syn::PathArguments::None => Punctuated::default(),
                        arguments => {
                            return Err(syn::Error::new_spanned(
                                arguments,
                                "Unsupported path segments",
                            ));
                        }
                    };

                    (name, arguments)
                }
            };

            let name = syn::Expr::Lit(syn::ExprLit {
                attrs: Vec::new(),
                lit: syn::Lit::Str(name),
            });

            (true, name, arguments)
        } else {
            let (name, arguments) = match &attrs.path {
                Path::None => {
                    let name = syn::Expr::Lit(syn::ExprLit {
                        attrs: Vec::new(),
                        lit: syn::Lit::Str(syn::LitStr::new(
                            &self.sig.ident.to_string(),
                            self.sig.ident.span(),
                        )),
                    });

                    (name, Punctuated::default())
                }
                Path::Rename(last) | Path::Instance(_, last) => {
                    let name = syn::LitStr::new(&last.ident.to_string(), last.ident.span());

                    let arguments = match &last.arguments {
                        syn::PathArguments::AngleBracketed(arguments) => arguments.args.clone(),
                        syn::PathArguments::None => Punctuated::default(),
                        arguments => {
                            return Err(syn::Error::new_spanned(
                                arguments,
                                "Unsupported path segments",
                            ));
                        }
                    };

                    let name = syn::Expr::Lit(syn::ExprLit {
                        attrs: Vec::new(),
                        lit: syn::Lit::Str(name),
                    });

                    (name, arguments)
                }
            };

            (false, name, arguments)
        };

        let function = match (instance, self.sig.asyncness.is_some()) {
            (true, false) => "instance",
            (true, true) => "async_instance",
            (_, false) => "function",
            (_, true) => "async_function",
        };

        if !instance && self_type.is_none() {
            name = {
                let mut out = syn::ExprArray {
                    attrs: Vec::new(),
                    bracket_token: syn::token::Bracket::default(),
                    elems: Punctuated::default(),
                };

                out.elems.push(name);
                syn::Expr::Array(out)
            };
        }

        let name = if !arguments.is_empty() {
            let mut array = syn::ExprArray {
                attrs: Vec::new(),
                bracket_token: <syn::token::Bracket>::default(),
                elems: Punctuated::default(),
            };

            for argument in arguments {
                array.elems.push(syn::Expr::Verbatim(quote_spanned! {
                    argument.span() =>
                    <#argument as rune::__private::TypeOf>::type_of()
                }));
            }

            quote!(rune::__private::Params::new(#name, #array))
        } else {
            quote!(#name)
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

        let build_with = if instance {
            None
        } else if let Some(self_type) = self_type {
            Some(quote!(.build_associated::<#self_type>()))
        } else {
            Some(quote!(.build()))
        };

        let meta_vis = &self.vis;

        let attr = (!real_fn_mangled).then(|| quote!(#[allow(non_snake_case)] #[doc(hidden)]));

        stream.extend(quote! {
            /// Get function metadata.
            #[automatically_derived]
            #attr
            #meta_vis fn #meta_fn() -> rune::__private::FunctionMetaData {
                rune::__private::FunctionMetaData {
                    kind: rune::__private::FunctionMetaKind::#meta_kind(#name, #real_fn_path)#build_with,
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
