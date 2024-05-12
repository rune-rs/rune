use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;

pub(crate) struct ItemImplAttrs {
    /// Name of the exporter function
    name: syn::Ident,
}

impl ItemImplAttrs {
    pub(crate) fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input
            .parse::<syn::Ident>()
            .unwrap_or_else(|_| syn::Ident::new("export_rune_methods", input.span()));
        Ok(Self { name })
    }
}

pub(crate) struct ItemImpl(pub syn::ItemImpl);

impl ItemImpl {
    pub(crate) fn expand(self, attrs: ItemImplAttrs) -> syn::Result<TokenStream> {
        let Self(mut block) = self;

        let mut export_list = Vec::new();
        let export_attr: syn::Attribute = syn::parse_quote!(#[export]);

        for item in block.items.iter_mut() {
            if let syn::ImplItem::Fn(method) = item {
                let attr_index = method
                    .attrs
                    .iter()
                    .enumerate()
                    .find_map(|(index, attr)| (*attr == export_attr).then_some(index));

                if let Some(index) = attr_index {
                    method.attrs.remove(index);

                    let reparsed = syn::parse::Parser::parse2(
                        crate::function::Function::parse,
                        method.to_token_stream(),
                    )?;

                    let name = method.sig.ident.clone();
                    let name_string = syn::LitStr::new(
                        &reparsed.sig.ident.to_string(),
                        reparsed.sig.ident.span(),
                    );
                    let path = syn::Path {
                        leading_colon: None,
                        segments: Punctuated::from_iter(
                            [
                                syn::PathSegment::from(<syn::Token![Self]>::default()),
                                syn::PathSegment::from(name.clone()),
                            ]
                            .into_iter(),
                        ),
                    };

                    let docs = reparsed.docs;
                    let arguments = reparsed.arguments;

                    let meta = quote! {
                        rune::__private::FunctionMetaData {
                            kind: rune::__private::FunctionMetaKind::instance(#name_string, #path)?,
                            name: #name_string,
                            deprecated: None,
                            docs: &#docs[..],
                            arguments: &#arguments[..],
                        }
                    };

                    export_list.push(meta);
                }
            }
        }

        let name = attrs.name;

        let export_count = export_list.len();
        let exporter = quote! {
            fn #name() -> rune::alloc::Result<[rune::__private::FunctionMetaData; #export_count]> {
                Ok([ #(#export_list),* ])
            }
        };

        block.items.push(syn::parse2(exporter).unwrap());

        Ok(block.to_token_stream())
    }
}
