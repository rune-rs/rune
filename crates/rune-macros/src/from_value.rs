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
            type_value,
            from_value,
            vm_result,
            tuple,
            vm_try,
            ..
        } = &self.tokens;

        let (expanded, expected) = match &st.fields {
            syn::Fields::Unit => {
                let expanded = quote! {
                    #type_value::Unit => {
                        #vm_result::Ok(Self)
                    }
                    #type_value::EmptyStruct(..) => {
                        #vm_result::Ok(Self)
                    }
                };

                (expanded, &self.tokens.owned_tuple)
            }
            syn::Fields::Unnamed(f) => {
                let expanded = self.expand_unnamed(f)?;

                let expanded = quote! {
                    #type_value::Unit => {
                        let tuple = #tuple::new(&[]);
                        #vm_result::Ok(Self(#expanded))
                    }
                    #type_value::Tuple(tuple) => {
                        #vm_result::Ok(Self(#expanded))
                    }
                    #type_value::TupleStruct(tuple) => {
                        #vm_result::Ok(Self(#expanded))
                    }
                };

                (expanded, &self.tokens.owned_tuple)
            }
            syn::Fields::Named(f) => {
                let expanded = self.expand_named(f)?;

                let expanded = quote! {
                    #type_value::Object(object) => {
                        #vm_result::Ok(Self { #expanded })
                    }
                    #type_value::Struct(object) => {
                        #vm_result::Ok(Self { #expanded })
                    }
                };

                (expanded, &self.tokens.object)
            }
        };

        Ok(quote! {
            #[automatically_derived]
            impl #from_value for #ident {
                fn from_value(value: #value) -> #vm_result<Self> {
                    match #vm_try!(#value::into_type_value(value)) {
                        #expanded
                        actual => {
                            #vm_result::expected::<#expected>(#type_value::type_info(&actual))
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
            type_value,
            from_value,
            variant_data,
            value,
            vm_result,
            vm_try,
            ..
        } = &self.tokens;

        for variant in &en.variants {
            let ident = &variant.ident;
            let lit_str = syn::LitStr::new(&ident.to_string(), variant.span());

            match &variant.fields {
                syn::Fields::Unit => {
                    unit_matches.push(quote! {
                        #lit_str => #vm_result::Ok(Self::#ident)
                    });
                }
                syn::Fields::Unnamed(named) => {
                    let expanded = self.expand_unnamed(named)?;

                    unnamed_matches.push(quote! {
                        #lit_str => #vm_result::Ok(Self::#ident ( #expanded ))
                    });
                }
                syn::Fields::Named(named) => {
                    let expanded = self.expand_named(named)?;

                    named_matches.push(quote! {
                        #lit_str => #vm_result::Ok(Self::#ident { #expanded })
                    });
                }
            }
        }

        let missing = quote! {
            name => #vm_try!(#vm_result::__rune_macros__missing_variant(name))
        };

        let variant = quote! {
            #type_value::Variant(variant) => {
                let mut it = variant.rtti().item.iter();

                let name = match it.next_back_str() {
                    Some(name) => name,
                    None => return #vm_result::__rune_macros__missing_variant_name(),
                };

                match variant.data() {
                    #variant_data::Empty => match name {
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

        Ok(quote! {
            #[automatically_derived]
            impl #from_value for #ident {
                fn from_value(value: #value) -> #vm_result<Self> {
                    match #vm_try!(#value::into_type_value(value)) {
                        #variant,
                        actual => {
                            #vm_result::__rune_macros__expected_variant(#type_value::type_info(&actual))
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
            vm_try,
            try_clone,
            ..
        } = &self.tokens;

        for (index, field) in unnamed.unnamed.iter().enumerate() {
            let _ = self.cx.field_attrs(&field.attrs)?;

            from_values.push(quote! {
                match tuple.get(#index) {
                    Some(value) => {
                        let value = #vm_try!(#try_clone::try_clone(value));
                        #vm_try!(#from_value::from_value(value))
                    }
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
                vm_try,
                ..
            } = &self.tokens;

            from_values.push(quote_spanned! {
                field.span() =>
                #ident: match object.get(#name) {
                    Some(value) => #vm_try!(#from_value::from_value(value.clone())),
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
