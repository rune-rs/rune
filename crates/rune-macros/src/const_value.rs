use core::fmt;

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::DeriveInput;

use crate::context::{Context, Tokens};

/// An internal call to the macro.
pub(super) struct Derive {
    input: DeriveInput,
}

impl Parse for Derive {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            input: input.parse()?,
        })
    }
}

pub(super) struct ConstBuilder<T> {
    ident: T,
    tokens: Tokens,
    body: TokenStream,
    variables: Vec<syn::Ident>,
    members: Vec<syn::Member>,
    from_const_fields: Vec<TokenStream>,
    from_value_fields: Vec<TokenStream>,
}

impl Derive {
    pub(super) fn into_builder(self, cx: &Context) -> Result<ConstBuilder<syn::Ident>, ()> {
        let attr = cx.const_value_type_attrs(&self.input.attrs)?;
        let tokens = cx.tokens_with_module(attr.module.as_ref());
        let body;

        let Tokens {
            const_value,
            from_const_value_t,
            to_const_value_t,
            type_hash_t,
            from_value,
            value,
            ..
        } = &tokens;

        let mut variables = Vec::new();
        let mut members = Vec::new();
        let mut from_const_fields = Vec::new();
        let mut from_value_fields = Vec::new();

        match self.input.data {
            syn::Data::Struct(data) => {
                let mut fields = Vec::new();

                for (index, field) in data.fields.iter().enumerate() {
                    let attr = cx.const_value_field_attrs(&field.attrs)?;

                    let member = match &field.ident {
                        Some(ident) => syn::Member::Named(ident.clone()),
                        None => syn::Member::Unnamed(syn::Index::from(index)),
                    };

                    let ty = &field.ty;

                    let var = syn::Ident::new(&format!("v{index}"), Span::call_site());

                    if let Some(path) = &attr.with {
                        let to_const_value: syn::Path =
                            syn::parse_quote_spanned!(path.span() => #path::to_const_value);
                        let from_const_value: syn::Path =
                            syn::parse_quote_spanned!(path.span() => #path::from_const_value);
                        let from_value: syn::Path =
                            syn::parse_quote_spanned!(path.span() => #path::from_value);

                        fields.push(quote!(#to_const_value(self.#member)?));
                        from_const_fields.push(quote!(#from_const_value(#var)?));
                        from_value_fields.push(quote!(#from_value(#value::take(#var))?));
                    } else {
                        fields.push(quote! {
                            <#ty as #to_const_value_t>::to_const_value(self.#member)?
                        });

                        from_const_fields.push(quote! {
                            <#ty as #from_const_value_t>::from_const_value(#var)?
                        });

                        from_value_fields.push(quote! {
                            <#ty as #from_value>::from_value(#value::take(#var)).into_result()?
                        });
                    }

                    variables.push(var);
                    members.push(member);
                }

                body = quote! {
                    #const_value::for_struct(<Self as #type_hash_t>::HASH, [#(#fields),*])?
                };
            }
            syn::Data::Enum(..) => {
                cx.error(syn::Error::new(
                    Span::call_site(),
                    "ToConstValue: enums are not supported",
                ));
                return Err(());
            }
            syn::Data::Union(..) => {
                cx.error(syn::Error::new(
                    Span::call_site(),
                    "ToConstValue: unions are not supported",
                ));
                return Err(());
            }
        }

        Ok(ConstBuilder {
            ident: self.input.ident,
            tokens,
            body,
            variables,
            members,
            from_const_fields,
            from_value_fields,
        })
    }
}

impl<T> ConstBuilder<T>
where
    T: ToTokens + fmt::Display,
{
    pub(super) fn expand(self) -> TokenStream {
        let Tokens {
            arc,
            const_construct_t,
            const_value,
            option,
            result,
            runtime_error,
            to_const_value_t,
            value,
            ..
        } = &self.tokens;

        let ident = self.ident;
        let construct = syn::Ident::new(&format!("{ident}Construct"), Span::call_site());
        let body = self.body;
        let members = &self.members;
        let variables = &self.variables;
        let from_const_fields = &self.from_const_fields;
        let from_value_fields = &self.from_value_fields;

        let expected = self.members.len();

        quote! {
            #[automatically_derived]
            impl #to_const_value_t for #ident {
                #[inline]
                fn to_const_value(self) -> #result<#const_value, #runtime_error> {
                    #result::Ok(#body)
                }

                #[inline]
                fn construct() -> #option<#arc<dyn #const_construct_t>> {
                    struct #construct;

                    impl #const_construct_t for #construct {
                        #[inline]
                        fn const_construct(&self, values: &[#const_value]) -> #result<#value, #runtime_error> {
                            let [#(#variables),*] = values else {
                                return #result::Err(#runtime_error::bad_argument_count(values.len(), #expected));
                            };

                            let value = #ident {
                                #(#members: #from_const_fields,)*
                            };

                            #result::Ok(Value::new(value)?)
                        }

                        #[inline]
                        fn runtime_construct(&self, values: &mut [#value]) -> #result<#value, #runtime_error> {
                            let [#(#variables),*] = values else {
                                return #result::Err(#runtime_error::bad_argument_count(values.len(), #expected));
                            };

                            let value = #ident {
                                #(#members: #from_value_fields,)*
                            };

                            #result::Ok(Value::new(value)?)
                        }
                    }

                    #option::Some(#arc::new(#construct))
                }
            }
        }
    }
}
