use std::collections::BTreeMap;
use std::mem::take;

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use rune_core::{ComponentRef, Hash, ItemBuf};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::Token;

use crate::context::{Context, Generate, GenerateTarget, Tokens, TypeAttrs};

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
        let ctx = Context::with_crate();
        let tokens = ctx.tokens_with_module(None);

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

        let expand_into = quote! {
            Ok(())
        };

        let generics = syn::Generics::default();

        let mut item = self.item.clone();
        item.segments.push(syn::PathSegment::from(name.clone()));
        let type_hash = build_type_hash(&item);

        let name = syn::LitStr::new(&name.to_string(), name.span());

        expand_any(
            &self.path,
            type_hash,
            &name,
            &expand_into,
            &tokens,
            &generics,
        )
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
        let ctx = Context::new();

        let Ok(attr) = ctx.type_attrs(&self.input.attrs) else {
            return Err(ctx.errors.into_inner());
        };

        let tokens = ctx.tokens_with_module(attr.module.as_ref());

        let generics = &self.input.generics;

        let Ok(install_with) = expand_install_with(&ctx, &self.input, &tokens, &attr, generics) else {
            return Err(ctx.errors.into_inner());
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

        expand_any(ident, type_hash, &name, &install_with, &tokens, generics)
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
    ctx: &Context,
    input: &syn::DeriveInput,
    tokens: &Tokens,
    attrs: &TypeAttrs,
    generics: &syn::Generics,
) -> Result<TokenStream, ()> {
    let mut installers = Vec::new();

    let ident = &input.ident;

    match &input.data {
        syn::Data::Struct(st) => {
            expand_struct_install_with(ctx, &mut installers, st, tokens)?;
        }
        syn::Data::Enum(en) => {
            expand_enum_install_with(ctx, &mut installers, ident, en, tokens, generics)?;
        }
        syn::Data::Union(..) => {
            ctx.error(syn::Error::new_spanned(
                input,
                "#[derive(Any)]: Not supported on unions",
            ));
            return Err(());
        }
    }

    if let Some(install_with) = &attrs.install_with {
        installers.push(quote_spanned! { input.span() =>
            #install_with(module)?;
        });
    }

    Ok(quote! {
        #(#installers)*
        Ok(())
    })
}

fn expand_struct_install_with(
    ctx: &Context,
    installers: &mut Vec<TokenStream>,
    st: &syn::DataStruct,
    tokens: &Tokens,
) -> Result<(), ()> {
    let mut fields = Vec::new();

    for (n, field) in st.fields.iter().enumerate() {
        let attrs = ctx.field_attrs(&field.attrs)?;
        let name;
        let index;

        let target = match &field.ident {
            Some(ident) => {
                name = syn::LitStr::new(&ident.to_string(), ident.span());

                if attrs.field {
                    fields.push(name.clone());
                }

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

    match &st.fields {
        syn::Fields::Named(..) => {
            let len = fields.len();

            installers.push(quote! {
                module.struct_meta::<Self, #len>([#(#fields),*])?;
            });
        }
        syn::Fields::Unnamed(..) => {}
        syn::Fields::Unit => {}
    }

    Ok(())
}

fn expand_enum_install_with(
    ctx: &Context,
    installers: &mut Vec<TokenStream>,
    ident: &syn::Ident,
    en: &syn::DataEnum,
    tokens: &Tokens,
    generics: &syn::Generics,
) -> Result<(), ()> {
    let Tokens {
        module_variant,
        protocol,
        to_value,
        type_of,
        vm_result,
        ..
    } = tokens;

    let mut is_variant = Vec::new();
    let mut variants = Vec::new();
    let mut constructors = Vec::new();

    // Protocol::GET implementations per available field. Each implementation
    // needs to match the enum to extract the appropriate field.
    let mut field_fns = BTreeMap::<String, Vec<TokenStream>>::new();
    let mut index_fns = BTreeMap::<usize, Vec<TokenStream>>::new();

    for (variant_index, variant) in en.variants.iter().enumerate() {
        let span = variant.fields.span();

        let variant_attrs = ctx.variant_attrs(&variant.attrs)?;
        let variant_ident = &variant.ident;
        let variant_name = syn::LitStr::new(&variant_ident.to_string(), span);

        is_variant.push(quote!((#ident::#variant_ident { .. }, #variant_index) => true));

        match &variant.fields {
            syn::Fields::Named(fields) => {
                let mut field_names = Vec::new();

                for f in &fields.named {
                    let attrs = ctx.field_attrs(&f.attrs)?;

                    let Some(f_ident) = &f.ident else {
                        ctx.error(syn::Error::new_spanned(f, "Missing field name"));
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

                variants.push(quote!((#variant_name, #module_variant::st([#(#field_names),*]))));
            }
            syn::Fields::Unnamed(fields) => {
                let mut fields_len = 0usize;

                for (n, field) in fields.unnamed.iter().enumerate() {
                    let span = field.span();
                    let attrs = ctx.field_attrs(&field.attrs)?;

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

                variants.push(quote!((#variant_name, #module_variant::tuple(#fields_len))));

                if variant_attrs.constructor {
                    if fields_len != fields.unnamed.len() {
                        ctx.error(syn::Error::new_spanned(fields, "#[rune(constructor)] can only be used if all fields are marked with #[rune(get)"));
                        return Err(());
                    }

                    constructors.push(quote!(#variant_index, #ident #generics :: #variant_ident));
                }
            }
            syn::Fields::Unit => {
                variants.push(quote!((#variant_name, #module_variant::unit())));

                if variant_attrs.constructor {
                    constructors
                        .push(quote!(#variant_index, || #ident #generics :: #variant_ident));
                }
            }
        }
    }

    let is_variant = quote! {
        module.inst_fn(#protocol::IS_VARIANT, |this: &#ident #generics, index: usize| {
            match (this, index) {
                #(#is_variant,)*
                _ => false,
            }
        })?;
    };

    installers.push(is_variant);

    for (field, matches) in field_fns {
        installers.push(quote! {
            module.field_fn(#protocol::GET, #field, |this: &#ident #generics| {
                match this {
                    #(#matches,)*
                    _ => return #vm_result::__rune_macros__unsupported_object_field_get(<Self as #type_of>::type_info()),
                }
            })?;
        });
    }

    for (index, matches) in index_fns {
        installers.push(quote! {
            module.index_fn(#protocol::GET, #index, |this: &#ident #generics| {
                match this {
                    #(#matches,)*
                    _ => return #vm_result::__rune_macros__unsupported_tuple_index_get(<Self as #type_of>::type_info()),
                }
            })?;
        });
    }

    let variant_count = en.variants.len();

    let enum_meta = quote! {
        module.enum_meta::<#ident #generics, #variant_count>([#(#variants),*])?;
    };

    installers.push(enum_meta);

    for constructor in constructors {
        installers.push(quote!(module.variant_constructor(#constructor)?;))
    }

    Ok(())
}

/// Expand the necessary implementation details for `Any`.
pub(super) fn expand_any<T>(
    ident: T,
    type_hash: Hash,
    name: &syn::LitStr,
    installers: &TokenStream,
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
        value,
        vm_result,
        install_with,
        ..
    } = &tokens;

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let generic_names = generics.type_params().map(|v| &v.ident).collect::<Vec<_>>();

    let impl_named = if !generic_names.is_empty() {
        quote! {
            #[automatically_derived]
            impl #impl_generics #named for #ident #ty_generics #where_clause {
                const BASE_NAME: #raw_str  = #raw_str::from_str(#name);

                fn full_name() -> Box<str> {
                    [#name, "<", &#(#generic_names::full_name(),)* ">"].join("").into_boxed_str()
                }
            }
        }
    } else {
        quote! {
            #[automatically_derived]
            impl #impl_generics #named for #ident #ty_generics #where_clause {
                const BASE_NAME: #raw_str = #raw_str::from_str(#name);
            }
        }
    };

    let type_hash = type_hash.into_inner();

    let make_hash = if !generic_names.is_empty() {
        quote!(#hash::new_with_parameters(#type_hash, #hash::parameters([#(<#generic_names as #type_of>::type_hash()),*])))
    } else {
        quote!(#hash::new(#type_hash))
    };

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #any for #ident #ty_generics #where_clause {
            fn type_hash() -> #hash {
                #make_hash
            }
        }

        #[automatically_derived]
        impl #impl_generics #install_with for #ident #ty_generics #where_clause {
            fn install_with(module: &mut #module) -> core::result::Result<(), #context_error> {
                #installers
            }
        }

        #impl_named

        #[automatically_derived]
        impl #impl_generics #type_of for #ident #ty_generics #where_clause {
            #[inline]
            fn type_hash() -> #hash {
                <Self as #any>::type_hash()
            }

            #[inline]
            fn type_info() -> #type_info {
                #type_info::Any(#any_type_info::new(#raw_str::from_str(core::any::type_name::<Self>())))
            }
        }

        #[automatically_derived]
        impl #impl_generics #maybe_type_of for #ident #ty_generics #where_clause {
            #[inline]
            fn maybe_type_of() -> Option<#full_type_of> {
                Some(<Self as #type_of>::type_of())
            }
        }

        #[automatically_derived]
        impl #impl_generics #unsafe_from_value for &#ident #ty_generics #where_clause {
            type Output = *const #ident #ty_generics;
            type Guard = #raw_into_ref;

            #[inline]
            fn from_value(value: #value) -> #vm_result<(Self::Output, Self::Guard)> {
                value.into_any_ptr()
            }

            unsafe fn unsafe_coerce(output: Self::Output) -> Self {
                &*output
            }
        }

        #[automatically_derived]
        impl #impl_generics #unsafe_from_value for &mut #ident #ty_generics #where_clause {
            type Output = *mut #ident  #ty_generics;
            type Guard = #raw_into_mut;

            fn from_value(value: #value) -> #vm_result<(Self::Output, Self::Guard)> {
                value.into_any_mut()
            }

            unsafe fn unsafe_coerce(output: Self::Output) -> Self {
                &mut *output
            }
        }

        #[automatically_derived]
        impl #impl_generics #unsafe_to_value for &#ident #ty_generics #where_clause {
            type Guard = #pointer_guard;

            unsafe fn unsafe_to_value(self) -> #vm_result<(#value, Self::Guard)> {
                let (shared, guard) = #shared::from_ref(self);
                #vm_result::Ok((#value::from(shared), guard))
            }
        }

        #[automatically_derived]
        impl #impl_generics #unsafe_to_value for &mut #ident #ty_generics #where_clause {
            type Guard = #pointer_guard;

            unsafe fn unsafe_to_value(self) -> #vm_result<(#value, Self::Guard)> {
                let (shared, guard) = #shared::from_mut(self);
                #vm_result::Ok((#value::from(shared), guard))
            }
        }
    })
}
