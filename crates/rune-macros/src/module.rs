use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;

pub(crate) struct ModuleAttrs {
    path: syn::Path,
}

impl ModuleAttrs {
    /// Parse the given parse stream.
    pub(crate) fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = input.parse::<syn::Path>()?;
        let stream = input.parse::<TokenStream>()?;

        if !stream.is_empty() {
            return Err(syn::Error::new_spanned(stream, "Unexpected input"));
        }

        Ok(Self { path })
    }
}

pub(crate) struct Module {
    attributes: Vec<syn::Attribute>,
    docs: syn::ExprArray,
    remainder: TokenStream,
}

impl Module {
    /// Parse the given parse stream.
    pub(crate) fn parse(input: ParseStream) -> syn::Result<Self> {
        let parsed_attributes = input.call(syn::Attribute::parse_outer)?;

        let mut docs = syn::ExprArray {
            attrs: Vec::new(),
            bracket_token: syn::token::Bracket::default(),
            elems: Punctuated::default(),
        };

        let mut attributes = Vec::new();

        for attr in parsed_attributes {
            if attr.path().is_ident("doc") {
                if let syn::Meta::NameValue(name_value) = &attr.meta {
                    docs.elems.push(name_value.value.clone());
                }
            }

            attributes.push(attr);
        }

        let remainder = input.parse::<TokenStream>()?;

        Ok(Self {
            attributes,
            docs,
            remainder,
        })
    }

    /// Expand the function declaration.
    pub(crate) fn expand(self, attrs: ModuleAttrs) -> syn::Result<TokenStream> {
        let docs = self.docs;

        let item = match attrs.path.leading_colon {
            Some(..) => {
                let mut it = attrs.path.segments.iter();

                let Some(krate) = it.next() else {
                    return Err(syn::Error::new_spanned(
                        &attrs.path,
                        "missing leading segment",
                    ));
                };

                let krate = syn::LitStr::new(&krate.ident.to_string(), krate.ident.span());
                let item = build_item(it);

                if item.elems.is_empty() {
                    quote!(rune::__private::ItemBuf::with_crate(#krate)?)
                } else {
                    quote!(rune::__private::ItemBuf::with_crate_item(#krate, #item)?)
                }
            }
            None => {
                let item = build_item(attrs.path.segments.iter());

                if item.elems.is_empty() {
                    quote!(rune::__private::ItemBuf::new()?)
                } else {
                    quote!(rune::__private::ItemBuf::from_item(#item)?)
                }
            }
        };

        let mut stream = TokenStream::new();

        stream.extend(quote! {
            /// Get module metadata.
            #[automatically_derived]
            #[doc(hidden)]
            fn module_meta() -> rune::alloc::Result<rune::__private::ModuleMetaData> {
                Ok(rune::__private::ModuleMetaData {
                    docs: &#docs[..],
                    item: #item,
                })
            }
        });

        stream.extend(quote!(#[allow(rustdoc::broken_intra_doc_links)]));

        for attribute in self.attributes {
            attribute.to_tokens(&mut stream);
        }

        stream.extend(self.remainder);
        Ok(stream)
    }
}

fn build_item(it: syn::punctuated::Iter<'_, syn::PathSegment>) -> syn::ExprArray {
    let mut item = syn::ExprArray {
        attrs: Vec::new(),
        bracket_token: syn::token::Bracket::default(),
        elems: Punctuated::default(),
    };

    for p in it {
        let p = syn::LitStr::new(&p.ident.to_string(), p.ident.span());

        item.elems.push(syn::Expr::Lit(syn::ExprLit {
            attrs: Vec::new(),
            lit: syn::Lit::Str(p),
        }))
    }
    item
}
