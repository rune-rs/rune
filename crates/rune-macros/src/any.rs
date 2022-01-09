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

    if let Some(install_with) = &attrs.install_with {
        installers.push(quote_spanned! { input.span() =>
            #install_with(module)?;
        });
    }

    let mut fields = Vec::new();

    let ident = &input.ident;
    let (_, ty_generics, _) = generics.split_for_impl();

    match &input.data {
        syn::Data::Struct(st) => {
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
        }
        syn::Data::Enum(..) => {
            ctx.errors.push(syn::Error::new_spanned(
                input,
                "`Any` not supported on enums",
            ));
            return None;
        }
        syn::Data::Union(..) => {
            ctx.errors.push(syn::Error::new_spanned(
                input,
                "`Any` not supported on unions",
            ));
            return None;
        }
    }

    installers.push(quote! {
        module.struct_meta::<Self>(&[#(#fields),*][..])?;
    });

    Some(quote! {
        #(#installers)*
        Ok(())
    })
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
