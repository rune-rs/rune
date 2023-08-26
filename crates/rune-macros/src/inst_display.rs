use core::mem::take;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::punctuated::Punctuated;
use syn::Token;

/// The `InstDisplay` derive.
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
        let mut errors = Vec::new();

        let syn::Data::Enum(en) = &self.input.data else {
            errors.push(syn::Error::new_spanned(
                &self.input.ident,
                "InstDisplay is only supported for enums",
            ));
            return Err(errors);
        };

        let fmt = syn::Ident::new("fmt", Span::call_site());
        let ident = self.input.ident;

        let mut variants = Vec::new();

        for variant in &en.variants {
            let variant_ident = &variant.ident;
            let mut patterns = Vec::new();
            let mut fmt_call = Vec::new();

            for (index, f) in variant.fields.iter().enumerate() {
                let mut display_with = None::<syn::Path>;

                for a in &f.attrs {
                    if a.path().is_ident("inst_display") {
                        let result = a.parse_nested_meta(|meta| {
                            if meta.path.is_ident("display_with") {
                                meta.input.parse::<Token![=]>()?;
                                display_with = Some(meta.input.parse()?);
                            } else {
                                return Err(syn::Error::new(
                                    meta.input.span(),
                                    "Unsupported attribute",
                                ));
                            }

                            Ok(())
                        });

                        if let Err(error) = result {
                            errors.push(error);
                            continue;
                        }
                    }
                }

                let member = match &f.ident {
                    Some(ident) => syn::Member::Named(ident.clone()),
                    None => syn::Member::Unnamed(syn::Index::from(index)),
                };

                let (assign, var) = match &f.ident {
                    Some(ident) => (false, ident.clone()),
                    None => (true, quote::format_ident!("_{index}")),
                };

                let mut path = syn::Path {
                    leading_colon: None,
                    segments: Punctuated::default(),
                };

                path.segments.push(syn::PathSegment::from(var.clone()));

                patterns.push(syn::FieldValue {
                    attrs: Vec::new(),
                    member,
                    colon_token: assign.then(<Token![:]>::default),
                    expr: syn::Expr::Path(syn::ExprPath {
                        attrs: Vec::new(),
                        qself: None,
                        path,
                    }),
                });

                let var_name = syn::LitStr::new(&var.to_string(), var.span());

                let var = syn::Expr::Path(syn::ExprPath {
                    attrs: Vec::new(),
                    qself: None,
                    path: syn::Path::from(var),
                });

                let arg = if let Some(display_with) = display_with {
                    let mut call = syn::ExprCall {
                        attrs: Vec::new(),
                        func: Box::new(syn::Expr::Path(syn::ExprPath {
                            attrs: Vec::new(),
                            qself: None,
                            path: display_with.clone(),
                        })),
                        paren_token: syn::token::Paren::default(),
                        args: Punctuated::new(),
                    };

                    call.args.push(var);
                    let call = syn::Expr::Call(call);

                    syn::Expr::Reference(syn::ExprReference {
                        attrs: Vec::new(),
                        and_token: <Token![&]>::default(),
                        mutability: None,
                        expr: Box::new(call),
                    })
                } else {
                    var
                };

                if fmt_call.is_empty() {
                    fmt_call.push(quote! {
                        #fmt::Formatter::write_str(f, " ")?;
                    });
                } else {
                    fmt_call.push(quote! {
                        #fmt::Formatter::write_str(f, ", ")?;
                    });
                }

                fmt_call.push(quote! {
                    #fmt::Formatter::write_str(f, #var_name)?;
                    #fmt::Formatter::write_str(f, "=")?;
                    #fmt::Display::fmt(#arg, f)?
                });
            }

            let variant_name = variant_name(&variant.ident.to_string());

            variants.push(quote! {
                #ident::#variant_ident { #(#patterns,)* } => {
                    #fmt::Formatter::write_str(f, #variant_name)?;
                    #(#fmt_call;)*
                    Ok(())
                }
            });
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        let (impl_g, ty_g, where_g) = self.input.generics.split_for_impl();

        Ok(quote! {
            impl #impl_g #fmt::Display for #ident #ty_g #where_g {
                fn fmt(&self, f: &mut #fmt::Formatter<'_>) -> #fmt::Result {
                    match self {
                        #(#variants,)*
                    }
                }
            }
        })
    }
}

fn variant_name(name: &str) -> String {
    let mut out = String::new();
    let mut first = true;

    for c in name.chars() {
        if take(&mut first) {
            out.extend(c.to_lowercase());
            continue;
        }

        if c.is_uppercase() {
            out.push('-');
            out.extend(c.to_lowercase());
            continue;
        }

        out.push(c);
    }

    out
}
