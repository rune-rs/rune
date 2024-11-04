use std::collections::BTreeMap;

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use rune_core::hash::Hash;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::Token;

use crate::context::{Context, Generate, GenerateTarget, Tokens, TypeAttr};

struct InternalItem {
    attrs: Vec<syn::Attribute>,
    #[allow(unused)]
    impl_token: Token![impl],
    item: syn::Path,
    #[allow(unused)]
    for_token: Token![for],
    ty: syn::Type,
}

impl syn::parse::Parse for InternalItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attrs = syn::Attribute::parse_outer(input)?;
        let impl_token = input.parse()?;
        let item = input.parse()?;
        let for_token = input.parse()?;
        let ty = input.parse()?;
        Ok(Self {
            impl_token,
            item,
            attrs,
            for_token,
            ty,
        })
    }
}

/// An internal call to the macro.
pub struct InternalCall {
    items: Vec<(InternalItem, Token![;])>,
}

impl syn::parse::Parse for InternalCall {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut items = Vec::new();

        while !input.is_empty() {
            items.push((input.parse()?, input.parse()?));
        }

        Ok(Self { items })
    }
}

impl InternalCall {
    pub(super) fn into_any_builders<'a>(
        self,
        cx: &Context,
        attr: &'a TypeAttr,
        tokens: &'a Tokens,
    ) -> Vec<TypeBuilder<'a, syn::Type>> {
        let mut output = Vec::new();

        for (item, _) in self.items {
            let type_item = match crate::item::build_item(&item.item) {
                Ok(type_item) => type_item,
                Err(error) => {
                    cx.error(error);
                    continue;
                }
            };

            let mut is_generic = None;
            let mut attrs = Vec::new();

            for attr in item.attrs {
                if attr.path().is_ident("generic") {
                    is_generic = Some(attr.path().span());
                    continue;
                }

                attrs.push(attr);
            }

            let args = crate::hash::Arguments::new(item.item);

            let Ok(type_hash) = args.build_type_hash(cx) else {
                continue;
            };

            output.push(TypeBuilder {
                attr,
                ident: item.ty,
                type_hash,
                type_item,
                installers: Vec::new(),
                tokens,
                generics: syn::Generics::default(),
                attrs,
                kind: if is_generic.is_some() {
                    TypeKind::Generic
                } else {
                    TypeKind::Derive
                },
            });
        }

        output
    }
}

/// An internal call to the macro.
pub(super) struct Derive {
    pub(super) input: syn::DeriveInput,
}

impl syn::parse::Parse for Derive {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            input: input.parse()?,
        })
    }
}

impl Derive {
    pub(super) fn into_any_builder<'a>(
        self,
        cx: &Context,
        attr: &'a TypeAttr,
        tokens: &'a Tokens,
    ) -> Result<TypeBuilder<'a, syn::Ident>, ()> {
        let mut installers = Vec::new();

        expand_install_with(cx, &self.input, tokens, attr, &mut installers)?;

        if matches!(&self.input.data, syn::Data::Enum(..)) {
            if let Some(span) = attr.constructor {
                cx.error(syn::Error::new(
                    span,
                    "#[rune(constructor)] is not supported on enums, only its variants",
                ));
            }
        }

        let name = match &attr.name {
            Some(name) => name,
            None => &self.input.ident,
        };

        let mut item = match &attr.item {
            Some(item) => item.clone(),
            None => syn::Path {
                leading_colon: None,
                segments: Punctuated::default(),
            },
        };

        item.segments.push(syn::PathSegment::from(name.clone()));

        let args = crate::hash::Arguments::new(item);
        let type_item = args.build_type_item(cx)?;
        let type_hash = args.build_type_hash(cx)?;

        Ok(TypeBuilder {
            attr,
            ident: self.input.ident,
            type_hash,
            type_item,
            installers,
            tokens,
            generics: self.input.generics,
            attrs: Vec::new(),
            kind: TypeKind::Derive,
        })
    }
}

/// Expannd the install into impl.
pub(crate) fn expand_install_with(
    cx: &Context,
    input: &syn::DeriveInput,
    tokens: &Tokens,
    attr: &TypeAttr,
    installers: &mut Vec<TokenStream>,
) -> Result<(), ()> {
    let ident = &input.ident;

    match &input.data {
        syn::Data::Struct(st) => {
            expand_struct_install_with(cx, installers, ident, st, tokens, attr)?;
        }
        syn::Data::Enum(en) => {
            expand_enum_install_with(cx, installers, ident, en, tokens, attr, &input.generics)?;
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
        let attrs = cx.field_attrs(&field.attrs);
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
                .is_some()
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
                module.type_meta::<Self>()?.make_named_struct(&[#(#fields,)*])?#constructor.static_docs(&#docs)?;
            });
        }
        syn::Fields::Unnamed(fields) => {
            let len = fields.unnamed.len();

            installers.push(quote! {
                module.type_meta::<Self>()?.make_unnamed_struct(#len)?.static_docs(&#docs)?;
            });
        }
        syn::Fields::Unit => {
            installers.push(quote! {
                module.type_meta::<Self>()?.make_empty_struct()?.static_docs(&#docs)?;
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
        runtime_error,
        to_value,
        vm_result,
        vm_try,
        any_t,
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

        let variant_attr = cx.variant_attr(&variant.attrs);

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
                    let attrs = cx.field_attrs(&f.attrs);

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
                            quote!(#vm_result::Ok(#vm_try!(#to_value::to_value(*#f_ident))))
                        } else {
                            quote!(#vm_result::Ok(#vm_try!(#to_value::to_value(#f_ident.clone()))))
                        };

                        fields.push(quote!(#ident::#variant_ident { #f_ident, .. } => #value));
                    }
                }

                variant_metas.push(quote! {
                    enum_.variant_mut(#variant_index)?.make_named(&[#(#field_names),*])?.static_docs(&#variant_docs)?
                });

                variants.push((None, variant_attr));
            }
            syn::Fields::Unnamed(fields) => {
                let mut fields_len = 0usize;

                for (n, field) in fields.unnamed.iter().enumerate() {
                    let span = field.span();
                    let attrs = cx.field_attrs(&field.attrs);

                    if attrs.field {
                        fields_len += 1;
                        let fields = index_fns.entry(n).or_default();
                        let n = syn::LitInt::new(&n.to_string(), span);

                        let value = if attrs.copy {
                            quote!(#vm_result::Ok(#vm_try!(#to_value::to_value(*value))))
                        } else {
                            quote!(#vm_result::Ok(#vm_try!(#to_value::to_value(value.clone()))))
                        };

                        fields.push(quote!(#ident::#variant_ident { #n: value, .. } => #value));
                    }
                }

                variant_metas.push(quote! {
                    enum_.variant_mut(#variant_index)?.make_unnamed(#fields_len)?.static_docs(&#variant_docs)?
                });

                if variant_attr.constructor.is_some() && fields_len != fields.unnamed.len() {
                    cx.error(syn::Error::new_spanned(fields, "#[rune(constructor)] can only be used if all fields are marked with #[rune(get)"));
                }

                let constructor = variant_attr
                    .constructor
                    .is_some()
                    .then(|| quote!(#ident #type_generics :: #variant_ident));

                variants.push((constructor, variant_attr));
            }
            syn::Fields::Unit => {
                variant_metas.push(quote! {
                    enum_.variant_mut(#variant_index)?.make_empty()?.static_docs(&#variant_docs)?
                });

                let constructor = if variant_attr.constructor.is_some() {
                    Some(quote!(|| #ident #type_generics :: #variant_ident))
                } else {
                    None
                };

                variants.push((constructor, variant_attr));
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
                    _ => return #vm_result::err(
                        #runtime_error::__rune_macros__unsupported_object_field_get(
                            <Self as #any_t>::ANY_TYPE_INFO
                        )
                    ),
                }
            })?;
        });
    }

    for (index, matches) in index_fns {
        installers.push(quote! {
            module.index_function(#protocol::GET, #index, |this: &Self| {
                match this {
                    #(#matches,)*
                    _ => return #vm_result::err(
                        #runtime_error::__rune_macros__unsupported_tuple_index_get(
                            <Self as #any_t>::ANY_TYPE_INFO,
                            #index
                        )
                    ),
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
        let mut enum_ = module.type_meta::<Self>()?.make_enum(&[#(#variant_names,)*])?.static_docs(&#docs)?;
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
            .push(quote!(module.variant_meta::<Self>(#index)?#constructor.static_docs(&#docs)?;))
    }

    Ok(())
}

enum TypeKind {
    Derive,
    Generic,
}

pub struct TypeBuilder<'a, T> {
    attr: &'a TypeAttr,
    ident: T,
    /// Hash of the type.
    type_hash: Hash,
    /// Bytes corresponding to the item array.
    type_item: syn::ExprArray,
    installers: Vec<TokenStream>,
    tokens: &'a Tokens,
    generics: syn::Generics,
    attrs: Vec<syn::Attribute>,
    kind: TypeKind,
}

impl<T> TypeBuilder<'_, T>
where
    T: ToTokens,
{
    /// Expand the necessary implementation details for `Any`.
    pub(super) fn expand(self) -> TokenStream {
        match self.kind {
            TypeKind::Derive => self.expand_any(),
            TypeKind::Generic => self.expand_generic(),
        }
    }

    pub(super) fn expand_generic(self) -> TokenStream {
        let TypeBuilder {
            ident,
            type_item,
            installers,
            tokens,
            generics,
            attrs,
            ..
        } = self;

        let Tokens {
            any_t,
            context_error,
            fmt,
            install_with,
            item,
            module,
            named,
            non_null,
            raw_value_guard,
            result,
            unsafe_to_mut,
            unsafe_to_ref,
            unsafe_to_value,
            value_mut_guard,
            value_ref_guard,
            value,
            write,
            runtime_error,
            ..
        } = tokens;

        let generic_names = generics.type_params().map(|v| &v.ident).collect::<Vec<_>>();
        let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

        let named_rest = if let [first_name, remainder @ ..] = &generic_names[..] {
            Some(quote! {
                #write!(f, "<")?;
                #first_name::full_name(f)?;
                #(
                    #write!(f, ", ")?;
                    #remainder::full_name(f)?;
                )*
                #write!(f, ">")?;
            })
        } else {
            None
        };

        let impl_named = quote! {
            #[automatically_derived]
            #(#attrs)*
            impl #impl_generics #named for #ident #type_generics #where_clause {
                const ITEM: &'static #item = unsafe { #item::from_bytes(&#type_item) };

                #[inline]
                fn full_name(f: &mut #fmt::Formatter<'_>) -> #fmt::Result {
                    #fmt::Display::fmt(Self::ITEM, f)?;
                    #named_rest
                    #result::Ok(())
                }
            }
        };

        let install_with = quote! {
            #[automatically_derived]
            #(#attrs)*
            impl #impl_generics #install_with for #ident #type_generics #where_clause {
                fn install_with(#[allow(unused)] module: &mut #module) -> core::result::Result<(), #context_error> {
                    #(#installers)*
                    Ok(())
                }
            }
        };

        let to_value_impl = quote! {
            #[automatically_derived]
            #(#attrs)*
            impl #impl_generics #unsafe_to_ref for #ident #type_generics #where_clause {
                type Guard = #raw_value_guard;

                #[inline]
                unsafe fn unsafe_to_ref<'a>(value: #value) -> #result<(&'a Self, Self::Guard), #runtime_error> {
                    let (value, guard) = #value::into_any_ref_ptr(value)?;
                    #result::Ok((#non_null::as_ref(&value), guard))
                }
            }

            #[automatically_derived]
            #(#attrs)*
            impl #impl_generics #unsafe_to_mut for #ident #type_generics #where_clause {
                type Guard = #raw_value_guard;

                #[inline]
                unsafe fn unsafe_to_mut<'a>(value: #value) -> #result<(&'a mut Self, Self::Guard), #runtime_error> {
                    let (mut value, guard) = #value::into_any_mut_ptr(value)?;
                    #result::Ok((#non_null::as_mut(&mut value), guard))
                }
            }

            #[automatically_derived]
            #(#attrs)*
            impl #impl_generics #unsafe_to_value for &#ident #type_generics #where_clause {
                type Guard = #value_ref_guard;

                #[inline]
                unsafe fn unsafe_to_value(self) -> #result<(#value, Self::Guard), #runtime_error> {
                    let (shared, guard) = #value::from_ref(self)?;
                    #result::Ok((shared, guard))
                }
            }

            #[automatically_derived]
            #(#attrs)*
            impl #impl_generics #unsafe_to_value for &mut #ident #type_generics #where_clause {
                type Guard = #value_mut_guard;

                #[inline]
                unsafe fn unsafe_to_value(self) -> #result<(#value, Self::Guard), #runtime_error> {
                    let (shared, guard) = #value::from_mut(self)?;
                    #result::Ok((shared, guard))
                }
            }
        };

        let impl_any = quote! {
            #[automatically_derived]
            #(#attrs)*
            impl #impl_generics #any_t for #ident #type_generics #where_clause {
            }
        };

        quote! {
            #install_with
            #impl_named
            #to_value_impl
            #impl_any
        }
    }

    pub(super) fn expand_any(self) -> TokenStream {
        let TypeBuilder {
            attr,
            ident,
            type_hash,
            type_item,
            installers,
            tokens,
            generics,
            attrs,
            ..
        } = self;

        let Tokens {
            alloc,
            any_t,
            any_marker_t,
            context_error,
            fmt,
            hash,
            install_with,
            item,
            maybe_type_of,
            meta,
            module,
            named,
            non_null,
            raw_value_guard,
            result,
            any_type_info,
            type_hash_t,
            type_of,
            unsafe_to_mut,
            unsafe_to_ref,
            unsafe_to_value,
            value_mut_guard,
            value_ref_guard,
            value,
            write,
            runtime_error,
            ..
        } = tokens;

        let empty;
        let mut current;
        let generic_names;

        let (impl_generics, type_generics, where_clause) = match &attr.impl_params {
            Some(params) => {
                empty = syn::Generics::default();
                current = syn::Generics::default();

                for p in params {
                    current.params.push(syn::GenericParam::Type(p.clone()));
                }

                let (impl_generics, _, where_clause) = empty.split_for_impl();
                let (_, type_generics, _) = current.split_for_impl();
                generic_names = Vec::new();
                (impl_generics, type_generics, where_clause)
            }
            None => {
                current = generics;
                generic_names = current.type_params().map(|v| &v.ident).collect::<Vec<_>>();
                current.split_for_impl()
            }
        };

        let named_rest = if let [first_name, remainder @ ..] = &generic_names[..] {
            Some(quote! {
                #write!(f, "<")?;
                #first_name::full_name(f)?;
                #(
                    #write!(f, ", ")?;
                    #remainder::full_name(f)?;
                )*
                #write!(f, ">")?;
            })
        } else {
            None
        };

        let impl_named = quote! {
            #[automatically_derived]
            #(#attrs)*
            impl #impl_generics #named for #ident #type_generics #where_clause {
                const ITEM: &'static #item = unsafe { #item::from_bytes(&#type_item) };

                fn full_name(f: &mut #fmt::Formatter<'_>) -> #fmt::Result {
                    #fmt::Display::fmt(Self::ITEM, f)?;
                    #named_rest
                    #result::Ok(())
                }
            }
        };

        let install_with = quote! {
            #[automatically_derived]
            #(#attrs)*
            impl #impl_generics #install_with for #ident #type_generics #where_clause {
                fn install_with(#[allow(unused)] module: &mut #module) -> core::result::Result<(), #context_error> {
                    #(#installers)*
                    Ok(())
                }
            }
        };

        let type_hash = type_hash.into_inner();

        let make_hash = if !generic_names.is_empty() {
            quote!(#hash::new_with_type_parameters(#type_hash, #hash::parameters([#(<#generic_names as #type_hash_t>::HASH),*])))
        } else {
            quote!(#hash::new(#type_hash))
        };

        let type_parameters =
            quote!(#hash::parameters([#(<#generic_names as #type_hash_t>::HASH),*]));

        let to_value_impl = quote! {
            #[automatically_derived]
            #(#attrs)*
            impl #impl_generics #unsafe_to_ref for #ident #type_generics #where_clause {
                type Guard = #raw_value_guard;

                #[inline]
                unsafe fn unsafe_to_ref<'a>(value: #value) -> #result<(&'a Self, Self::Guard), #runtime_error> {
                    let (value, guard) = #value::into_any_ref_ptr(value)?;
                    #result::Ok((#non_null::as_ref(&value), guard))
                }
            }

            #[automatically_derived]
            #(#attrs)*
            impl #impl_generics #unsafe_to_mut for #ident #type_generics #where_clause {
                type Guard = #raw_value_guard;

                #[inline]
                unsafe fn unsafe_to_mut<'a>(value: #value) -> #result<(&'a mut Self, Self::Guard), #runtime_error> {
                    let (mut value, guard) = #value::into_any_mut_ptr(value)?;
                    #result::Ok((#non_null::as_mut(&mut value), guard))
                }
            }

            #[automatically_derived]
            #(#attrs)*
            impl #impl_generics #unsafe_to_value for &#ident #type_generics #where_clause {
                type Guard = #value_ref_guard;

                #[inline]
                unsafe fn unsafe_to_value(self) -> #result<(#value, Self::Guard), #runtime_error> {
                    let (shared, guard) = #value::from_ref(self)?;
                    #result::Ok((shared, guard))
                }
            }

            #[automatically_derived]
            #(#attrs)*
            impl #impl_generics #unsafe_to_value for &mut #ident #type_generics #where_clause {
                type Guard = #value_mut_guard;

                #[inline]
                unsafe fn unsafe_to_value(self) -> #result<(#value, Self::Guard), #runtime_error> {
                    let (shared, guard) = #value::from_mut(self)?;
                    #result::Ok((shared, guard))
                }
            }
        };

        let impl_type_of = quote! {
            #[automatically_derived]
            #(#attrs)*
            impl #impl_generics #type_hash_t for #ident #type_generics #where_clause {
                const HASH: #hash = #make_hash;
            }

            #[automatically_derived]
            #(#attrs)*
            impl #impl_generics #type_of for #ident #type_generics #where_clause {
                const PARAMETERS: #hash = #type_parameters;
                const STATIC_TYPE_INFO: #any_type_info = <Self as #any_t>::ANY_TYPE_INFO;
            }

            #[automatically_derived]
            #(#attrs)*
            impl #impl_generics #maybe_type_of for #ident #type_generics #where_clause {
                #[inline]
                fn maybe_type_of() -> #alloc::Result<#meta::DocType> {
                    #meta::DocType::with_generics(
                        <Self as #type_hash_t>::HASH,
                        [#(<#generic_names as #maybe_type_of>::maybe_type_of()?),*]
                    )
                }
            }
        };

        let impl_any = quote! {
            #[automatically_derived]
            #(#attrs)*
            impl #impl_generics #any_t for #ident #type_generics #where_clause {
            }
        };

        let impl_non_generic = quote! {
            #[automatically_derived]
            #(#attrs)*
            impl #impl_generics #any_marker_t for #ident #type_generics #where_clause {
            }
        };

        quote! {
            #install_with
            #impl_named
            #to_value_impl
            #impl_type_of
            #impl_any
            #impl_non_generic
        }
    }
}
