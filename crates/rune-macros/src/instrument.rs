use proc_macro2::TokenStream;
use quote::quote;

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
    pub fn expand(self) -> Result<TokenStream, Vec<syn::Error>> {
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

                Some(quote! {
                    let _instrument_span = ::tracing::span!(::tracing::Level::TRACE, #ident);
                    let _instrument_enter = _instrument_span.enter();

                    if let Some(source) = #b.q.sources.source(#b.source_id, #a.span()) {
                        ::tracing::trace!("{:?}", source);
                    }
                })
            }
            _ => None,
        };

        let attrs = &self.attrs;
        let vis = &self.vis;
        let sig = &self.sig;
        let remaining = &self.remaining;

        Ok(quote! {
            #(#attrs)*
            #vis #sig {
                #log
                #remaining
            }
        })
    }
}
