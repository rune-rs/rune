use std::cell::RefCell;

use crate::internals::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote_spanned;
use quote::{quote, ToTokens};
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned as _;
use syn::Token;

/// Parsed `#[rune(..)]` field attributes.
#[derive(Default)]
pub(crate) struct FieldAttrs {
    /// A field that is an identifier. Should use `Default::default` to be
    /// constructed and ignored during `ToTokens` and `Spanned`.
    pub(crate) id: Option<Span>,
    /// `#[rune(iter)]`
    pub(crate) iter: Option<Span>,
    /// `#[rune(skip)]`
    pub(crate) skip: Option<Span>,
    /// `#[rune(option)]`
    pub(crate) option: Option<Span>,
    /// `#[rune(meta)]`
    pub(crate) meta: Option<Span>,
    /// A single field marked with `#[rune(span)]`.
    pub(crate) span: Option<Span>,
    /// Custom parser `#[rune(parse_with = "..")]`.
    pub(crate) parse_with: Option<syn::Ident>,
    /// `#[rune(..)]` to generate a protocol function.
    pub(crate) protocols: Vec<FieldProtocol>,
    /// `#[rune(copy)]` to indicate that a field is copy and does not need to be
    /// cloned.
    pub(crate) copy: bool,
    /// Whether this field should be known at compile time or not.
    pub(crate) field: bool,
}

impl FieldAttrs {
    /// Indicate if the field should be skipped.
    pub(crate) fn skip(&self) -> bool {
        self.skip.is_some() || self.id.is_some()
    }
}

/// The parsing implementations to build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParseKind {
    /// Generate default functions.
    Default,
    /// Only generate meta parse function.
    MetaOnly,
}

impl Default for ParseKind {
    fn default() -> Self {
        Self::Default
    }
}

/// Parsed field attributes.
#[derive(Default)]
pub(crate) struct TypeAttr {
    /// `#[rune(name = TypeName)]` to override the default type name.
    pub(crate) name: Option<syn::Ident>,
    /// `#[rune(module = <path>)]`.
    pub(crate) module: Option<syn::Path>,
    /// `#[rune(install_with = "...")]`.
    pub(crate) install_with: Option<syn::Path>,
    /// `#[rune(parse = "..")]` type attribute.
    pub(crate) parse: ParseKind,
    /// `#[rune(item = <path>)]`.
    pub(crate) item: Option<syn::Path>,
    /// `#[rune(constructor)]`.
    pub(crate) constructor: bool,
    /// Parsed documentation.
    pub(crate) docs: Vec<syn::Expr>,
    /// Indicates that this is a builtin type, so don't generate an `Any`
    /// implementation for it.
    pub(crate) builtin: Option<Span>,
    /// Indicate a static type to use.
    pub(crate) static_type: Option<syn::Ident>,
    /// Method to use to convert from value.
    pub(crate) from_value: Option<syn::Path>,
    /// Method to use to convert from value.
    pub(crate) from_value_params: Option<syn::punctuated::Punctuated<syn::Type, Token![,]>>,
}

/// Parsed variant attributes.
#[derive(Default)]
pub(crate) struct VariantAttrs {
    /// `#[rune(constructor)]`.
    pub(crate) constructor: bool,
    /// Discovered documentation.
    pub(crate) docs: Vec<syn::Expr>,
}

#[derive(Clone, Copy)]
pub(crate) enum GenerateTarget<'a> {
    Named {
        field_ident: &'a syn::Ident,
        field_name: &'a syn::LitStr,
    },
    Numbered {
        field_index: &'a syn::LitInt,
    },
}

#[derive(Clone)]
pub(crate) struct Generate<'a> {
    pub(crate) tokens: &'a Tokens,
    pub(crate) attrs: &'a FieldAttrs,
    pub(crate) protocol: &'a FieldProtocol,
    pub(crate) field: &'a syn::Field,
    pub(crate) ty: &'a syn::Type,
    pub(crate) target: GenerateTarget<'a>,
}

pub(crate) struct FieldProtocol {
    pub(crate) generate: fn(Generate<'_>) -> TokenStream,
    custom: Option<syn::Path>,
}

#[derive(Default)]
pub(crate) struct Context {
    pub(crate) errors: RefCell<Vec<syn::Error>>,
    pub(crate) module: Option<syn::Path>,
}

impl Context {
    /// Construct a new context.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Construct a new context intended to resolve items inside of the crate
    /// in which it was declared.
    pub(crate) fn with_crate() -> Self {
        let mut crate_module = syn::Path {
            leading_colon: None,
            segments: Punctuated::default(),
        };
        crate_module
            .segments
            .push(syn::PathSegment::from(<Token![crate]>::default()));

        Self {
            errors: RefCell::new(Vec::new()),
            module: Some(crate_module),
        }
    }

    /// Register an error.
    pub(crate) fn error(&self, error: syn::Error) {
        self.errors.borrow_mut().push(error)
    }

    /// Test if context has any errors.
    pub(crate) fn has_errors(&self) -> bool {
        !self.errors.borrow().is_empty()
    }

    /// Get a field identifier.
    pub(crate) fn field_ident<'a>(&self, field: &'a syn::Field) -> Result<&'a syn::Ident, ()> {
        let Some(ident) = &field.ident else {
            self.error(syn::Error::new_spanned(
                field,
                "Unnamed fields are not supported",
            ));
            return Err(());
        };

        Ok(ident)
    }

    /// Parse field attributes.
    pub(crate) fn field_attrs(&self, input: &[syn::Attribute]) -> Result<FieldAttrs, ()> {
        macro_rules! generate_assign {
            ($proto:ident, $op:tt) => {
                |g| {
                    let Generate {
                        ty,
                        target,
                        field,
                        protocol,
                        ..
                    } = g;

                    let protocol_field = g.tokens.protocol($proto);

                    match target {
                        GenerateTarget::Named { field_ident, field_name } => {
                            if let Some(custom) = &protocol.custom {
                                quote_spanned! { field.span() =>
                                    module.field_function(#protocol_field, #field_name, #custom)?;
                                }
                            } else {
                                quote_spanned! { field.span() =>
                                    module.field_function(#protocol_field, #field_name, |s: &mut Self, value: #ty| {
                                        s.#field_ident $op value;
                                    })?;
                                }
                            }
                        }
                        GenerateTarget::Numbered { field_index } => {
                            if let Some(custom) = &protocol.custom {
                                quote_spanned! { field.span() =>
                                    module.index_function(#protocol_field, #field_index, #custom)?;
                                }
                            } else {
                                quote_spanned! { field.span() =>
                                    module.index_function(#protocol_field, #field_index, |s: &mut Self, value: #ty| {
                                        s.#field_index $op value;
                                    })?;
                                }
                            }
                        }
                    }
                }
            };
        }

        let mut error = false;
        let mut attr = FieldAttrs::default();

        for a in input {
            if a.path() != RUNE {
                continue;
            }

            let result = a.parse_nested_meta(|meta| {
                if meta.path == ID {
                    // Parse `#[rune(id)]`
                    attr.id = Some(meta.path.span());
                } else if meta.path == ITER {
                    // `#[rune(iter)]`.
                    attr.iter = Some(meta.path.span());
                } else if meta.path == SKIP {
                    // `#[rune(skip)]`.
                    attr.skip = Some(meta.path.span());
                } else if meta.path == OPTION {
                    // `#[rune(option)]`.
                    attr.option = Some(meta.path.span());
                } else if meta.path == META {
                    // `#[rune(meta)]`.
                    attr.meta = Some(meta.path.span());
                } else if meta.path == SPAN {
                    // `#[rune(span)]`.
                    attr.span = Some(meta.path.span());
                } else if meta.path == COPY {
                    // `#[rune(copy)]`.
                    attr.copy = true;
                } else if meta.path == PARSE_WITH {
                    // Parse `#[rune(parse_with = "..")]`.
                    if let Some(old) = &attr.parse_with {
                        let mut error = syn::Error::new_spanned(
                            &meta.path,
                            "#[rune(parse_with = \"..\")] can only be used once",
                        );

                        error.combine(syn::Error::new_spanned(old, "previously defined here"));
                        return Err(error);
                    }

                    meta.input.parse::<Token![=]>()?;
                    let s = meta.input.parse::<syn::LitStr>()?;
                    attr.parse_with = Some(syn::Ident::new(&s.value(), s.span()));
                } else if meta.path == GET {
                    attr.field = true;
                    attr.protocols.push(FieldProtocol {
                        custom: self.parse_field_custom(meta.input)?,
                        generate: |g| {
                            let Generate {
                                target,
                                ..
                            } = g;

                            match target {
                                GenerateTarget::Named { field_ident, field_name } => {
                                    let access = if g.attrs.copy {
                                        quote!(s.#field_ident)
                                    } else {
                                        quote!(Clone::clone(&s.#field_ident))
                                    };

                                    let protocol = g.tokens.protocol(PROTOCOL_GET);

                                    quote_spanned! { g.field.span() =>
                                        module.field_function(#protocol, #field_name, |s: &Self| #access)?;
                                    }
                                }
                                GenerateTarget::Numbered { field_index } => {
                                    let access = if g.attrs.copy {
                                        quote!(s.#field_index)
                                    } else {
                                        quote!(Clone::clone(&s.#field_index))
                                    };

                                    let protocol = g.tokens.protocol(PROTOCOL_GET);

                                    quote_spanned! { g.field.span() =>
                                        module.index_function(#protocol, #field_index, |s: &Self| #access)?;
                                    }
                                }
                            }
                        },
                    });
                } else if meta.path == SET {
                    attr.protocols.push(FieldProtocol {
                        custom: self.parse_field_custom(meta.input)?,
                        generate: |g| {
                            let Generate {
                                ty,
                                target,
                                ..
                            } = g;

                            let protocol = g.tokens.protocol(PROTOCOL_SET);

                            match target {
                                GenerateTarget::Named { field_ident, field_name } => {
                                    quote_spanned! { g.field.span() =>
                                        module.field_function(#protocol, #field_name, |s: &mut Self, value: #ty| {
                                            s.#field_ident = value;
                                        })?;
                                    }
                                }
                                GenerateTarget::Numbered { field_index } => {
                                    quote_spanned! { g.field.span() =>
                                        module.index_function(#protocol, #field_index, |s: &mut Self, value: #ty| {
                                            s.#field_index = value;
                                        })?;
                                    }
                                }
                            }
                        },
                    });
                } else if meta.path == ADD_ASSIGN {
                    attr.protocols.push(FieldProtocol {
                        custom: self.parse_field_custom(meta.input)?,
                        generate: generate_assign!(PROTOCOL_ADD_ASSIGN, +=),
                    });
                } else if meta.path == SUB_ASSIGN {
                    attr.protocols.push(FieldProtocol {
                        custom: self.parse_field_custom(meta.input)?,
                        generate: generate_assign!(PROTOCOL_SUB_ASSIGN, -=),
                    });
                } else if meta.path == DIV_ASSIGN {
                    attr.protocols.push(FieldProtocol {
                        custom: self.parse_field_custom(meta.input)?,
                        generate: generate_assign!(PROTOCOL_DIV_ASSIGN, /=),
                    });
                } else if meta.path == MUL_ASSIGN {
                    attr.protocols.push(FieldProtocol {
                        custom: self.parse_field_custom(meta.input)?,
                        generate: generate_assign!(PROTOCOL_MUL_ASSIGN, *=),
                    });
                } else if meta.path == BIT_AND_ASSIGN {
                    attr.protocols.push(FieldProtocol {
                        custom: self.parse_field_custom(meta.input)?,
                        generate: generate_assign!(PROTOCOL_BIT_AND_ASSIGN, &=),
                    });
                } else if meta.path == BIT_OR_ASSIGN {
                    attr.protocols.push(FieldProtocol {
                        custom: self.parse_field_custom(meta.input)?,
                        generate: generate_assign!(PROTOCOL_BIT_OR_ASSIGN, |=),
                    });
                } else if meta.path == BIT_XOR_ASSIGN {
                    attr.protocols.push(FieldProtocol {
                        custom: self.parse_field_custom(meta.input)?,
                        generate: generate_assign!(PROTOCOL_BIT_XOR_ASSIGN, ^=),
                    });
                } else if meta.path == SHL_ASSIGN {
                    attr.protocols.push(FieldProtocol {
                        custom: self.parse_field_custom(meta.input)?,
                        generate: generate_assign!(PROTOCOL_SHL_ASSIGN, <<=),
                    });
                } else if meta.path == SHR_ASSIGN {
                    attr.protocols.push(FieldProtocol {
                        custom: self.parse_field_custom(meta.input)?,
                        generate: generate_assign!(PROTOCOL_SHR_ASSIGN, >>=),
                    });
                } else if meta.path == REM_ASSIGN {
                    attr.protocols.push(FieldProtocol {
                        custom: self.parse_field_custom(meta.input)?,
                        generate: generate_assign!(PROTOCOL_REM_ASSIGN, %=),
                    });
                } else {
                    return Err(syn::Error::new_spanned(&meta.path, "Unsupported attribute"));
                }

                Ok(())
            });

            if let Err(e) = result {
                error = true;
                self.error(e);
            }
        }

        if error {
            return Err(());
        }

        Ok(attr)
    }

    /// Parse field attributes.
    pub(crate) fn type_attrs(&self, input: &[syn::Attribute]) -> Result<TypeAttr, ()> {
        let mut error = false;
        let mut attr = TypeAttr::default();

        for a in input {
            if a.path().is_ident("doc") {
                if let syn::Meta::NameValue(meta) = &a.meta {
                    attr.docs.push(meta.value.clone());
                }

                continue;
            }

            if a.path() == RUNE {
                let result = a.parse_nested_meta(|meta| {
                    if meta.path == PARSE {
                        // Parse `#[rune(parse = "..")]`
                        meta.input.parse::<Token![=]>()?;
                        let s: syn::LitStr = meta.input.parse()?;

                        match s.value().as_str() {
                            "meta_only" => {
                                attr.parse = ParseKind::MetaOnly;
                            }
                            other => {
                                return Err(syn::Error::new(
                                    meta.input.span(),
                                    format!(
                                        "Unsupported `#[rune(parse = ..)]` argument `{}`",
                                        other
                                    ),
                                ));
                            }
                        };
                    } else if meta.path == ITEM {
                        // Parse `#[rune(item = "..")]`
                        meta.input.parse::<Token![=]>()?;
                        attr.item = Some(meta.input.parse()?);
                    } else if meta.path == NAME {
                        // Parse `#[rune(name = "..")]`
                        meta.input.parse::<Token![=]>()?;
                        attr.name = Some(meta.input.parse()?);
                    } else if meta.path == MODULE {
                        // Parse `#[rune(module = <path>)]`
                        meta.input.parse::<Token![=]>()?;
                        attr.module = Some(parse_path_compat(meta.input)?);
                    } else if meta.path == INSTALL_WITH {
                        // Parse `#[rune(install_with = <path>)]`
                        meta.input.parse::<Token![=]>()?;
                        attr.install_with = Some(parse_path_compat(meta.input)?);
                    } else if meta.path == CONSTRUCTOR {
                        attr.constructor = true;
                    } else if meta.path == BUILTIN {
                        attr.builtin = Some(meta.path.span());
                    } else if meta.path == STATIC_TYPE {
                        meta.input.parse::<Token![=]>()?;
                        attr.static_type = Some(meta.input.parse()?);
                    } else if meta.path == FROM_VALUE {
                        meta.input.parse::<Token![=]>()?;
                        attr.from_value = Some(meta.input.parse()?);
                    } else if meta.path == FROM_VALUE_PARAMS {
                        meta.input.parse::<Token![=]>()?;
                        let content;
                        syn::bracketed!(content in meta.input);
                        attr.from_value_params =
                            Some(syn::punctuated::Punctuated::parse_terminated(&content)?);
                    } else {
                        return Err(syn::Error::new_spanned(
                            &meta.path,
                            "Unsupported type attribute",
                        ));
                    }

                    Ok(())
                });

                if let Err(e) = result {
                    error = true;
                    self.error(e);
                };
            }
        }

        if error {
            return Err(());
        }

        Ok(attr)
    }

    /// Parse and extract variant attributes.
    pub(crate) fn variant_attr(&self, input: &[syn::Attribute]) -> Result<VariantAttrs, ()> {
        let mut attr = VariantAttrs::default();
        let mut error = false;

        for a in input {
            if a.path().is_ident("doc") {
                if let syn::Meta::NameValue(meta) = &a.meta {
                    attr.docs.push(meta.value.clone());
                }

                continue;
            }

            if a.path() == RUNE {
                let result = a.parse_nested_meta(|meta| {
                    if meta.path == CONSTRUCTOR {
                        if attr.constructor {
                            return Err(syn::Error::new_spanned(
                                &meta.path,
                                "#[rune(constructor)] must only be used once",
                            ));
                        }

                        attr.constructor = true;
                    } else {
                        return Err(syn::Error::new_spanned(&meta.path, "Unsupported attribute"));
                    }

                    Ok(())
                });

                if let Err(e) = result {
                    error = true;
                    self.error(e);
                };
            }
        }

        if error {
            return Err(());
        }

        Ok(attr)
    }

    /// Parse path to custom field function.
    fn parse_field_custom(&self, input: ParseStream<'_>) -> Result<Option<syn::Path>, syn::Error> {
        if !input.peek(Token![=]) {
            return Ok(None);
        };

        input.parse::<Token![=]>()?;
        Ok(Some(parse_path_compat(input)?))
    }

    pub(crate) fn tokens_with_module(&self, module: Option<&syn::Path>) -> Tokens {
        let mut core = syn::Path {
            leading_colon: Some(<Token![::]>::default()),
            segments: Punctuated::default(),
        };
        core.segments.push(syn::PathSegment::from(syn::Ident::new(
            "core",
            Span::call_site(),
        )));

        let mut alloc = syn::Path {
            leading_colon: Some(<Token![::]>::default()),
            segments: Punctuated::default(),
        };
        alloc.segments.push(syn::PathSegment::from(syn::Ident::new(
            "alloc",
            Span::call_site(),
        )));

        let mut default_module;

        let m = match module {
            Some(module) => module,
            None => match &self.module {
                Some(module) => module,
                None => {
                    default_module = syn::Path {
                        leading_colon: None,
                        segments: Punctuated::default(),
                    };
                    default_module
                        .segments
                        .push(syn::PathSegment::from(RUNE.to_ident(Span::call_site())));
                    &default_module
                }
            },
        };

        Tokens {
            any_type_info: path(m, ["runtime", "AnyTypeInfo"]),
            any: path(m, ["Any"]),
            box_: path(m, ["__private", "Box"]),
            vec: path(m, ["alloc", "Vec"]),
            clone: path(&core, ["clone", "Clone"]),
            compile_error: path(m, ["compile", "Error"]),
            context_error: path(m, ["compile", "ContextError"]),
            double_ended_iterator: path(&core, ["iter", "DoubleEndedIterator"]),
            from_value: path(m, ["runtime", "FromValue"]),
            full_type_of: path(m, ["runtime", "FullTypeOf"]),
            hash: path(m, ["Hash"]),
            id: path(m, ["parse", "Id"]),
            install_with: path(m, ["__private", "InstallWith"]),
            into_iterator: path(&core, ["iter", "IntoIterator"]),
            iterator: path(&core, ["iter", "Iterator"]),
            macro_context: path(m, ["macros", "MacroContext"]),
            maybe_type_of: path(m, ["runtime", "MaybeTypeOf"]),
            module: path(m, ["__private", "Module"]),
            mut_: path(m, ["runtime", "Mut"]),
            named: path(m, ["compile", "Named"]),
            non_null: path(&core, ["ptr", "NonNull"]),
            object: path(m, ["runtime", "Object"]),
            opaque: path(m, ["parse", "Opaque"]),
            option_spanned: path(m, ["ast", "OptionSpanned"]),
            option: path(&core, ["option", "Option"]),
            owned_tuple: path(m, ["runtime", "OwnedTuple"]),
            parse: path(m, ["parse", "Parse"]),
            parser: path(m, ["parse", "Parser"]),
            pointer_guard: path(m, ["runtime", "SharedPointerGuard"]),
            protocol: path(m, ["runtime", "Protocol"]),
            raw_into_mut: path(m, ["runtime", "RawMut"]),
            raw_into_ref: path(m, ["runtime", "RawRef"]),
            raw_mut: path(m, ["runtime", "RawMut"]),
            raw_ref: path(m, ["runtime", "RawRef"]),
            raw_str: path(m, ["runtime", "RawStr"]),
            ref_: path(m, ["runtime", "Ref"]),
            result: path(&core, ["result", "Result"]),
            shared: path(m, ["runtime", "Shared"]),
            span: path(m, ["ast", "Span"]),
            spanned: path(m, ["ast", "Spanned"]),
            static_type_mod: path(m, ["runtime", "static_type"]),
            string: path(m, ["alloc", "String"]),
            to_tokens: path(m, ["macros", "ToTokens"]),
            to_value: path(m, ["runtime", "ToValue"]),
            token_stream: path(m, ["macros", "TokenStream"]),
            try_from: path(&core, ["convert", "TryFrom"]),
            tuple: path(m, ["runtime", "Tuple"]),
            type_info: path(m, ["runtime", "TypeInfo"]),
            type_name: path(&core, ["any", "type_name"]),
            type_of: path(m, ["runtime", "TypeOf"]),
            unsafe_to_mut: path(m, ["runtime", "UnsafeToMut"]),
            unsafe_to_ref: path(m, ["runtime", "UnsafeToRef"]),
            unsafe_to_value: path(m, ["runtime", "UnsafeToValue"]),
            value: path(m, ["runtime", "Value"]),
            variant_data: path(m, ["runtime", "VariantData"]),
            vm_result: path(m, ["runtime", "VmResult"]),
            vm_try: path(m, ["vm_try"]),
            alloc: path(m, ["alloc"]),
        }
    }
}

fn parse_path_compat(input: ParseStream<'_>) -> syn::Result<syn::Path> {
    if input.peek(syn::LitStr) {
        let path = input
            .parse::<syn::LitStr>()?
            .parse_with(syn::Path::parse_mod_style)?;

        return Err(syn::Error::new_spanned(
            &path,
            format_args!(
                "String literals are no longer supported here, use a path like `{}`",
                path.to_token_stream()
            ),
        ));
    }

    syn::Path::parse_mod_style(input)
}

fn path<const N: usize>(base: &syn::Path, path: [&'static str; N]) -> syn::Path {
    let mut base = base.clone();

    for s in path {
        let ident = syn::Ident::new(s, base.span());
        base.segments.push(syn::PathSegment::from(ident));
    }

    base
}

pub(crate) struct Tokens {
    pub(crate) any_type_info: syn::Path,
    pub(crate) any: syn::Path,
    pub(crate) box_: syn::Path,
    pub(crate) vec: syn::Path,
    pub(crate) clone: syn::Path,
    pub(crate) compile_error: syn::Path,
    pub(crate) context_error: syn::Path,
    pub(crate) double_ended_iterator: syn::Path,
    pub(crate) from_value: syn::Path,
    pub(crate) full_type_of: syn::Path,
    pub(crate) hash: syn::Path,
    pub(crate) id: syn::Path,
    pub(crate) install_with: syn::Path,
    pub(crate) into_iterator: syn::Path,
    pub(crate) iterator: syn::Path,
    pub(crate) macro_context: syn::Path,
    pub(crate) maybe_type_of: syn::Path,
    pub(crate) module: syn::Path,
    pub(crate) mut_: syn::Path,
    pub(crate) named: syn::Path,
    pub(crate) non_null: syn::Path,
    pub(crate) object: syn::Path,
    pub(crate) opaque: syn::Path,
    pub(crate) option_spanned: syn::Path,
    pub(crate) option: syn::Path,
    pub(crate) owned_tuple: syn::Path,
    pub(crate) parse: syn::Path,
    pub(crate) parser: syn::Path,
    pub(crate) pointer_guard: syn::Path,
    pub(crate) protocol: syn::Path,
    pub(crate) raw_into_mut: syn::Path,
    pub(crate) raw_into_ref: syn::Path,
    pub(crate) raw_mut: syn::Path,
    pub(crate) raw_ref: syn::Path,
    pub(crate) raw_str: syn::Path,
    pub(crate) ref_: syn::Path,
    pub(crate) result: syn::Path,
    pub(crate) shared: syn::Path,
    pub(crate) span: syn::Path,
    pub(crate) spanned: syn::Path,
    pub(crate) static_type_mod: syn::Path,
    pub(crate) string: syn::Path,
    pub(crate) to_tokens: syn::Path,
    pub(crate) to_value: syn::Path,
    pub(crate) token_stream: syn::Path,
    pub(crate) try_from: syn::Path,
    pub(crate) tuple: syn::Path,
    pub(crate) type_info: syn::Path,
    pub(crate) type_name: syn::Path,
    pub(crate) type_of: syn::Path,
    pub(crate) unsafe_to_mut: syn::Path,
    pub(crate) unsafe_to_ref: syn::Path,
    pub(crate) unsafe_to_value: syn::Path,
    pub(crate) value: syn::Path,
    pub(crate) variant_data: syn::Path,
    pub(crate) vm_result: syn::Path,
    pub(crate) vm_try: syn::Path,
    pub(crate) alloc: syn::Path,
}

impl Tokens {
    /// Define a tokenstream for the specified protocol
    pub(crate) fn protocol(&self, sym: Symbol) -> TokenStream {
        let protocol = &self.protocol;
        quote!(#protocol::#sym)
    }
}
