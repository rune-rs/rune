use crate::context::Context;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned as _;

struct Expander {
    ctx: Context,
}

impl Expander {
    /// Expand on a struct.
    fn expand_struct(
        &mut self,
        input: &syn::DeriveInput,
        st: &syn::DataStruct,
    ) -> Option<TokenStream> {
        let inner = self.expand_fields(&st.fields)?;

        let ident = &input.ident;
        let value = &self.ctx.value;
        let vm_error = &self.ctx.vm_error;
        let from_value = &self.ctx.from_value;

        Some(quote! {
            impl #from_value for #ident {
                fn from_value(value: #value) -> Result<Self, #vm_error> {
                    #inner
                }
            }
        })
    }

    fn expand_union(
        &mut self,
        input: &syn::DeriveInput,
        un: &syn::DataUnion,
    ) -> Option<TokenStream> {
        let inner = self.expand_named(&un.fields)?;

        let ident = &input.ident;
        let value = &self.ctx.value;
        let vm_error = &self.ctx.vm_error;
        let from_value = &self.ctx.from_value;

        Some(quote! {
            impl #from_value for #ident {
                fn from_value(value: #value) -> Result<Self, #vm_error> {
                    #inner
                }
            }
        })
    }

    fn expand_enum(
        &mut self,
        input: &syn::DeriveInput,
        en: &syn::DataEnum,
    ) -> Option<TokenStream> {

        let inner = self.expand_variants(&en.variants)?;

        let ident = &input.ident;
        let value = &self.ctx.value;
        let vm_error = &self.ctx.vm_error;
        let from_value = &self.ctx.from_value;

        Some(quote! {
            impl #from_value for #ident {
                fn from_value(value: #value) -> Result<Self, #vm_error> {
                    #inner
                }
            }
        })
    }

    fn expand_variants(&mut self, named: &syn::punctuated::Punctuated<syn::Variant, syn::Token![,]>) -> Option<TokenStream> {
        let mut from_values = Vec::new();

        for field in named {
            let ident = &field.ident;
            let _ = self.ctx.parse_field_attrs(&field.attrs)?;

            let name = &syn::LitStr::new(&ident.to_string(), ident.span());

            let from_value = &self.ctx.from_value;
            let vm_error = &self.ctx.vm_error;
            let vm_error_kind = &self.ctx.vm_error_kind;

            let from_value = quote_spanned! {
                field.span() => #from_value::from_value(value.clone())?
            };

            from_values.push(quote_spanned! {
                field.span() =>
                #ident: match object.get(#name) {
                    Some(value) => #from_value,
                    None => {
                        return Err(#vm_error::from(#vm_error_kind::MissingDynamicStructField {
                            target: std::any::type_name::<Self>(),
                            name: #name,
                        }));
                    }
                }
            });
        }

        let object = &self.ctx.object;
        let value = &self.ctx.value;
        let vm_error = &self.ctx.vm_error;

        Some(quote_spanned! {
            named.span() =>
            match value {
                #value::Object(object) => {
                    let object = object.borrow_ref()?;

                    Ok(Self {
                        #(#from_values),*
                    })
                }
                #value::TypedObject(object) => {
                    let object = object.borrow_ref()?;

                    Ok(Self {
                        #(#from_values),*
                    })
                }
                actual => {
                    Err(#vm_error::expected::<#object>(actual.type_info()?))
                }
            }
        })
    }

    /// Expand field decoding.
    fn expand_fields(&mut self, fields: &syn::Fields) -> Option<TokenStream> {
        match fields {
            syn::Fields::Unnamed(named) => self.expand_unnamed(named),
            syn::Fields::Named(named) => self.expand_named(named),
            syn::Fields::Unit => {
                self.ctx.errors.push(syn::Error::new_spanned(
                    fields,
                    "unit structs are not supported",
                ));
                None
            }
        }
    }

    /// Get a field identifier.
    fn field_ident<'a>(&mut self, field: &'a syn::Field) -> Option<&'a syn::Ident> {
        match &field.ident {
            Some(ident) => Some(ident),
            None => {
                self.ctx.errors.push(syn::Error::new_spanned(
                    field,
                    "unnamed fields are not supported",
                ));
                None
            }
        }
    }

    /// Expand unnamed fields.
    fn expand_unnamed(&mut self, unnamed: &syn::FieldsUnnamed) -> Option<TokenStream> {
        let mut from_values = Vec::new();

        for (index, field) in unnamed.unnamed.iter().enumerate() {
            let _ = self.ctx.parse_field_attrs(&field.attrs)?;

            let from_value = &self.ctx.from_value;
            let vm_error = &self.ctx.vm_error;
            let vm_error_kind = &self.ctx.vm_error_kind;

            let from_value = quote_spanned! {
                field.span() => #from_value::from_value(value.clone())?
            };

            from_values.push(quote_spanned! {
                field.span() =>
                match tuple.get(#index) {
                    Some(value) => #from_value,
                    None => {
                        return Err(#vm_error::from(#vm_error_kind::MissingDynamicStructTupleIndex {
                            target: std::any::type_name::<Self>(),
                            index: #index,
                        }));
                    }
                }
            });
        }

        let tuple = &self.ctx.tuple;
        let value = &self.ctx.value;
        let vm_error = &self.ctx.vm_error;

        Some(quote_spanned! {
            unnamed.span() =>
            match value {
                #value::Tuple(tuple) => {
                    let tuple = tuple.borrow_ref()?;
                    Ok(Self(#(#from_values),*))
                }
                #value::TypedTuple(tuple) => {
                    let tuple = tuple.borrow_ref()?;
                    Ok(Self(#(#from_values),*))
                }
                actual => {
                    Err(#vm_error::expected::<#tuple>(actual.type_info()?))
                }
            }
        })
    }

    /// Expand named fields.
    fn expand_named(&mut self, named: &syn::FieldsNamed) -> Option<TokenStream> {
        let mut from_values = Vec::new();

        for field in &named.named {
            let ident = self.field_ident(&field)?;
            let _ = self.ctx.parse_field_attrs(&field.attrs)?;

            let name = &syn::LitStr::new(&ident.to_string(), ident.span());

            let from_value = &self.ctx.from_value;
            let vm_error = &self.ctx.vm_error;
            let vm_error_kind = &self.ctx.vm_error_kind;

            let from_value = quote_spanned! {
                field.span() => #from_value::from_value(value.clone())?
            };

            from_values.push(quote_spanned! {
                field.span() =>
                #ident: match object.get(#name) {
                    Some(value) => #from_value,
                    None => {
                        return Err(#vm_error::from(#vm_error_kind::MissingDynamicStructField {
                            target: std::any::type_name::<Self>(),
                            name: #name,
                        }));
                    }
                }
            });
        }

        let object = &self.ctx.object;
        let value = &self.ctx.value;
        let vm_error = &self.ctx.vm_error;

        Some(quote_spanned! {
            named.span() =>
            match value {
                #value::Object(object) => {
                    let object = object.borrow_ref()?;

                    Ok(Self {
                        #(#from_values),*
                    })
                }
                #value::TypedObject(object) => {
                    let object = object.borrow_ref()?;

                    Ok(Self {
                        #(#from_values),*
                    })
                }
                actual => {
                    Err(#vm_error::expected::<#object>(actual.type_info()?))
                }
            }
        })
    }
}

pub(super) fn expand(input: &syn::DeriveInput) -> Result<TokenStream, Vec<syn::Error>> {
    let mut expander = Expander {
        ctx: Context::new(),
    };

    match &input.data {
        syn::Data::Struct(st) => {
            if let Some(expanded) = expander.expand_struct(input, st) {
                return Ok(expanded);
            }
        }
        syn::Data::Enum(en) => {
            if let Some(expanded) = expander.expand_enum(input, en) {
                return Ok(expanded);
            }
        }
        syn::Data::Union(un) => {
            if let Some(expanded) = expander.expand_union(input, un) {
                return Ok(expanded);
            }
        }
    }

    Err(expander.ctx.errors)
}
