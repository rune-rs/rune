use crate::context::{Context, Tokens};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned as _;

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
        let ident = &input.ident;

        let Tokens {
            value,
            from_value,
            vm_result,
            ..
        } = &self.tokens;

        let (expanded, expected) = match &st.fields {
            syn::Fields::Unit => {
                let expanded = quote_spanned! {
                    input.span() =>
                    #value::Unit => {
                        #vm_result::Ok(Self)
                    }
                    #value::UnitStruct(..) => {
                        #vm_result::Ok(Self)
                    }
                };

                (expanded, &self.tokens.unit_struct)
            }
            syn::Fields::Unnamed(f) => {
                let expanded = self.expand_unnamed(f)?;
                let borrow_ref = self.tokens.vm_try(quote!(tuple.borrow_ref()));

                let expanded = quote_spanned! {
                    f.span() =>
                    #value::Tuple(tuple) => {
                        let tuple = #borrow_ref;
                        #vm_result::Ok(Self(#expanded))
                    }
                    #value::TupleStruct(tuple) => {
                        let tuple = #borrow_ref;
                        #vm_result::Ok(Self(#expanded))
                    }
                };

                (expanded, &self.tokens.tuple)
            }
            syn::Fields::Named(f) => {
                let expanded = self.expand_named(f)?;
                let borrow_ref = self.tokens.vm_try(quote!(object.borrow_ref()));

                let expanded = quote_spanned! {
                    f.span() =>
                    #value::Object(object) => {
                        let object = #borrow_ref;
                        #vm_result::Ok(Self { #expanded })
                    }
                    #value::Struct(object) => {
                        let object = #borrow_ref;
                        #vm_result::Ok(Self { #expanded })
                    }
                };

                (expanded, &self.tokens.object)
            }
        };

        let actual_type_info = self.tokens.vm_try(quote!(actual.type_info()));

        Ok(quote_spanned! { input.span() =>
            #[automatically_derived]
            impl #from_value for #ident {
                fn from_value(value: #value) -> #vm_result<Self> {
                    match value {
                        #expanded
                        actual => {
                            #vm_result::expected::<#expected>(#actual_type_info)
                        }
                    }
                }
            }
        })
    }

    /// Expand on a struct.
    fn expand_enum(
        &mut self,
        input: &syn::DeriveInput,
        en: &syn::DataEnum,
    ) -> Result<TokenStream, ()> {
        let mut unit_matches = Vec::new();
        let mut unnamed_matches = Vec::new();
        let mut named_matches = Vec::new();

        let ident = &input.ident;

        let Tokens {
            from_value,
            variant_data,
            value,
            vm_result,
            ..
        } = &self.tokens;

        for variant in &en.variants {
            let ident = &variant.ident;
            let lit_str = syn::LitStr::new(&ident.to_string(), variant.span());

            match &variant.fields {
                syn::Fields::Unit => {
                    unit_matches.push(quote_spanned! { variant.span() =>
                        #lit_str => #vm_result::Ok(Self::#ident)
                    });
                }
                syn::Fields::Unnamed(named) => {
                    let expanded = self.expand_unnamed(named)?;

                    unnamed_matches.push(quote_spanned! { variant.span() =>
                        #lit_str => #vm_result::Ok(Self::#ident ( #expanded ))
                    });
                }
                syn::Fields::Named(named) => {
                    let expanded = self.expand_named(named)?;

                    named_matches.push(quote_spanned! { variant.span() =>
                        #lit_str => #vm_result::Ok(Self::#ident { #expanded })
                    });
                }
            }
        }

        let borrow_ref = self.tokens.vm_try(quote!(variant.borrow_ref()));

        let missing = quote_spanned! { input.span() =>
            name => #vm_result::__rune_macros__missing_variant(name)
        };

        let variant = quote_spanned! { input.span() =>
            #value::Variant(variant) => {
                let variant = #borrow_ref;
                let mut it = variant.rtti().item.iter();

                let name = match it.next_back_str() {
                    Some(name) => name,
                    None => return #vm_result::__rune_macros__missing_variant_name(),
                };

                match variant.data() {
                    #variant_data::Unit => match name {
                        #(#unit_matches,)* #missing,
                    },
                    #variant_data::Tuple(tuple) => match name {
                        #(#unnamed_matches,)* #missing,
                    },
                    #variant_data::Struct(object) => match name {
                        #(#named_matches,)* #missing,
                    },
                }
            }
        };

        let actual_type_info = self.tokens.vm_try(quote!(actual.type_info()));

        Ok(quote_spanned! { input.span() =>
            #[automatically_derived]
            impl #from_value for #ident {
                fn from_value(value: #value) -> #vm_result<Self> {
                    match value {
                        #variant,
                        actual => {
                            #vm_result::__rune_macros__expected_variant(#actual_type_info)
                        }
                    }
                }
            }
        })
    }

    /// Get a field identifier.
    fn field_ident<'a>(&self, field: &'a syn::Field) -> Result<&'a syn::Ident, ()> {
        match &field.ident {
            Some(ident) => Ok(ident),
            None => {
                self.cx.error(syn::Error::new_spanned(
                    field,
                    "unnamed fields are not supported",
                ));
                Err(())
            }
        }
    }

    /// Expand unnamed fields.
    fn expand_unnamed(&self, unnamed: &syn::FieldsUnnamed) -> Result<TokenStream, ()> {
        let mut from_values = Vec::new();

        let Tokens {
            from_value,
            vm_result,
            type_name,
            ..
        } = &self.tokens;

        for (index, field) in unnamed.unnamed.iter().enumerate() {
            let _ = self.cx.field_attrs(&field.attrs)?;
            let from_value = self
                .tokens
                .vm_try(quote!(#from_value::from_value(value.clone())));

            from_values.push(quote_spanned! {
                field.span() =>
                match tuple.get(#index) {
                    Some(value) => #from_value,
                    None => {
                        return #vm_result::__rune_macros__missing_tuple_index(#type_name::<Self>(), #index);
                    }
                }
            });
        }

        Ok(quote_spanned!(unnamed.span() => #(#from_values),*))
    }

    /// Expand named fields.
    fn expand_named(&self, named: &syn::FieldsNamed) -> Result<TokenStream, ()> {
        let mut from_values = Vec::new();

        for field in &named.named {
            let ident = self.field_ident(field)?;
            let _ = self.cx.field_attrs(&field.attrs)?;

            let name = &syn::LitStr::new(&ident.to_string(), ident.span());

            let Tokens {
                from_value,
                vm_result,
                type_name,
                ..
            } = &self.tokens;

            let from_value = self
                .tokens
                .vm_try(quote!(#from_value::from_value(value.clone())));

            from_values.push(quote_spanned! {
                field.span() =>
                #ident: match object.get(#name) {
                    Some(value) => #from_value,
                    None => {
                        return #vm_result::__rune_macros__missing_struct_field(#type_name::<Self>(), #name);
                    }
                }
            });
        }

        Ok(quote_spanned!(named.span() => #(#from_values),* ))
    }
}

pub(super) fn expand(input: &syn::DeriveInput) -> Result<TokenStream, Vec<syn::Error>> {
    let cx = Context::new();

    let Ok(attr) = cx.type_attrs(&input.attrs) else {
        return Err(cx.errors.into_inner());
    };

    let tokens = cx.tokens_with_module(attr.module.as_ref());

    let mut expander = Expander { cx, tokens };

    match &input.data {
        syn::Data::Struct(st) => {
            if let Ok(expanded) = expander.expand_struct(input, st) {
                return Ok(expanded);
            }
        }
        syn::Data::Enum(en) => {
            if let Ok(expanded) = expander.expand_enum(input, en) {
                return Ok(expanded);
            }
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
