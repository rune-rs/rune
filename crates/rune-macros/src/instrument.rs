use proc_macro2::TokenStream;
use quote::quote;

/// An internal call to the macro.
pub struct Expander {
    f: syn::ItemFn,
}

impl syn::parse::Parse for Expander {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let f: syn::ItemFn = input.parse()?;

        Ok(Self { f })
    }
}

impl Expander {
    pub fn expand(self) -> Result<TokenStream, Vec<syn::Error>> {
        let f = self.f;

        let mut it = f.sig.inputs.iter();

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

        let ident = &f.sig.ident;

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

        let vis = &f.vis;
        let stmts = &f.block.stmts;
        let sig = &f.sig;

        Ok(quote! {
            #vis #sig {
                #log
                { #(#stmts)* }
            }
        })
    }
}
