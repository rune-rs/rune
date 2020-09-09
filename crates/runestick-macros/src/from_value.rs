use crate::context::Context;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned as _;

impl Context {
    /// Expand on a struct.
    fn expand_struct(
        &mut self,
        input: &syn::DeriveInput,
        st: &syn::DataStruct,
    ) -> Option<TokenStream> {
        let inner = self.expand_fields(&st.fields)?;

        let ident = &input.ident;
        let value = &self.value;
        let vm_error = &self.vm_error;
        let from_value = &self.from_value;

        Some(quote! {
            impl #from_value for #ident {
                fn from_value(value: #value) -> Result<Self, #vm_error> {
                    #inner
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
                self.errors.push(syn::Error::new_spanned(
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
                self.errors.push(syn::Error::new_spanned(
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
            let attrs = self.parse_field_attrs(&field.attrs)?;

            let from_value = &self.from_value;
            let from_any = &self.from_any;
            let vm_error = &self.vm_error;
            let vm_error_kind = &self.vm_error_kind;

            if attrs.any {
                let from_any = quote_spanned! {
                    field.span() => #from_any::from_any(any.clone())?
                };

                from_values.push(quote_spanned! {
                    field.span() =>
                    match tuple.get(#index) {
                        Some(Value::Any(any)) => #from_any,
                        Some(actual) => return Err(#vm_error::expected_any(actual.type_info()?)),
                        None => {
                            return Err(#vm_error::from(#vm_error_kind::MissingDynamicStructTupleIndex {
                                target: std::any::type_name::<Self>(),
                                index: #index,
                            }));
                        }
                    }
                });
            } else {
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
        }

        let tuple = &self.tuple;
        let value = &self.value;
        let vm_error = &self.vm_error;

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
            let attrs = self.parse_field_attrs(&field.attrs)?;

            let name = &syn::LitStr::new(&ident.to_string(), ident.span());

            let from_value = &self.from_value;
            let from_any = &self.from_any;
            let vm_error = &self.vm_error;
            let vm_error_kind = &self.vm_error_kind;

            if attrs.any {
                let from_any = quote_spanned! {
                    field.span() => #from_any::from_any(any.clone())?
                };

                from_values.push(quote_spanned! {
                    field.span() =>
                    #ident: match object.get(#name) {
                        Some(Value::Any(any)) => #from_any,
                        Some(actual) => return Err(#vm_error::expected_any(actual.type_info()?)),
                        None => {
                            return Err(#vm_error::from(#vm_error_kind::MissingDynamicStructField {
                                target: std::any::type_name::<Self>(),
                                name: #name,
                            }));
                        }
                    }
                });
            } else {
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
        }

        let object = &self.object;
        let value = &self.value;
        let vm_error = &self.vm_error;

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
    let mut ctx = Context::new();

    match &input.data {
        syn::Data::Struct(st) => {
            if let Some(expanded) = ctx.expand_struct(input, st) {
                return Ok(expanded);
            }
        }
        syn::Data::Enum(en) => {
            ctx.errors.push(syn::Error::new_spanned(
                en.enum_token,
                "not supported on enums",
            ));
        }
        syn::Data::Union(un) => {
            ctx.errors.push(syn::Error::new_spanned(
                un.union_token,
                "not supported on unions",
            ));
        }
    }

    Err(ctx.errors)
}
