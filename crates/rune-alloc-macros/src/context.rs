use std::cell::RefCell;

use proc_macro2::Span;
use syn::spanned::Spanned as _;

#[derive(Default)]
pub(crate) struct Context {
    pub(crate) errors: RefCell<Vec<syn::Error>>,
    pub(crate) module: Option<syn::Path>,
}

impl Context {
    /// Construct a new context.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Register an error.
    pub(crate) fn error(&self, error: syn::Error) {
        self.errors.borrow_mut().push(error)
    }

    /// Test if context has any errors.
    pub(crate) fn has_errors(&self) -> bool {
        !self.errors.borrow().is_empty()
    }

    /// Convert into errors.
    pub(crate) fn into_errors(self) -> Vec<syn::Error> {
        self.errors.into_inner()
    }

    pub(crate) fn tokens_with_module(&self, module: Option<&syn::Path>) -> Tokens {
        let default_module;

        let m = match module {
            Some(module) => module,
            None => match &self.module {
                Some(module) => module,
                None => {
                    default_module = syn::Path::from(syn::Ident::new("rune", Span::call_site()));
                    &default_module
                }
            },
        };

        Tokens {
            try_clone: path(m, ["alloc", "clone", "TryClone"]),
            alloc: path(m, ["alloc"]),
        }
    }
}

fn path<const N: usize>(base: &syn::Path, path: [&'static str; N]) -> syn::Path {
    let mut base = base.clone();

    for s in path {
        let ident = syn::Ident::new(s, base.span());
        base.segments.push(syn::PathSegment::from(ident));
    }

    base
}

pub(crate) struct Tokens {
    pub(crate) try_clone: syn::Path,
    pub(crate) alloc: syn::Path,
}
