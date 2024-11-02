use syn::parse::{Parse, ParseStream};
use syn::Token;

pub(super) struct PathIn<T = syn::Path> {
    pub(super) in_crate: syn::Path,
    #[allow(unused)]
    pub(super) comma_token: Token![,],
    pub(super) item: T,
}

impl<T> Parse for PathIn<T>
where
    T: Parse,
{
    #[inline]
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            in_crate: input.parse()?,
            comma_token: input.parse()?,
            item: input.parse()?,
        })
    }
}
