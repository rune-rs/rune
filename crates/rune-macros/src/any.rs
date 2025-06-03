use std::collections::BTreeMap;

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use rune_core::hash::Hash;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::Token;

use crate::context::{Context, FieldAttr, Generate, GenerateTarget, Tokens, TypeAttr, TypeFields};

struct InternalItem {
    attrs: Vec<syn::Attribute>,
    #[allow(unused)]
    impl_token: Token![impl],
    generics: syn::Generics,
    item: syn::Path,
    #[allow(unused)]
    for_token: Token![for],
    ty: syn::Type,
}

impl syn::parse::Parse for InternalItem {
    #[inline]
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            attrs: syn::Attribute::parse_outer(input)?,
            impl_token: input.parse()?,
            generics: input.parse()?,
            item: input.parse()?,
            for_token: input.parse()?,
            ty: input.parse()?,
        })
    }
}

/// An internal call to the macro.
pub struct InternalCall {
    items: Vec<(InternalItem, Option<Token![;]>)>,
}

impl syn::parse::Parse for InternalCall {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut items = Vec::new();

        while !input.is_empty() {
            let item = input.parse()?;
            let semi = input.parse::<Option<Token![;]>>()?;
            let done = semi.is_none();

            items.push((item, semi));

            if done {
                break;
            }
        }

        Ok(Self { items })
    }
}

impl InternalCall {
    pub(super) fn into_any_builders<'a>(
        self,
        cx: &Context,
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

            let mut any = None;
            let mut type_of = None;
            let mut attrs = Vec::new();

            for attr in item.attrs {
                if attr.path().is_ident("any") {
                    any = Some(attr.path().span());
                    continue;
                }

                if attr.path().is_ident("type_of") {
                    type_of = Some(attr.path().span());
                    continue;
                }

                attrs.push(attr);
            }

            let args = crate::hash::Arguments::new(item.item);

            let Ok(type_hash) = args.build_type_hash(cx) else {
                continue;
            };

            let kind = match (any, type_of) {
                (Some(a), Some(..)) => {
                    cx.error(syn::Error::new(a, "Cannot combine #[any] and #[type_of]"));
                    continue;
                }
                (Some(..), _) => TypeKind::Any,
                (_, Some(..)) => TypeKind::TypeOf,
                (None, None) => TypeKind::Derive,
            };

            output.push(TypeBuilder {
                ident: item.ty,
                type_hash,
                type_item,
                installers: Vec::new(),
                tokens,
                generics: item.generics,
                attrs,
                kind,
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

        let mut item = match &attr.item {
            Some(item) => item.clone(),
            None => syn::Path {
                leading_colon: None,
                segments: Punctuated::default(),
            },
        };

        let name = match &attr.name {
            Some(name) => name,
            None => &self.input.ident,
        };

        item.segments.push(syn::PathSegment::from(name.clone()));

        let args = crate::hash::Arguments::new(item);
        let type_item = args.build_type_item(cx)?;
        let type_hash = args.build_type_hash(cx)?;

        expand_install_with(cx, &self.input, tokens, attr, &mut installers, &args)?;

        if matches!(&self.input.data, syn::Data::Enum(..)) {
            if let Some(span) = attr.constructor.as_span() {
                cx.error(syn::Error::new(
                    span,
                    "#[rune(constructor)] is not supported on enums, only its variants",
                ));
            }
        }

        Ok(TypeBuilder {
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
    args: &crate::hash::Arguments,
) -> Result<(), ()> {
    let ident = &input.ident;

    match &input.data {
        syn::Data::Struct(st) => {
            expand_struct_install_with(cx, installers, ident, st, tokens, attr)?;
        }
        syn::Data::Enum(en) => {
            expand_enum_install_with(
                cx,
                installers,
                ident,
                en,
                tokens,
                attr,
                &input.generics,
                args,
            )?;
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
    let mut field_attrs = Vec::new();

    for (n, field) in st.fields.iter().enumerate() {
        let attr = cx.field_attrs(&field.attrs);

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

        for protocol in &attr.protocols {
            installers.push((protocol.generate)(Generate {
                tokens,
                attr: &attr,
                protocol,
                field,
                ty,
                target,
            }));
        }

        field_attrs.push(attr);
    }

    let mut docs = syn::ExprArray {
        attrs: Vec::new(),
        bracket_token: syn::token::Bracket::default(),
        elems: Punctuated::default(),
    };

    for el in &attr.docs {
        docs.elems.push(el.clone());
    }

    let make_constructor;
    let make_fields;

    match &attr.fields {
        TypeFields::Default => match &st.fields {
            syn::Fields::Named(fields) => {
                make_constructor = attr.constructor.or_implicit(|| {
                    make_named_constructor(
                        tokens,
                        syn::parse_quote!(#ident),
                        &fields.named,
                        &field_attrs,
                    )
                });

                let fields = fields.named.iter().zip(&field_attrs).filter_map(|(f, a)| {
                    if !a.field {
                        return None;
                    }

                    let ident = f.ident.as_ref()?;
                    Some(syn::LitStr::new(&ident.to_string(), ident.span()))
                });

                make_fields = Some(quote!(.make_named_struct(&[#(#fields,)*])?));
            }
            syn::Fields::Unnamed(fields) => {
                make_constructor = attr.constructor.or_implicit(|| {
                    make_unnamed_constructor(
                        tokens,
                        syn::parse_quote!(#ident),
                        &fields.unnamed,
                        &field_attrs,
                    )
                });

                let len = field_attrs.iter().take_while(|f| f.field).count();
                make_fields = Some(quote!(.make_unnamed_struct(#len)?));
            }
            syn::Fields::Unit => {
                make_constructor = attr.constructor.or_implicit(|| quote!(|| #ident));
                make_fields = Some(quote!(.make_empty_struct()?));
            }
        },
        TypeFields::Empty => {
            make_constructor = attr.constructor.as_explicit();
            make_fields = Some(quote!(.make_empty_struct()?));
        }
        TypeFields::Unnamed(n) => {
            make_constructor = attr.constructor.as_explicit();
            make_fields = Some(quote!(.make_unnamed_struct(#n)?));
        }
    }

    let make_constructor = make_constructor.map(|c| quote!(.constructor(#c)?));

    installers.push(quote! {
        module.type_meta::<Self>()?
            .static_docs(&#docs)?
            #make_constructor
            #make_fields;
    });

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
    args: &crate::hash::Arguments,
) -> Result<(), ()> {
    let Tokens {
        protocol,
        runtime_error,
        to_value,
        vm_result,
        any_t,
        hash,
        vm_try,
        ..
    } = tokens;

    let (_, type_generics, _) = generics.split_for_impl();

    let mut is_variant = Vec::new();
    let mut variant_metas = Vec::new();
    let mut variant_names = Vec::new();

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
        let variant_name = variant_ident.to_string();
        variant_names.push(syn::LitStr::new(&variant_name, span));

        let Ok(variant_hash) = args.build_type_hash_with(cx, &variant_name) else {
            continue;
        };

        let variant_hash = variant_hash.into_inner();

        is_variant.push(quote!((#ident::#variant_ident { .. }, #hash(#variant_hash)) => true));

        let mut field_attrs = Vec::new();

        for f in variant.fields.iter() {
            field_attrs.push(cx.field_attrs(&f.attrs));
        }

        let make_constructor;
        let make_fields;

        match &attr.fields {
            TypeFields::Default => match &variant.fields {
                syn::Fields::Named(fields) => {
                    for (f, attrs) in fields.named.iter().zip(&field_attrs) {
                        let Some(field_ident) = &f.ident else {
                            cx.error(syn::Error::new_spanned(f, "Missing field name"));
                            return Err(());
                        };

                        if attrs.field {
                            let field_name = field_ident.to_string();
                            let fields = field_fns.entry(field_name).or_default();
                            let access = attrs.clone_with.decorate(tokens, quote!(#field_ident));
                            fields.push(quote!(#ident::#variant_ident { #field_ident, .. } => #vm_result::Ok(#vm_try!(#to_value::to_value(#access)))));
                        }
                    }

                    make_constructor = variant_attr.constructor.or_implicit(|| {
                        make_named_constructor(
                            tokens,
                            syn::parse_quote!(#ident::#variant_ident),
                            &fields.named,
                            &field_attrs,
                        )
                    });

                    let fields = fields.named.iter().zip(&field_attrs).filter_map(|(f, a)| {
                        if !a.field {
                            return None;
                        }

                        let ident = f.ident.as_ref()?;
                        Some(syn::LitStr::new(&ident.to_string(), ident.span()))
                    });

                    make_fields = Some(quote!(.make_named(&[#(#fields),*])?));
                }
                syn::Fields::Unnamed(fields) => {
                    for (n, field) in fields.unnamed.iter().enumerate() {
                        let attrs = cx.field_attrs(&field.attrs);

                        if attrs.field {
                            let fields = index_fns.entry(n).or_default();
                            let n = syn::LitInt::new(&n.to_string(), field.span());

                            let access = attrs.clone_with.decorate(tokens, quote!(value));
                            fields.push(quote!(#ident::#variant_ident { #n: value, .. } => #vm_result::Ok(#vm_try!(#to_value::to_value(#access)))));
                        }
                    }

                    make_constructor = variant_attr.constructor.or_implicit(|| {
                        make_unnamed_constructor(
                            tokens,
                            syn::parse_quote!(#ident #type_generics :: #variant_ident),
                            &fields.unnamed,
                            &field_attrs,
                        )
                    });

                    let len = field_attrs.iter().take_while(|f| f.field).count();
                    make_fields = Some(quote!(.make_unnamed(#len)?));
                }
                syn::Fields::Unit => {
                    make_constructor = variant_attr
                        .constructor
                        .or_implicit(|| quote!(|| #ident #type_generics :: #variant_ident));
                    make_fields = Some(quote!(.make_empty()?));
                }
            },
            TypeFields::Empty => {
                make_constructor = attr.constructor.as_explicit();
                make_fields = Some(quote!(.make_empty()?));
            }
            TypeFields::Unnamed(n) => {
                make_constructor = attr.constructor.as_explicit();
                make_fields = Some(quote!(.make_unnamed(#n)?));
            }
        }

        let make_constructor = make_constructor.map(|c| quote!(.constructor(#c)?));

        variant_metas.push(quote! {
            enum_.variant_mut(#variant_index)?
                .static_docs(&#variant_docs)?
                #make_fields
                #make_constructor;
        });
    }

    let is_variant = quote! {
        module.associated_function(&#protocol::IS_VARIANT, |this: &Self, hash: #hash| {
            match (this, hash) {
                #(#is_variant,)*
                _ => false,
            }
        })?;
    };

    installers.push(is_variant);

    for (field, matches) in field_fns {
        installers.push(quote! {
            module.field_function(&#protocol::GET, #field, |this: &Self| {
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
            module.index_function(&#protocol::GET, #index, |this: &Self| {
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
        #(#variant_metas)*
    };

    installers.push(enum_meta);
    Ok(())
}

enum TypeKind {
    Any,
    TypeOf,
    Derive,
}

pub struct TypeBuilder<'a, T> {
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
            TypeKind::Derive => self.expand_derive(),
            TypeKind::Any => self.expand_any(),
            TypeKind::TypeOf => self.expand_type_of(),
        }
    }

    pub(super) fn expand_derive(self) -> TokenStream {
        let TypeBuilder {
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

    pub(super) fn expand_any(self) -> TokenStream {
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

    pub(super) fn expand_type_of(self) -> TokenStream {
        let TypeBuilder {
            ident,
            type_item,
            tokens,
            attrs,
            generics,
            type_hash,
            ..
        } = self;

        let Tokens {
            type_hash_t,
            hash,
            maybe_type_of,
            any_type_info,
            fmt,
            meta,
            item,
            type_of,
            alloc,
            ..
        } = tokens;

        let p = generics.type_params().collect::<Vec<_>>();

        let type_hash = type_hash.into_inner();
        let make_hash = quote!(#hash::new(#type_hash));

        quote! {
            #[automatically_derived]
            #(#attrs)*
            impl #generics #type_hash_t for #ident {
                const HASH: #hash = #make_hash;
            }

            #[automatically_derived]
            #(#attrs)*
            impl #generics #type_of for #ident
            where
                #(#p: #maybe_type_of,)*
            {
                const STATIC_TYPE_INFO: #any_type_info = #any_type_info::new(
                    {
                        fn full_name(f: &mut #fmt::Formatter<'_>) -> #fmt::Result {
                            write!(f, "{}", unsafe { #item::from_bytes(&#type_item) })
                        }

                        full_name
                    },
                    <Self as #type_hash_t>::HASH,
                );
            }

            #[automatically_derived]
            #(#attrs)*
            impl #generics #maybe_type_of for #ident
            where
                #(#p: #maybe_type_of,)*
            {
                #[inline]
                fn maybe_type_of() -> #alloc::Result<#meta::DocType> {
                    Ok(#meta::DocType::new(<Self as #type_hash_t>::HASH))
                }
            }
        }
    }
}

fn make_named_constructor(
    tokens: &Tokens,
    path: syn::Path,
    named: &Punctuated<syn::Field, Token![,]>,
    attrs: &[FieldAttr],
) -> TokenStream {
    let Tokens { default, .. } = tokens;

    let args = named
        .iter()
        .zip(attrs)
        .flat_map(|(syn::Field { ident, ty, .. }, a)| {
            if !a.field {
                return None;
            }

            let ident = ident.as_ref()?;
            Some(quote!(#ident: #ty))
        });

    let field_names = named.iter().zip(attrs).flat_map(|(f, a)| {
        if !a.field {
            return None;
        }

        f.ident.as_ref()
    });

    // Pad out remaining fields with calls to `Default::default()`.
    let remaining = named
        .iter()
        .zip(attrs)
        .filter(|(_, a)| !a.field)
        .filter_map(|(syn::Field { ident, ty, .. }, _)| {
            let ident = ident.as_ref()?;
            Some(quote!(#ident: <#ty as #default>::default()))
        });

    quote! {
        |#(#args,)*| #path { #(#field_names,)* #(#remaining,)* }
    }
}

fn make_unnamed_constructor(
    tokens: &Tokens,
    path: syn::Path,
    named: &Punctuated<syn::Field, Token![,]>,
    attrs: &[FieldAttr],
) -> TokenStream {
    // If all fields are visible, then we can simply just return the path as a
    // constructor. Otherwise we need to pad them out with default impls.
    if attrs.iter().all(|f| f.field) {
        return quote!(#path);
    }

    let Tokens { default, .. } = tokens;

    let field_names = named
        .iter()
        .zip(attrs)
        .enumerate()
        .take_while(|(_, (_, a))| a.field)
        .map(|(n, _)| quote::format_ident!("v{n}"));

    let args = named
        .iter()
        .zip(field_names.clone())
        .flat_map(|(syn::Field { ty, .. }, ident)| Some(quote!(#ident: #ty)));

    // Pad out remaining fields with calls to `Default::default()`.
    let remaining = named
        .iter()
        .zip(attrs)
        .skip_while(|(_, a)| a.field)
        .map(|(syn::Field { ty, .. }, _)| quote!(<#ty as #default>::default()));

    quote! {
        |#(#args,)*| #path(#(#field_names,)* #(#remaining,)*)
    }
}
