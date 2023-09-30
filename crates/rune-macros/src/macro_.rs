use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

#[derive(Default)]
enum Path {
    #[default]
    None,
    Path(Span, syn::Path),
}

#[derive(Default)]
pub(crate) struct Config {
    path: Path,
}

impl Config {
    /// Parse the given parse stream.
    pub(crate) fn parse(input: ParseStream) -> syn::Result<Self> {
        let span = input.span();
        let mut out = Self::default();

        while !input.is_empty() {
            let ident = input.parse::<syn::Ident>()?;

            if ident == "path" {
                input.parse::<syn::Token![=]>()?;
                out.path = Path::Path(span, input.parse()?);
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

pub(crate) struct Macro {
    attributes: Vec<syn::Attribute>,
    vis: syn::Visibility,
    sig: syn::Signature,
    remainder: TokenStream,
    name_string: syn::LitStr,
    docs: syn::ExprArray,
    meta_vis: syn::Visibility,
    real_fn: syn::Ident,
    meta_fn: syn::Ident,
}

impl Macro {
    /// Parse the given parse stream.
    pub(crate) fn parse(input: ParseStream) -> syn::Result<Self> {
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

        let meta_vis = vis.clone();
        let meta_fn = sig.ident.clone();
        let real_fn = syn::Ident::new(&format!("__rune_macro__{}", sig.ident), sig.ident.span());
        sig.ident = real_fn.clone();

        let remainder = input.parse::<TokenStream>()?;

        Ok(Self {
            attributes,
            vis,
            sig,
            remainder,
            name_string,
            docs,
            meta_vis,
            real_fn,
            meta_fn,
        })
    }

    /// Expand the function declaration.
    pub(crate) fn expand(self, attrs: Config, macro_kind: Ident) -> syn::Result<TokenStream> {
        let real_fn_path = {
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

        let meta_name = syn::Expr::Array({
            let mut meta_name = syn::ExprArray {
                attrs: Vec::new(),
                bracket_token: syn::token::Bracket::default(),
                elems: Punctuated::default(),
            };

            match attrs.path {
                Path::None => {
                    meta_name.elems.push(syn::Expr::Lit(syn::ExprLit {
                        attrs: Vec::new(),
                        lit: syn::Lit::Str(self.name_string.clone()),
                    }));
                }
                Path::Path(_, path) => {
                    for s in &path.segments {
                        let syn::PathArguments::None = s.arguments else {
                            return Err(syn::Error::new_spanned(
                                s,
                                "Expected simple ident path segment",
                            ));
                        };

                        let ident = syn::LitStr::new(&s.ident.to_string(), s.span());

                        meta_name.elems.push(syn::Expr::Lit(syn::ExprLit {
                            attrs: Vec::new(),
                            lit: syn::Lit::Str(ident),
                        }));
                    }
                }
            }

            meta_name
        });

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
        let docs = &self.docs;
        let name_string = self.name_string;

        stream.extend(quote! {
            /// Get function metadata.
            #[automatically_derived]
            #meta_vis fn #meta_fn() -> rune::alloc::Result<rune::__private::MacroMetaData> {
                Ok(rune::__private::MacroMetaData {
                    kind: rune::__private::MacroMetaKind::#macro_kind(#meta_name, #real_fn_path)?,
                    name: #name_string,
                    docs: &#docs[..],
                })
            }
        });

        Ok(stream)
    }
}
