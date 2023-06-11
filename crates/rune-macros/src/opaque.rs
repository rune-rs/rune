use crate::context::{Context, Tokens};
use proc_macro2::TokenStream;
use quote::quote;

/// Derive implementation of `Opaque`.
pub struct Derive {
    input: syn::DeriveInput,
}

impl syn::parse::Parse for Derive {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            input: input.parse()?,
        })
    }
}

impl Derive {
    pub(super) fn expand(self) -> Result<TokenStream, Vec<syn::Error>> {
        let cx = Context::new();
        let tokens = cx.tokens_with_module(None);

        let mut expander = Expander { cx, tokens };

        match &self.input.data {
            syn::Data::Struct(st) => {
                if let Ok(stream) = expander.expand_struct(&self.input, st) {
                    return Ok(stream);
                }
            }
            syn::Data::Enum(en) => {
                expander.cx.error(syn::Error::new_spanned(
                    en.enum_token,
                    "not supported on enums",
                ));
            }
            syn::Data::Union(un) => {
                expander.cx.error(syn::Error::new_spanned(
                    un.union_token,
                    "not supported on unions",
                ));
            }
        }

        Err(expander.cx.errors.into_inner())
    }
}

struct Expander {
    cx: Context,
    tokens: Tokens,
}

impl Expander {
    /// Expand on a struct.
    fn expand_struct(
        &mut self,
        input: &syn::DeriveInput,
        st: &syn::DataStruct,
    ) -> Result<TokenStream, ()> {
        let accessor = self.pick_field(&st.fields)?;

        let ident = &input.ident;
        let opaque = &self.tokens.opaque;
        let id = &self.tokens.id;

        let (gen_impl, gen_type, gen_where) = input.generics.split_for_impl();

        Ok(quote! {
            #[automatically_derived]
            impl #gen_impl #opaque for #ident #gen_type #gen_where {
                fn id(&self) -> #id {
                    #accessor
                }
            }
        })
    }

    /// Expand field decoding.
    fn pick_field(&mut self, fields: &syn::Fields) -> Result<TokenStream, ()> {
        let mut field = None;

        for (n, f) in fields.iter().enumerate() {
            let attrs = self.cx.field_attrs(&f.attrs)?;

            if attrs.id.is_some() {
                if field.is_some() {
                    self.cx.error(syn::Error::new_spanned(
                        f,
                        "only one field can be marked `#[rune(id)]`",
                    ));
                }

                field = Some((n, f));
            }
        }

        let Some((n, f)) = field else {
            self.cx.error(syn::Error::new_spanned(
                fields,
                "Could not find a suitable identifier field",
            ));
            return Err(());
        };

        Ok(match &f.ident {
            Some(ident) => quote!(self.#ident),
            None => quote!(self.#n),
        })
    }
}
