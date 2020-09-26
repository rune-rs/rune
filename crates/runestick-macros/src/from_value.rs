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
        let (expanded, expected) = match &st.fields {
            syn::Fields::Unit => {
                let value = &self.ctx.value;

                let expanded = quote_spanned! {
                    input.span() =>
                    #value::Unit => {
                        Ok(Self)
                    }
                    #value::UnitStruct(..) => {
                        Ok(Self)
                    }
                };

                (expanded, &self.ctx.unit_struct)
            }
            syn::Fields::Unnamed(unnamed) => {
                let expanded = &self.expand_unnamed(unnamed)?;
                let value = &self.ctx.value;

                let expanded = quote_spanned! {
                    unnamed.span() =>
                    #value::Tuple(tuple) => {
                        let tuple = tuple.borrow_ref()?;
                        Ok(Self(#expanded))
                    }
                    #value::TupleStruct(tuple) => {
                        let tuple = tuple.borrow_ref()?;
                        Ok(Self(#expanded))
                    }
                };

                (expanded, &self.ctx.tuple)
            }
            syn::Fields::Named(named) => {
                let expanded = &self.expand_named(named)?;
                let value = &self.ctx.value;

                let expanded = quote_spanned! {
                    named.span() =>
                    #value::Object(object) => {
                        let object = object.borrow_ref()?;
                        Ok(Self { #expanded })
                    }
                    #value::Struct(object) => {
                        let object = object.borrow_ref()?;
                        Ok(Self { #expanded })
                    }
                };

                (expanded, &self.ctx.object)
            }
        };

        let ident = &input.ident;
        let value = &self.ctx.value;
        let vm_error = &self.ctx.vm_error;
        let from_value = &self.ctx.from_value;

        Some(quote! {
            impl #from_value for #ident {
                fn from_value(value: #value) -> Result<Self, #vm_error> {
                    match value {
                        #expanded
                        actual => {
                            Err(#vm_error::expected::<#expected>(actual.type_info()?))
                        }
                    }
                }
            }
        })
    }

    /// Expand on a struct.
    fn expand_enum(&mut self, input: &syn::DeriveInput, en: &syn::DataEnum) -> Option<TokenStream> {
        let mut unit_matches = Vec::new();
        let mut unnamed_matches = Vec::new();
        let mut named_matches = Vec::new();

        for variant in &en.variants {
            let ident = &variant.ident;
            let lit_str = syn::LitStr::new(&ident.to_string(), variant.span());

            match &variant.fields {
                syn::Fields::Unit => {
                    unit_matches.push(quote_spanned! { variant.span() =>
                        #lit_str => Ok(Self::#ident)
                    });
                }
                syn::Fields::Unnamed(named) => {
                    let expanded = self.expand_unnamed(named)?;

                    unnamed_matches.push(quote_spanned! { variant.span() =>
                        #lit_str => {
                            let tuple = value.data();
                            Ok( Self::#ident ( #expanded ) )
                        }
                    });
                }
                syn::Fields::Named(named) => {
                    let expanded = self.expand_named(named)?;

                    named_matches.push(quote_spanned! { variant.span() =>
                        #lit_str => {
                            let object = value.data();
                            Ok( Self::#ident { #expanded } )
                        }
                    });
                }
            }
        }

        let from_value = &self.ctx.from_value;
        let ident = &input.ident;
        let value = &self.ctx.value;
        let vm_error = &self.ctx.vm_error;
        let vm_error_kind = &self.ctx.vm_error_kind;

        let name = &quote_spanned! {
            input.span() =>
            let value = value.borrow_ref()?;
            let mut it = value.rtti().item.iter();

            let name = match it.next_back_str() {
                Some(name) => name,
                None => return Err(#vm_error::from(#vm_error_kind::MissingVariantName)),
            };
        };

        let mut matches = Vec::new();

        matches.push(quote_spanned! { input.span() =>
            #value::UnitVariant(value) => {
                #name

                match name {
                    #(#unit_matches,)*
                    name => {
                        return Err(#vm_error::from(#vm_error_kind::MissingVariant { name: name.into() }))
                    }
                }
            }
        });

        matches.push(quote_spanned! { input.span() =>
            #value::TupleVariant(value) => {
                #name

                match name {
                    #(#unnamed_matches)*
                    name => {
                        return Err(#vm_error::from(#vm_error_kind::MissingVariant { name: name.into() }))
                    }
                }
            }
        });

        matches.push(quote_spanned! { input.span() =>
            #value::StructVariant(value) => {
                #name

                match name {
                    #(#named_matches)*
                    name => {
                        return Err(#vm_error::from(#vm_error_kind::MissingVariant { name: name.into() }))
                    }
                }
            }
        });

        Some(quote_spanned! { input.span() =>
            impl #from_value for #ident {
                fn from_value(value: #value) -> Result<Self, #vm_error> {
                    match value {
                        #(#matches,)*
                        actual => {
                            Err(#vm_error::from(#vm_error_kind::ExpectedVariant {
                                actual: actual.type_info()?,
                            }))
                        }
                    }
                }
            }
        })
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
            let _ = self.ctx.parse_rune_attrs(&field.attrs)?;

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
                        return Err(#vm_error::from(#vm_error_kind::MissingTupleIndex {
                            target: std::any::type_name::<Self>(),
                            index: #index,
                        }));
                    }
                }
            });
        }

        Some(quote_spanned!(unnamed.span() => #(#from_values),*))
    }

    /// Expand named fields.
    fn expand_named(&mut self, named: &syn::FieldsNamed) -> Option<TokenStream> {
        let mut from_values = Vec::new();

        for field in &named.named {
            let ident = self.field_ident(&field)?;
            let _ = self.ctx.parse_rune_attrs(&field.attrs)?;

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
                        return Err(#vm_error::from(#vm_error_kind::MissingStructField {
                            target: std::any::type_name::<Self>(),
                            name: #name,
                        }));
                    }
                }
            });
        }

        Some(quote_spanned!(named.span() => #(#from_values),* ))
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
            expander.ctx.errors.push(syn::Error::new_spanned(
                un.union_token,
                "not supported on unions",
            ));
        }
    }

    Err(expander.ctx.errors)
}
