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
        let ctx = Context::with_crate();
        let tokens = ctx.tokens_with_module(None);

        let mut expander = Expander { ctx, tokens };

        match &self.input.data {
            syn::Data::Struct(st) => {
                if let Some(stream) = expander.expand_struct(&self.input, st) {
                    return Ok(stream);
                }
            }
            syn::Data::Enum(en) => {
                expander.ctx.errors.push(syn::Error::new_spanned(
                    en.enum_token,
                    "not supported on enums",
                ));
            }
            syn::Data::Union(un) => {
                expander.ctx.errors.push(syn::Error::new_spanned(
                    un.union_token,
                    "not supported on unions",
                ));
            }
        }

        Err(expander.ctx.errors)
    }
}

struct Expander {
    ctx: Context,
    tokens: Tokens,
}

impl Expander {
    /// Expand on a struct.
    fn expand_struct(
        &mut self,
        input: &syn::DeriveInput,
        st: &syn::DataStruct,
    ) -> Option<TokenStream> {
        let accessor = self.pick_field(&st.fields)?;

        let ident = &input.ident;
        let opaque = &self.tokens.opaque;
        let id = &self.tokens.id;

        let (gen_impl, gen_type, gen_where) = input.generics.split_for_impl();

        Some(quote! {
            impl #gen_impl #opaque for #ident #gen_type #gen_where {
                fn id(&self) -> #id {
                    #accessor
                }
            }
        })
    }

    /// Expand field decoding.
    fn pick_field(&mut self, fields: &syn::Fields) -> Option<TokenStream> {
        let mut field = None;

        for (n, f) in fields.iter().enumerate() {
            let attrs = self.ctx.field_attrs(&f.attrs)?;

            if attrs.id.is_some() {
                if field.is_some() {
                    self.ctx.errors.push(syn::Error::new_spanned(
                        f,
                        "only one field can be marked `#[rune(id)]`",
                    ));
                }

                field = Some((n, f));
            }
        }

        let (n, f) = match field {
            Some(field) => field,
            None => {
                self.ctx.errors.push(syn::Error::new_spanned(
                    fields,
                    "could not find a suitable identifier field",
                ));
                return None;
            }
        };

        Some(match &f.ident {
            Some(ident) => quote!(self.#ident),
            None => quote!(self.#n),
        })
    }
}
