use std::collections::BTreeMap;

use crate::context::{Context, Generate, Tokens, TypeAttrs};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::spanned::Spanned as _;

/// An internal call to the macro.
pub struct InternalCall {
    path: syn::Path,
    name: Option<(syn::Token![,], syn::LitStr)>,
}

impl syn::parse::Parse for InternalCall {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let path = input.parse()?;

        let name = if input.peek(syn::Token![,]) {
            Some((input.parse()?, input.parse()?))
        } else {
            None
        };

        Ok(Self { path, name })
    }
}

impl InternalCall {
    pub fn expand(self) -> Result<TokenStream, Vec<syn::Error>> {
        let ctx = Context::with_module(&quote!(crate));
        let tokens = ctx.tokens_with_module(None);

        let name = match self.name {
            Some((_, name)) => quote!(#name),
            None => match self.path.segments.last() {
                Some(last) if last.arguments.is_empty() => quote!(stringify!(#last)),
                _ => {
                    return Err(vec![syn::Error::new(
                        self.path.span(),
                        "expected last component in path to be without parameters,
                        give it an explicit name instead with `, \"Type\"`",
                    )])
                }
            },
        };

        let expand_into = quote! {
            Ok(())
        };

        let generics = syn::Generics::default();
        expand_any(&self.path, &name, &expand_into, &tokens, &generics)
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
        let mut ctx = Context::new();

        let attrs = match ctx.type_attrs(&self.input.attrs) {
            Some(attrs) => attrs,
            None => return Err(ctx.errors),
        };

        let tokens = ctx.tokens_with_module(attrs.module.as_ref());

        let generics = &self.input.generics;
        let install_with =
            match expand_install_with(&mut ctx, &self.input, &tokens, &attrs, generics) {
                Some(install_with) => install_with,
                None => return Err(ctx.errors),
            };

        let name = match attrs.name {
            Some(name) => name,
            None => syn::LitStr::new(&self.input.ident.to_string(), self.input.ident.span()),
        };

        let name = &quote!(#name);
        let ident = &self.input.ident;

        expand_any(&ident, name, &install_with, &tokens, generics)
    }
}

/// Expannd the install into impl.
pub(crate) fn expand_install_with(
    ctx: &mut Context,
    input: &syn::DeriveInput,
    tokens: &Tokens,
    attrs: &TypeAttrs,
    generics: &syn::Generics,
) -> Option<TokenStream> {
    let mut installers = Vec::new();

    let ident = &input.ident;

    match &input.data {
        syn::Data::Struct(st) => {
            expand_struct_install_with(ctx, &mut installers, ident, st, tokens, generics)?;
        }
        syn::Data::Enum(en) => {
            expand_enum_install_with(ctx, &mut installers, ident, en, tokens, generics)?;
        }
        syn::Data::Union(..) => {
            ctx.errors.push(syn::Error::new_spanned(
                input,
                "`Any` not supported on unions",
            ));
            return None;
        }
    }

    if let Some(install_with) = &attrs.install_with {
        installers.push(quote_spanned! { input.span() =>
            #install_with(module)?;
        });
    }

    Some(quote! {
        #(#installers)*
        Ok(())
    })
}

fn expand_struct_install_with(
    ctx: &mut Context,
    installers: &mut Vec<TokenStream>,
    ident: &syn::Ident,
    st: &syn::DataStruct,
    tokens: &Tokens,
    generics: &syn::Generics,
) -> Option<()> {
    let (_, ty_generics, _) = generics.split_for_impl();

    let mut fields = Vec::new();

    for field in &st.fields {
        let attrs = ctx.field_attrs(&field.attrs)?;

        let field_ident = match &field.ident {
            Some(ident) => ident,
            None => {
                if !attrs.protocols.is_empty() {
                    ctx.errors.push(syn::Error::new_spanned(
                        field,
                        "only named fields can be used with protocol generators like `#[rune(get)]`",
                    ));
                    return None;
                }

                continue;
            }
        };

        let ty = &field.ty;
        let name = syn::LitStr::new(&field_ident.to_string(), field_ident.span());

        for protocol in &attrs.protocols {
            installers.push((protocol.generate)(Generate {
                tokens,
                protocol,
                attrs: &attrs,
                ident,
                field,
                field_ident,
                ty,
                name: &name,
                ty_generics: &ty_generics,
            }));
        }

        if attrs.field {
            fields.push(name);
        }
    }

    let len = fields.len();

    installers.push(quote! {
        module.struct_meta::<Self, #len>([#(#fields),*])?;
    });

    Some(())
}

fn expand_enum_install_with(
    ctx: &mut Context,
    installers: &mut Vec<TokenStream>,
    ident: &syn::Ident,
    en: &syn::DataEnum,
    tokens: &Tokens,
    generics: &syn::Generics,
) -> Option<()> {
    let protocol = &tokens.protocol;
    let variant_meta = &tokens.variant;
    let vm_error = &tokens.vm_error;
    let vm_error_kind = &tokens.vm_error_kind;
    let to_value = &tokens.to_value;
    let type_of = &tokens.type_of;

    let mut is_variant = Vec::new();
    let mut variants = Vec::new();
    let mut constructors = Vec::new();

    // Protocol::GET implementations per available field. Each implementation
    // needs to match the enum to extract the appropriate field.
    let mut get = BTreeMap::<String, Vec<TokenStream>>::new();
    let mut get_index = BTreeMap::<usize, Vec<TokenStream>>::new();

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

                    let f_ident = match &f.ident {
                        Some(ident) => ident,
                        None => {
                            ctx.errors
                                .push(syn::Error::new_spanned(f, "missing field name"));
                            return None;
                        }
                    };

                    if attrs.field {
                        let f_name = f_ident.to_string();
                        let name = syn::LitStr::new(&f_name, f.span());
                        field_names.push(name);

                        let fields = get.entry(f_name).or_default();

                        let value = if attrs.copy {
                            quote!(#to_value::to_value(*#f_ident)?)
                        } else {
                            quote!(#to_value::to_value(#f_ident.clone())?)
                        };

                        fields.push(quote!(#ident::#variant_ident { #f_ident, .. } => #value));
                    }
                }

                variants.push(quote!((#variant_name, #variant_meta::st([#(#field_names),*]))));
            }
            syn::Fields::Unnamed(fields) => {
                let mut fields_len = 0usize;

                for (n, field) in fields.unnamed.iter().enumerate() {
                    let span = field.span();
                    let attrs = ctx.field_attrs(&field.attrs)?;

                    if attrs.field {
                        fields_len += 1;
                        let fields = get_index.entry(n).or_default();
                        let n = syn::LitInt::new(&n.to_string(), span);

                        let value = if attrs.copy {
                            quote!(#to_value::to_value(*value)?)
                        } else {
                            quote!(#to_value::to_value(value.clone())?)
                        };

                        fields.push(quote!(#ident::#variant_ident { #n: value, .. } => #value));
                    }
                }

                variants.push(quote!((#variant_name, #variant_meta::tuple(#fields_len))));

                if variant_attrs.constructor {
                    if fields_len != fields.unnamed.len() {
                        ctx.errors.push(syn::Error::new_spanned(fields, "#[rune(constructor)] can only be used if all fields are marked with #[rune(get)"));
                        return None;
                    }

                    constructors.push(quote!(#variant_index, #ident #generics :: #variant_ident));
                }
            }
            syn::Fields::Unit => {
                variants.push(quote!((#variant_name, #variant_meta::unit())));

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

    for (field, matches) in get {
        installers.push(quote! {
            module.field_fn(#protocol::GET, #field, |this: &#ident #generics| {
                Ok::<_, #vm_error>(match this {
                    #(#matches,)*
                    _ => return Err(#vm_error::from(#vm_error_kind::UnsupportedObjectFieldGet {
                        target: <Self as #type_of>::type_info(),
                    })),
                })
            })?;
        });
    }

    for (index, matches) in get_index {
        installers.push(quote! {
            module.index_fn(#protocol::GET, #index, |this: &#ident #generics| {
                Ok::<_, #vm_error>(match this {
                    #(#matches,)*
                    _ => return Err(#vm_error::from(#vm_error_kind::UnsupportedTupleIndexGet {
                        target: <Self as #type_of>::type_info(),
                    })),
                })
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

    Some(())
}

/// Expand the necessary implementation details for `Any`.
pub(super) fn expand_any<T>(
    ident: T,
    name: &TokenStream,
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
        type_of,
        unsafe_from_value,
        unsafe_to_value,
        value,
        vm_error,
        install_with,
        ..
    } = &tokens;

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let generic_names = generics.type_params().map(|v| &v.ident).collect::<Vec<_>>();

    let impl_named = if !generic_names.is_empty() {
        quote! {
            impl #impl_generics #named for #ident #ty_generics #where_clause {
                const BASE_NAME: #raw_str  = #raw_str::from_str(#name);

                fn full_name() -> Box<str> {
                    [#name, "<", &#(#generic_names::full_name(),)* ">"].join("").into_boxed_str()
                }
            }
        }
    } else {
        quote! {
            impl #impl_generics #named for #ident #ty_generics #where_clause {
                const BASE_NAME: #raw_str = #raw_str::from_str(#name);
            }
        }
    };

    Ok(quote! {
        impl #impl_generics #any for #ident #ty_generics #where_clause {
            fn type_hash() -> #hash {
                // Safety: `Hash` asserts that it is layout compatible with `TypeId`.
                // TODO: remove this once we can have transmute-like functionality in a const fn.
                #hash::from_type_id(std::any::TypeId::of::<Self>())
            }
        }

        impl #impl_generics #install_with for #ident #ty_generics #where_clause {
            fn install_with(module: &mut #module) -> ::std::result::Result<(), #context_error> {
                #installers
            }
        }

        #impl_named

        impl #impl_generics #type_of for #ident #ty_generics #where_clause {
            fn type_hash() -> #hash {
                <Self as #any>::type_hash()
            }

            fn type_info() -> #type_info {
                #type_info::Any(#raw_str::from_str(std::any::type_name::<Self>()))
            }
        }

        impl #impl_generics #unsafe_from_value for &#ident #ty_generics #where_clause {
            type Output = *const #ident #ty_generics;
            type Guard = #raw_into_ref;

            fn from_value(
                value: #value,
            ) -> ::std::result::Result<(Self::Output, Self::Guard), #vm_error> {
                value.into_any_ptr()
            }

            unsafe fn unsafe_coerce(output: Self::Output) -> Self {
                &*output
            }
        }

        impl #impl_generics #unsafe_from_value for &mut #ident #ty_generics #where_clause {
            type Output = *mut #ident  #ty_generics;
            type Guard = #raw_into_mut;

            fn from_value(
                value: #value,
            ) -> ::std::result::Result<(Self::Output, Self::Guard), #vm_error> {
                value.into_any_mut()
            }

            unsafe fn unsafe_coerce(output: Self::Output) -> Self {
                &mut *output
            }
        }

        impl #impl_generics #unsafe_to_value for &#ident #ty_generics #where_clause {
            type Guard = #pointer_guard;

            unsafe fn unsafe_to_value(self) -> ::std::result::Result<(#value, Self::Guard), #vm_error> {
                let (shared, guard) = #shared::from_ref(self);
                Ok((#value::from(shared), guard))
            }
        }

        impl #impl_generics #unsafe_to_value for &mut #ident #ty_generics #where_clause {
            type Guard = #pointer_guard;

            unsafe fn unsafe_to_value(self) -> ::std::result::Result<(#value, Self::Guard), #vm_error> {
                let (shared, guard) = #shared::from_mut(self);
                Ok((#value::from(shared), guard))
            }
        }
    })
}
