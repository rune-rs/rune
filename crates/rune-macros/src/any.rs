use std::collections::BTreeMap;
use std::mem::take;

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use rune_core::{ComponentRef, Hash, ItemBuf};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::Token;

use crate::context::{Context, Generate, GenerateTarget, Tokens, TypeAttr};

/// An internal call to the macro.
pub struct InternalCall {
    item: syn::Path,
    path: syn::Path,
}

impl syn::parse::Parse for InternalCall {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let item = input.parse()?;
        input.parse::<Token![,]>()?;
        let path = input.parse()?;
        Ok(Self { item, path })
    }
}

impl InternalCall {
    pub fn expand(self) -> Result<TokenStream, Vec<syn::Error>> {
        let cx = Context::with_crate();
        let tokens = cx.tokens_with_module(None);

        let name = match self.path.segments.last() {
            Some(last) if last.arguments.is_empty() => last.ident.clone(),
            _ => {
                return Err(vec![syn::Error::new(
                    self.path.span(),
                    "expected last component in path to be without parameters,
                    give it an explicit name instead with `, \"Type\"`",
                )])
            }
        };

        let generics = syn::Generics::default();

        let mut item = self.item.clone();
        item.segments.push(syn::PathSegment::from(name.clone()));
        let type_hash = build_type_hash(&item);

        let name = syn::LitStr::new(&name.to_string(), name.span());

        expand_any(&self.path, type_hash, &name, &[], &tokens, &generics)
    }
}

/// An internal call to the macro.
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

        let Ok(attr) = cx.type_attrs(&self.input.attrs) else {
            return Err(cx.errors.into_inner());
        };

        let tokens = cx.tokens_with_module(attr.module.as_ref());

        let generics = &self.input.generics;
        let mut installers = Vec::new();

        let Ok(()) = expand_install_with(&cx, &self.input, &tokens, &attr, generics, &mut installers) else {
            return Err(cx.errors.into_inner());
        };

        let name = match &attr.name {
            Some(name) => name,
            None => &self.input.ident,
        };

        let ident = &self.input.ident;

        let mut item = match &attr.item {
            Some(item) => item.clone(),
            None => syn::Path {
                leading_colon: None,
                segments: Punctuated::default(),
            },
        };

        item.segments.push(syn::PathSegment::from(name.clone()));
        let type_hash = build_type_hash(&item);

        let name = syn::LitStr::new(&name.to_string(), name.span());

        expand_any(ident, type_hash, &name, &installers, &tokens, generics)
    }
}

fn build_type_hash(item: &syn::Path) -> Hash {
    // Construct type hash.
    let mut buf = ItemBuf::new();
    let mut first = item.leading_colon.is_some();

    for s in &item.segments {
        let ident = s.ident.to_string();

        if take(&mut first) {
            buf.push(ComponentRef::Crate(&ident));
        } else {
            buf.push(ComponentRef::Str(&ident));
        }
    }

    Hash::type_hash(&buf)
}

/// Expannd the install into impl.
pub(crate) fn expand_install_with(
    cx: &Context,
    input: &syn::DeriveInput,
    tokens: &Tokens,
    attr: &TypeAttr,
    generics: &syn::Generics,
    installers: &mut Vec<TokenStream>,
) -> Result<(), ()> {
    let ident = &input.ident;

    match &input.data {
        syn::Data::Struct(st) => {
            expand_struct_install_with(cx, installers, ident, st, tokens, attr)?;
        }
        syn::Data::Enum(en) => {
            expand_enum_install_with(cx, installers, ident, en, tokens, attr, generics)?;
        }
        syn::Data::Union(..) => {
            cx.error(syn::Error::new_spanned(
                input,
                "#[derive(Any)]: Not supported on unions",
            ));
            return Err(());
        }
    }

    if let Some(install_with) = &attr.install_with {
        installers.push(quote_spanned! { input.span() =>
            #install_with(module)?;
        });
    }

    Ok(())
}

fn expand_struct_install_with(
    cx: &Context,
    installers: &mut Vec<TokenStream>,
    ident: &syn::Ident,
    st: &syn::DataStruct,
    tokens: &Tokens,
    attr: &TypeAttr,
) -> Result<(), ()> {
    for (n, field) in st.fields.iter().enumerate() {
        let attrs = cx.field_attrs(&field.attrs)?;
        let name;
        let index;

        let target = match &field.ident {
            Some(ident) => {
                name = syn::LitStr::new(&ident.to_string(), ident.span());

                GenerateTarget::Named {
                    field_ident: ident,
                    field_name: &name,
                }
            }
            None => {
                index = syn::LitInt::new(&n.to_string(), field.span());

                GenerateTarget::Numbered {
                    field_index: &index,
                }
            }
        };

        let ty = &field.ty;

        for protocol in &attrs.protocols {
            installers.push((protocol.generate)(Generate {
                tokens,
                protocol,
                attrs: &attrs,
                field,
                ty,
                target,
            }));
        }
    }

    let mut docs = syn::ExprArray {
        attrs: Vec::new(),
        bracket_token: syn::token::Bracket::default(),
        elems: Punctuated::default(),
    };

    for el in &attr.docs {
        docs.elems.push(el.clone());
    }

    match &st.fields {
        syn::Fields::Named(fields) => {
            let constructor = attr
                .constructor
                .then(|| {
                    let args = fields.named.iter().map(|f| {
                        let ident = f.ident.as_ref().expect("named fields must have an Ident");
                        let typ = &f.ty;
                        quote!(#ident: #typ)
                    });

                    let field_names = fields.named.iter().map(|f| f.ident.as_ref());

                    quote!(|#(#args),*| {
                        #ident {
                            #(#field_names),*
                        }
                    })
                })
                .map(|c| quote!(.constructor(#c)?));

            let fields = fields.named.iter().flat_map(|f| {
                let ident = f.ident.as_ref()?;
                Some(syn::LitStr::new(&ident.to_string(), ident.span()))
            });

            installers.push(quote! {
                module.type_meta::<Self>()?.make_named_struct(&[#(#fields,)*])?#constructor.static_docs(&#docs);
            });
        }
        syn::Fields::Unnamed(fields) => {
            let len = fields.unnamed.len();

            installers.push(quote! {
                module.type_meta::<Self>()?.make_unnamed_struct(#len)?.static_docs(&#docs);
            });
        }
        syn::Fields::Unit => {
            installers.push(quote! {
                module.type_meta::<Self>()?.make_empty_struct()?.static_docs(&#docs);
            });
        }
    }

    Ok(())
}

fn expand_enum_install_with(
    cx: &Context,
    installers: &mut Vec<TokenStream>,
    ident: &syn::Ident,
    en: &syn::DataEnum,
    tokens: &Tokens,
    attr: &TypeAttr,
    generics: &syn::Generics,
) -> Result<(), ()> {
    let Tokens {
        protocol,
        to_value,
        type_of,
        vm_result,
        ..
    } = tokens;

    let (_, type_generics, _) = generics.split_for_impl();

    let mut is_variant = Vec::new();
    let mut variant_metas = Vec::new();
    let mut variant_names = Vec::new();
    let mut variants = Vec::new();

    // Protocol::GET implementations per available field. Each implementation
    // needs to match the enum to extract the appropriate field.
    let mut field_fns = BTreeMap::<String, Vec<TokenStream>>::new();
    let mut index_fns = BTreeMap::<usize, Vec<TokenStream>>::new();

    for (variant_index, variant) in en.variants.iter().enumerate() {
        let span = variant.fields.span();

        let variant_attr = cx.variant_attr(&variant.attrs)?;

        let mut variant_docs = syn::ExprArray {
            attrs: Vec::new(),
            bracket_token: syn::token::Bracket::default(),
            elems: Punctuated::default(),
        };

        for el in &variant_attr.docs {
            variant_docs.elems.push(el.clone());
        }

        let variant_ident = &variant.ident;
        variant_names.push(syn::LitStr::new(&variant_ident.to_string(), span));

        is_variant.push(quote!((#ident::#variant_ident { .. }, #variant_index) => true));

        match &variant.fields {
            syn::Fields::Named(fields) => {
                let mut field_names = Vec::new();

                for f in &fields.named {
                    let attrs = cx.field_attrs(&f.attrs)?;

                    let Some(f_ident) = &f.ident else {
                        cx.error(syn::Error::new_spanned(f, "Missing field name"));
                        return Err(());
                    };

                    if attrs.field {
                        let f_name = f_ident.to_string();
                        let name = syn::LitStr::new(&f_name, f.span());
                        field_names.push(name);

                        let fields = field_fns.entry(f_name).or_default();

                        let value = if attrs.copy {
                            quote!(#to_value::to_value(*#f_ident))
                        } else {
                            quote!(#to_value::to_value(#f_ident.clone()))
                        };

                        fields.push(quote!(#ident::#variant_ident { #f_ident, .. } => #value));
                    }
                }

                variant_metas.push(quote! {
                    enum_.variant_mut(#variant_index)?.make_named(&[#(#field_names),*])?.static_docs(&#variant_docs)
                });

                variants.push((None, attr));
            }
            syn::Fields::Unnamed(fields) => {
                let mut fields_len = 0usize;

                for (n, field) in fields.unnamed.iter().enumerate() {
                    let span = field.span();
                    let attrs = cx.field_attrs(&field.attrs)?;

                    if attrs.field {
                        fields_len += 1;
                        let fields = index_fns.entry(n).or_default();
                        let n = syn::LitInt::new(&n.to_string(), span);

                        let value = if attrs.copy {
                            quote!(#to_value::to_value(*value))
                        } else {
                            quote!(#to_value::to_value(value.clone()))
                        };

                        fields.push(quote!(#ident::#variant_ident { #n: value, .. } => #value));
                    }
                }

                variant_metas.push(quote! {
                    enum_.variant_mut(#variant_index)?.make_unnamed(#fields_len)?.static_docs(&#variant_docs)
                });

                let constructor = if variant_attr.constructor {
                    if fields_len != fields.unnamed.len() {
                        cx.error(syn::Error::new_spanned(fields, "#[rune(constructor)] can only be used if all fields are marked with #[rune(get)"));
                        return Err(());
                    }

                    Some(quote!(#ident #type_generics :: #variant_ident))
                } else {
                    None
                };

                variants.push((constructor, attr));
            }
            syn::Fields::Unit => {
                variant_metas.push(quote! {
                    enum_.variant_mut(#variant_index)?.make_empty()?.static_docs(&#variant_docs)
                });

                let constructor = if variant_attr.constructor {
                    Some(quote!(|| #ident #type_generics :: #variant_ident))
                } else {
                    None
                };

                variants.push((constructor, attr));
            }
        }
    }

    let is_variant = quote! {
        module.associated_function(#protocol::IS_VARIANT, |this: &Self, index: usize| {
            match (this, index) {
                #(#is_variant,)*
                _ => false,
            }
        })?;
    };

    installers.push(is_variant);

    for (field, matches) in field_fns {
        installers.push(quote! {
            module.field_function(#protocol::GET, #field, |this: &Self| {
                match this {
                    #(#matches,)*
                    _ => return #vm_result::__rune_macros__unsupported_object_field_get(<Self as #type_of>::type_info()),
                }
            })?;
        });
    }

    for (index, matches) in index_fns {
        installers.push(quote! {
            module.index_function(#protocol::GET, #index, |this: &Self| {
                match this {
                    #(#matches,)*
                    _ => return #vm_result::__rune_macros__unsupported_tuple_index_get(<Self as #type_of>::type_info(), #index),
                }
            })?;
        });
    }

    let mut docs = syn::ExprArray {
        attrs: Vec::new(),
        bracket_token: syn::token::Bracket::default(),
        elems: Punctuated::default(),
    };

    for el in &attr.docs {
        docs.elems.push(el.clone());
    }

    let enum_meta = quote! {
        let mut enum_ = module.type_meta::<Self>()?.make_enum(&[#(#variant_names,)*])?.static_docs(&#docs);
        #(#variant_metas;)*
    };

    installers.push(enum_meta);

    for (index, (constructor, attr)) in variants.iter().enumerate() {
        let mut docs = syn::ExprArray {
            attrs: Vec::new(),
            bracket_token: syn::token::Bracket::default(),
            elems: Punctuated::default(),
        };

        for el in &attr.docs {
            docs.elems.push(el.clone());
        }

        let constructor = constructor.as_ref().map(|c| quote!(.constructor(#c)?));

        installers
            .push(quote!(module.variant_meta::<Self>(#index)?#constructor.static_docs(&#docs);))
    }

    Ok(())
}

/// Expand the necessary implementation details for `Any`.
pub(super) fn expand_any<T>(
    ident: T,
    type_hash: Hash,
    name: &syn::LitStr,
    installers: &[TokenStream],
    tokens: &Tokens,
    generics: &syn::Generics,
) -> Result<TokenStream, Vec<syn::Error>>
where
    T: Copy + ToTokens,
{
    let Tokens {
        any,
        context_error,
        hash,
        module,
        named,
        pointer_guard,
        raw_into_mut,
        raw_into_ref,
        raw_str,
        shared,
        type_info,
        any_type_info,
        type_of,
        maybe_type_of,
        full_type_of,
        unsafe_from_value,
        unsafe_to_value,
        unsafe_to_ref,
        unsafe_to_mut,
        value,
        vm_result,
        install_with,
        ..
    } = &tokens;

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let generic_names = generics.type_params().map(|v| &v.ident).collect::<Vec<_>>();

    let impl_named = if !generic_names.is_empty() {
        quote! {
            #[automatically_derived]
            impl #impl_generics #named for #ident #type_generics #where_clause {
                const BASE_NAME: #raw_str  = #raw_str::from_str(#name);

                fn full_name() -> Box<str> {
                    [#name, "<", &#(#generic_names::full_name(),)* ">"].join("").into_boxed_str()
                }
            }
        }
    } else {
        quote! {
            #[automatically_derived]
            impl #impl_generics #named for #ident #type_generics #where_clause {
                const BASE_NAME: #raw_str = #raw_str::from_str(#name);
            }
        }
    };

    let type_hash = type_hash.into_inner();

    let make_hash = if !generic_names.is_empty() {
        quote!(#hash::new_with_type_parameters(#type_hash, #hash::parameters([#(<#generic_names as #type_of>::type_hash()),*])))
    } else {
        quote!(#hash::new(#type_hash))
    };

    let type_parameters = if !generic_names.is_empty() {
        quote!(#hash::parameters([#(<#generic_names as #type_of>::type_hash()),*]))
    } else {
        quote!(#hash::EMPTY)
    };

    let install_with = quote! {
        #[automatically_derived]
        impl #impl_generics #install_with for #ident #type_generics #where_clause {
            fn install_with(#[allow(unused)] module: &mut #module) -> core::result::Result<(), #context_error> {
                #(#installers)*
                Ok(())
            }
        }
    };

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #any for #ident #type_generics #where_clause {
            fn type_hash() -> #hash {
                #make_hash
            }
        }

        #install_with
        #impl_named

        #[automatically_derived]
        impl #impl_generics #type_of for #ident #type_generics #where_clause {
            #[inline]
            fn type_hash() -> #hash {
                <Self as #any>::type_hash()
            }

            #[inline]
            fn type_parameters() -> #hash {
                #type_parameters
            }

            #[inline]
            fn type_info() -> #type_info {
                #type_info::Any(#any_type_info::new(#raw_str::from_str(core::any::type_name::<Self>())))
            }
        }

        #[automatically_derived]
        impl #impl_generics #maybe_type_of for #ident #type_generics #where_clause {
            #[inline]
            fn maybe_type_of() -> Option<#full_type_of> {
                Some(<Self as #type_of>::type_of())
            }
        }

        #[automatically_derived]
        impl #impl_generics #unsafe_to_ref for #ident #type_generics #where_clause {
            type Guard = #raw_into_ref;

            unsafe fn unsafe_to_ref<'a>(value: #value) -> #vm_result<(&'a Self, Self::Guard)> {
                let (value, guard) = match value.into_any_ptr() {
                    #vm_result::Ok(value) => value,
                    #vm_result::Err(err) => return #vm_result::Err(err),
                };

                #vm_result::Ok((&*value, guard))
            }
        }

        #[automatically_derived]
        impl #impl_generics #unsafe_to_mut for #ident #type_generics #where_clause {
            type Guard = #raw_into_mut;

            unsafe fn unsafe_to_mut<'a>(value: #value) -> #vm_result<(&'a mut Self, Self::Guard)> {
                let (value, guard) = match value.into_any_mut() {
                    #vm_result::Ok(value) => value,
                    #vm_result::Err(err) => return #vm_result::Err(err),
                };

                #vm_result::Ok((&mut *value, guard))
            }
        }

        #[automatically_derived]
        impl #impl_generics #unsafe_from_value for &#ident #type_generics #where_clause {
            type Output = *const #ident #type_generics;
            type Guard = #raw_into_ref;

            #[inline]
            fn unsafe_from_value(value: #value) -> #vm_result<(Self::Output, Self::Guard)> {
                value.into_any_ptr()
            }

            unsafe fn unsafe_coerce(output: Self::Output) -> Self {
                &*output
            }
        }

        #[automatically_derived]
        impl #impl_generics #unsafe_from_value for &mut #ident #type_generics #where_clause {
            type Output = *mut #ident  #type_generics;
            type Guard = #raw_into_mut;

            fn unsafe_from_value(value: #value) -> #vm_result<(Self::Output, Self::Guard)> {
                value.into_any_mut()
            }

            unsafe fn unsafe_coerce(output: Self::Output) -> Self {
                &mut *output
            }
        }

        #[automatically_derived]
        impl #impl_generics #unsafe_to_value for &#ident #type_generics #where_clause {
            type Guard = #pointer_guard;

            unsafe fn unsafe_to_value(self) -> #vm_result<(#value, Self::Guard)> {
                let (shared, guard) = #shared::from_ref(self);
                #vm_result::Ok((#value::from(shared), guard))
            }
        }

        #[automatically_derived]
        impl #impl_generics #unsafe_to_value for &mut #ident #type_generics #where_clause {
            type Guard = #pointer_guard;

            unsafe fn unsafe_to_value(self) -> #vm_result<(#value, Self::Guard)> {
                let (shared, guard) = #shared::from_mut(self);
                #vm_result::Ok((#value::from(shared), guard))
            }
        }
    })
}
