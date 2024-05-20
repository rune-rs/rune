use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::Token;

pub(crate) struct ItemImplAttrs {
    /// Name of the function meta list function
    list: syn::Ident,
    /// Name of the function export function
    exporter: Option<syn::Ident>,
}

impl Default for ItemImplAttrs {
    fn default() -> Self {
        Self {
            list: syn::Ident::new("rune_api", proc_macro2::Span::call_site()),
            exporter: None,
        }
    }
}

impl ItemImplAttrs {
    const LIST_IDENT: &'static str = "list";
    const EXPORTER_IDENT: &'static str = "exporter";

    pub(crate) fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attrs = Self::default();

        while !input.is_empty() {
            let ident = input.parse::<syn::Ident>()?;

            match ident.to_string().as_str() {
                Self::LIST_IDENT | Self::EXPORTER_IDENT => {
                    input.parse::<Token![=]>()?;
                    if ident == Self::LIST_IDENT {
                        attrs.list = input.parse()?;
                    } else {
                        attrs.exporter = Some(input.parse()?);
                    }
                }
                _ => return Err(syn::Error::new_spanned(ident, "Unsupported option")),
            }

            if input.parse::<Option<Token![,]>>()?.is_none() {
                break;
            }
        }

        Ok(attrs)
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
                                reparsed
                                    .takes_self
                                    .then(|| syn::PathSegment::from(<syn::Token![Self]>::default()))
                                    .or_else(|| {
                                        Some(syn::PathSegment::from(
                                            syn::parse2::<syn::Ident>(
                                                block.self_ty.to_token_stream(),
                                            )
                                            .unwrap(),
                                        ))
                                    }),
                                Some(syn::PathSegment::from(name.clone())),
                            ]
                            .into_iter()
                            .flatten(),
                        ),
                    };

                    let docs = reparsed.docs;
                    let arguments = reparsed.arguments;
                    let meta_kind = syn::Ident::new(
                        ["function", "instance"][reparsed.takes_self as usize],
                        reparsed.sig.span(),
                    );
                    let build_with = if reparsed.takes_self {
                        None
                    } else {
                        Some(quote!(.build()?))
                    };

                    let meta = quote! {
                        rune::__private::FunctionMetaData {
                            kind: rune::__private::FunctionMetaKind::#meta_kind(#name_string, #path)?#build_with,
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

        let name = attrs.list;

        let export_count = export_list.len();
        let list_function = quote! {
            fn #name() -> ::rune::alloc::Result<[::rune::__private::FunctionMetaData; #export_count]> {
                Ok([ #(#export_list),* ])
            }
        };
        block.items.push(syn::parse2(list_function).unwrap());

        if let Some(exporter_name) = attrs.exporter {
            let exporter_function = quote! {
                fn #exporter_name(mut module: ::rune::Module) -> ::rune::alloc::Result<Result<::rune::Module, ::rune::ContextError>> {
                    for meta in Self::#name()? {
                        if let Err(e) = module.function_from_meta(meta) {
                            return Ok(Err(e));
                        }
                    }
                    Ok(Ok(module))
                }
            };

            block.items.push(syn::parse2(exporter_function).unwrap());
        }

        Ok(block.to_token_stream())
    }
}
