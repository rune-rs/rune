use proc_macro2::{Span, TokenStream};
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
    vis: syn::Visibility,
    signature: syn::Signature,
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

        Ok(Self {
            attributes,
            docs,
            vis: input.parse()?,
            signature: input.parse()?,
            remainder: input.parse()?,
        })
    }

    /// Expand the function declaration.
    pub(crate) fn expand(self, attrs: ModuleAttrs) -> syn::Result<TokenStream> {
        let docs = self.docs;

        let item_buf = crate::item::build_buf(&attrs.path)?;
        let item_bytes = crate::item::buf_as_bytes(&item_buf);

        let mut stream = TokenStream::new();

        let name = quote::format_ident!("{}__meta", self.signature.ident);
        let doc = syn::LitStr::new(
            &format!(" Module metadata for `{item_buf}`."),
            Span::call_site(),
        );

        stream.extend(quote! {
            #[doc = #doc]
            #[automatically_derived]
            #[allow(non_snake_case)]
            #[doc(hidden)]
            fn #name() -> Result<rune::__priv::ModuleMetaData, rune::alloc::Error> {
                Ok(rune::__priv::ModuleMetaData {
                    item: unsafe { rune::__priv::Item::from_bytes(&#item_bytes) },
                    docs: &#docs[..],
                })
            }
        });

        stream.extend(quote!(#[allow(rustdoc::broken_intra_doc_links)]));

        for attribute in self.attributes {
            attribute.to_tokens(&mut stream);
        }

        self.vis.to_tokens(&mut stream);
        self.signature.to_tokens(&mut stream);
        stream.extend(self.remainder);
        Ok(stream)
    }
}
