use proc_macro2::TokenStream;
use quote::quote;
use syn::Token;

/// An internal call to the macro.
#[derive(Default)]
pub struct Attr {
    skip_span: bool,
    leaving: bool,
    span: Option<syn::Ident>,
}

impl syn::parse::Parse for Attr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut attr = Attr::default();
        let mut last = false;

        while !input.is_empty() {
            let ident = input.parse::<syn::Ident>()?;

            if ident == "skip_span" {
                attr.skip_span = true;
            } else if ident == "leaving" {
                attr.leaving = true;
            } else if ident == "span" {
                input.parse::<Token![=]>()?;
                attr.span = Some(input.parse()?);
            } else {
                return Err(syn::Error::new_spanned(ident, "Unsupported attribute"));
            }

            if last {
                break;
            }

            last = input.parse::<Option<Token![,]>>()?.is_none();
        }

        Ok(attr)
    }
}

/// An internal call to the macro.
pub struct Expander {
    attrs: Vec<syn::Attribute>,
    vis: syn::Visibility,
    sig: syn::Signature,
    remaining: TokenStream,
}

impl syn::parse::Parse for Expander {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            attrs: input.call(syn::Attribute::parse_outer)?,
            vis: input.parse()?,
            sig: input.parse()?,
            remaining: input.parse()?,
        })
    }
}

impl Expander {
    pub fn expand(self, attr: &Attr) -> Result<TokenStream, Vec<syn::Error>> {
        let mut it = self.sig.inputs.iter();

        let first = match it.next() {
            Some(syn::FnArg::Typed(ty)) => match &*ty.pat {
                syn::Pat::Ident(ident) => Some(&ident.ident),
                _ => None,
            },
            _ => None,
        };

        let second = match it.next() {
            Some(syn::FnArg::Typed(ty)) => match &*ty.pat {
                syn::Pat::Ident(ident) => Some(&ident.ident),
                _ => None,
            },
            _ => None,
        };

        let ident = &self.sig.ident;

        let log = match (first, second) {
            (Some(a), Some(b)) => {
                let ident = syn::LitStr::new(&ident.to_string(), ident.span());

                let enter = match (attr.skip_span, &attr.span) {
                    (false, None) => Some(quote! {
                        let _instrument_span = ::tracing::span!(::tracing::Level::TRACE, #ident);
                        let _instrument_enter = _instrument_span.enter();

                        if let Some(source) = #b.q.sources.source(#b.source_id, #a.span()) {
                            ::tracing::trace!("{:?}", source);
                        }
                    }),
                    (_, Some(span)) => Some(quote! {
                        let _instrument_span = ::tracing::span!(::tracing::Level::TRACE, #ident);
                        let _instrument_enter = _instrument_span.enter();

                        if let Some(source) = #a.q.sources.source(#a.source_id, Spanned::span(#span)) {
                            ::tracing::trace!("{:?}", source);
                        }
                    }),
                    _ => Some(quote! {
                        let _instrument_span = ::tracing::span!(::tracing::Level::TRACE, #ident);
                        let _instrument_enter = _instrument_span.enter();
                        ::tracing::trace!("entering");
                    }),
                };

                enter
            }
            _ => None,
        };

        let attrs = &self.attrs;
        let vis = &self.vis;
        let sig = &self.sig;
        let remaining = &self.remaining;

        let leave = if attr.leaving {
            Some(quote!(::tracing::trace!("leaving");))
        } else {
            None
        };

        Ok(quote! {
            #(#attrs)*
            #vis #sig {
                #log
                let __result = (||{ #remaining })();
                #leave
                __result
            }
        })
    }
}
