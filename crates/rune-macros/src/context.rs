use crate::internals::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote_spanned;
use quote::{quote, ToTokens};
use syn::spanned::Spanned as _;
use syn::Lit;
use syn::Meta::*;
use syn::MetaNameValue;
use syn::NestedMeta::*;

/// Parsed `#[rune(..)]` field attributes.
#[derive(Default)]
pub(crate) struct FieldAttrs {
    /// A field that is an identifier. Should use `Default::default` to be
    /// constructed and ignored during `ToTokens` and `Spanned`.
    pub(crate) id: Option<Span>,
    /// `#[rune(iter)]`
    pub(crate) iter: Option<Span>,
    /// `#[rune(skip)]`
    skip: Option<Span>,
    /// `#[rune(optional)]`
    pub(crate) optional: Option<Span>,
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
pub(crate) struct TypeAttrs {
    /// `#[rune(name = "TypeName")]` to override the default type name.
    pub(crate) name: Option<syn::LitStr>,
    /// `#[rune(module = "...")]`.
    pub(crate) module: Option<syn::Path>,
    /// `#[rune(install_with = "...")]`.
    pub(crate) install_with: Option<syn::Path>,
    /// `#[rune(parse = "..")]` type attribute.
    pub(crate) parse: ParseKind,
}

#[derive(Clone)]
pub(crate) struct Generate<'a> {
    pub(crate) tokens: &'a Tokens,
    pub(crate) attrs: &'a FieldAttrs,
    pub(crate) protocol: &'a FieldProtocol,
    pub(crate) ident: &'a syn::Ident,
    pub(crate) field: &'a syn::Field,
    pub(crate) field_ident: &'a syn::Ident,
    pub(crate) ty: &'a syn::Type,
    pub(crate) name: &'a syn::LitStr,
    pub(crate) ty_generics: &'a syn::TypeGenerics<'a>,
}

pub(crate) struct FieldProtocol {
    pub(crate) generate: fn(Generate<'_>) -> TokenStream,
    custom: Option<syn::Path>,
}

#[derive(Default)]
pub(crate) struct Context {
    pub(crate) errors: Vec<syn::Error>,
    pub(crate) module: Option<TokenStream>,
}

impl Context {
    /// Construct a new context.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Construct a new context intended to resolve items inside of the crate
    /// in which it was declared.
    pub(crate) fn with_crate() -> Self {
        Self {
            errors: Vec::new(),
            module: Some(quote!(crate)),
        }
    }

    /// Construct a new context.
    pub(crate) fn with_module<M>(module: M) -> Self
    where
        M: Copy + ToTokens,
    {
        Self {
            errors: Vec::new(),
            module: Some(module.to_token_stream()),
        }
    }

    /// Get a field identifier.
    pub(crate) fn field_ident<'a>(&mut self, field: &'a syn::Field) -> Option<&'a syn::Ident> {
        match &field.ident {
            Some(ident) => Some(ident),
            None => {
                self.errors.push(syn::Error::new_spanned(
                    field,
                    "unnamed fields are not supported",
                ));
                None
            }
        }
    }

    /// Parse the toplevel component of the attribute, which must be `#[parse(..)]`.
    fn get_meta_items(
        &mut self,
        attr: &syn::Attribute,
        symbol: Symbol,
    ) -> Option<Vec<syn::NestedMeta>> {
        if attr.path != symbol {
            return Some(Vec::new());
        }

        match attr.parse_meta() {
            Ok(List(meta)) => Some(meta.nested.into_iter().collect()),
            Ok(other) => {
                self.errors.push(syn::Error::new_spanned(
                    other,
                    format!("expected #[{}(...)]", symbol),
                ));
                None
            }
            Err(error) => {
                self.errors.push(syn::Error::new(Span::call_site(), error));
                None
            }
        }
    }

    /// Parse field attributes.
    pub(crate) fn field_attrs(&mut self, input: &[syn::Attribute]) -> Option<FieldAttrs> {
        macro_rules! generate_op {
            ($proto:ident, $op:tt) => {
                |g| {
                    let Generate {
                        ident,
                        field_ident,
                        ty,
                        name,
                        ty_generics,
                        ..
                    } = g;

                    let protocol = g.tokens.protocol($proto);

                    if let Some(custom) = &g.protocol.custom {
                        quote_spanned! { g.field.span() =>
                            module.field_fn(#protocol, #name, #custom)?;
                        }
                    } else {
                        quote_spanned! { g.field.span() =>
                            module.field_fn(#protocol, #name, |s: &mut #ident #ty_generics, value: #ty| {
                                s.#field_ident $op value;
                            })?;
                        }
                    }
                }
            };
        }

        let mut attrs = FieldAttrs::default();

        for attr in input {
            #[allow(clippy::never_loop)] // I guess this is on purpose?
            for meta in self.get_meta_items(attr, RUNE)? {
                let span = meta.span();

                match meta {
                    // Parse `#[rune(id)]`
                    Meta(Path(word)) if word == ID => {
                        attrs.id = Some(span);
                    }
                    // Parse `#[rune(iter)]`.
                    Meta(Path(word)) if word == ITER => {
                        attrs.iter = Some(span);
                    }
                    // Parse `#[rune(skip)]`.
                    Meta(Path(word)) if word == SKIP => {
                        attrs.skip = Some(span);
                    }
                    // Parse `#[rune(optional)]`.
                    Meta(Path(word)) if word == OPTIONAL => {
                        attrs.optional = Some(span);
                    }
                    // Parse `#[rune(attributes)]`
                    Meta(Path(word)) if word == META => {
                        attrs.meta = Some(span);
                    }
                    // Parse `#[rune(span)]`
                    Meta(Path(word)) if word == SPAN => {
                        attrs.span = Some(span);
                    }
                    // Parse `#[rune(parse_with = "..")]`.
                    Meta(NameValue(MetaNameValue {
                        path,
                        lit: syn::Lit::Str(s),
                        ..
                    })) if path == PARSE_WITH => {
                        if let Some(old) = attrs.parse_with {
                            let mut error = syn::Error::new_spanned(
                                path,
                                "#[rune(parse_with = \"..\")] can only be used once",
                            );

                            error.combine(syn::Error::new_spanned(old, "previously defined here"));
                            self.errors.push(error);
                            return None;
                        }

                        attrs.parse_with = Some(syn::Ident::new(&s.value(), s.span()));
                    }
                    Meta(Path(path)) if path == COPY => {
                        attrs.copy = true;
                    }
                    Meta(meta) if meta.path() == GET => {
                        attrs.field = true;
                        attrs.protocols.push(FieldProtocol {
                            custom: self.parse_field_custom(meta)?,
                            generate: |g| {
                                let Generate {
                                    ident,
                                    field_ident,
                                    name,
                                    ty_generics,
                                    ..
                                } = g;

                                let access = if g.attrs.copy {
                                    quote!(s.#field_ident)
                                } else {
                                    quote!(Clone::clone(&s.#field_ident))
                                };

                                let protocol = g.tokens.protocol(PROTOCOL_GET);

                                quote_spanned! { g.field.span() =>
                                    module.field_fn(#protocol, #name, |s: &#ident #ty_generics| #access)?;
                                }
                            },
                        });
                    }
                    Meta(meta) if meta.path() == SET => {
                        attrs.protocols.push(FieldProtocol {
                            custom: self.parse_field_custom(meta)?,
                            generate: |g| {
                                let Generate {
                                    ident,
                                    field_ident,
                                    ty,
                                    name,
                                    ty_generics,
                                    ..
                                } = g;

                                let protocol = g.tokens.protocol(PROTOCOL_SET);
                                quote_spanned! { g.field.span() =>
                                    module.field_fn(#protocol, #name, |s: &mut #ident #ty_generics, value: #ty| {
                                        s.#field_ident = value;
                                    })?;
                                }
                            },
                        });
                    }
                    Meta(meta) if meta.path() == ADD_ASSIGN => {
                        attrs.protocols.push(FieldProtocol {
                            custom: self.parse_field_custom(meta)?,
                            generate: generate_op!(PROTOCOL_ADD_ASSIGN, +=),
                        });
                    }
                    Meta(meta) if meta.path() == SUB_ASSIGN => {
                        attrs.protocols.push(FieldProtocol {
                            custom: self.parse_field_custom(meta)?,
                            generate: generate_op!(PROTOCOL_SUB_ASSIGN, -=),
                        });
                    }
                    Meta(meta) if meta.path() == DIV_ASSIGN => {
                        attrs.protocols.push(FieldProtocol {
                            custom: self.parse_field_custom(meta)?,
                            generate: generate_op!(PROTOCOL_DIV_ASSIGN, /=),
                        });
                    }
                    Meta(meta) if meta.path() == MUL_ASSIGN => {
                        attrs.protocols.push(FieldProtocol {
                            custom: self.parse_field_custom(meta)?,
                            generate: generate_op!(PROTOCOL_MUL_ASSIGN, *=),
                        });
                    }
                    Meta(meta) if meta.path() == BIT_AND_ASSIGN => {
                        attrs.protocols.push(FieldProtocol {
                            custom: self.parse_field_custom(meta)?,
                            generate: generate_op!(PROTOCOL_BIT_AND_ASSIGN, &=),
                        });
                    }
                    Meta(meta) if meta.path() == BIT_OR_ASSIGN => {
                        attrs.protocols.push(FieldProtocol {
                            custom: self.parse_field_custom(meta)?,
                            generate: generate_op!(PROTOCOL_BIT_OR_ASSIGN, |=),
                        });
                    }
                    Meta(meta) if meta.path() == BIT_XOR_ASSIGN => {
                        attrs.protocols.push(FieldProtocol {
                            custom: self.parse_field_custom(meta)?,
                            generate: generate_op!(PROTOCOL_BIT_XOR_ASSIGN, ^=),
                        });
                    }
                    Meta(meta) if meta.path() == SHL_ASSIGN => {
                        attrs.protocols.push(FieldProtocol {
                            custom: self.parse_field_custom(meta)?,
                            generate: generate_op!(PROTOCOL_SHL_ASSIGN, <<=),
                        });
                    }
                    Meta(meta) if meta.path() == SHR_ASSIGN => {
                        attrs.protocols.push(FieldProtocol {
                            custom: self.parse_field_custom(meta)?,
                            generate: generate_op!(PROTOCOL_SHR_ASSIGN, >>=),
                        });
                    }
                    Meta(meta) if meta.path() == REM_ASSIGN => {
                        attrs.protocols.push(FieldProtocol {
                            custom: self.parse_field_custom(meta)?,
                            generate: generate_op!(PROTOCOL_REM_ASSIGN, %=),
                        });
                    }
                    _ => {
                        self.errors
                            .push(syn::Error::new_spanned(meta, "unsupported attribute"));

                        return None;
                    }
                }
            }
        }

        Some(attrs)
    }

    /// Parse field attributes.
    pub(crate) fn type_attrs(&mut self, input: &[syn::Attribute]) -> Option<TypeAttrs> {
        let mut attrs = TypeAttrs::default();

        for attr in input {
            for meta in self.get_meta_items(attr, RUNE)? {
                let span = meta.span();

                match meta {
                    // Parse `#[rune(parse = "..")]`
                    Meta(NameValue(MetaNameValue {
                        path,
                        lit: syn::Lit::Str(s),
                        ..
                    })) if path == PARSE => {
                        let parse = match s.value().as_str() {
                            "meta_only" => ParseKind::MetaOnly,
                            other => {
                                self.errors.push(syn::Error::new(
                                    span,
                                    format!(
                                        "unsupported `#[rune(parse = ..)]` argument `{}`",
                                        other
                                    ),
                                ));
                                return None;
                            }
                        };

                        attrs.parse = parse;
                    }
                    // Parse `#[rune(name = "..")]`.
                    Meta(NameValue(syn::MetaNameValue {
                        path,
                        lit: Lit::Str(name),
                        ..
                    })) if path == NAME => {
                        attrs.name = Some(name);
                    }
                    // Parse `#[rune(module = "..")]`.
                    Meta(NameValue(syn::MetaNameValue {
                        path,
                        lit: Lit::Str(s),
                        ..
                    })) if path == MODULE => {
                        let module = match s.parse_with(syn::Path::parse_mod_style) {
                            Ok(module) => module,
                            Err(e) => {
                                self.errors.push(e);
                                return None;
                            }
                        };

                        attrs.module = Some(module);
                    }
                    // Parse `#[rune(install_with = "..")]`.
                    Meta(NameValue(syn::MetaNameValue {
                        path,
                        lit: Lit::Str(s),
                        ..
                    })) if path == INSTALL_WITH => {
                        let install_with = match s.parse_with(syn::Path::parse_mod_style) {
                            Ok(install_with) => install_with,
                            Err(e) => {
                                self.errors.push(e);
                                return None;
                            }
                        };

                        attrs.install_with = Some(install_with);
                    }
                    meta => {
                        self.errors
                            .push(syn::Error::new_spanned(meta, "unsupported type attribute"));

                        return None;
                    }
                }
            }
        }

        Some(attrs)
    }

    /// Parse path to custom field function.
    fn parse_field_custom(&mut self, meta: syn::Meta) -> Option<Option<syn::Path>> {
        let s = match meta {
            Path(..) => return Some(None),
            NameValue(syn::MetaNameValue {
                lit: syn::Lit::Str(s),
                ..
            }) => s,
            _ => {
                self.errors
                    .push(syn::Error::new(meta.span(), "unsupported meta"));
                return None;
            }
        };

        match s.parse_with(syn::Path::parse_mod_style) {
            Ok(path) => Some(Some(path)),
            Err(error) => {
                self.errors.push(error);
                None
            }
        }
    }

    /// Build an inner spanned decoder from an iterator.
    pub(crate) fn build_spanned_iter<'a>(
        &mut self,
        tokens: &Tokens,
        back: bool,
        mut it: impl Iterator<Item = (Option<TokenStream>, &'a syn::Field)>,
    ) -> Option<(bool, Option<TokenStream>)> {
        let mut quote = None::<TokenStream>;

        loop {
            let (var, field) = match it.next() {
                Some((var, field)) => (var?, field),
                None => {
                    return Some((true, quote));
                }
            };

            let attrs = self.field_attrs(&field.attrs)?;

            let spanned = &tokens.spanned;

            if attrs.skip() {
                continue;
            }

            if attrs.optional.is_some() {
                let option_spanned = &tokens.option_spanned;
                let next = quote_spanned! {
                    field.span() => #option_spanned::option_span(#var)
                };

                if quote.is_some() {
                    quote = Some(quote_spanned! {
                        field.span() => #quote.or_else(|| #next)
                    });
                } else {
                    quote = Some(next);
                }

                continue;
            }

            if attrs.iter.is_some() {
                let next = if back {
                    quote_spanned!(field.span() => next_back)
                } else {
                    quote_spanned!(field.span() => next)
                };

                let spanned = &tokens.spanned;
                let next = quote_spanned! {
                    field.span() => IntoIterator::into_iter(#var).#next().map(#spanned::span)
                };

                if quote.is_some() {
                    quote = Some(quote_spanned! {
                        field.span() => #quote.or_else(|| #next)
                    });
                } else {
                    quote = Some(next);
                }

                continue;
            }

            if quote.is_some() {
                quote = Some(quote_spanned! {
                    field.span() => #quote.unwrap_or_else(|| #spanned::span(#var))
                });
            } else {
                quote = Some(quote_spanned! {
                    field.span() => #spanned::span(#var)
                });
            }

            return Some((false, quote));
        }
    }

    /// Explicit span for fields.
    pub(crate) fn explicit_span(
        &mut self,
        named: &syn::FieldsNamed,
    ) -> Option<Option<TokenStream>> {
        let mut explicit_span = None;

        for field in &named.named {
            let attrs = self.field_attrs(&field.attrs)?;

            if let Some(span) = attrs.span {
                if explicit_span.is_some() {
                    self.errors.push(syn::Error::new(
                        span,
                        "only one field can be marked `#[rune(span)]`",
                    ));
                    return None;
                }

                let ident = &field.ident;

                explicit_span = Some(quote_spanned! {
                    field.span() => self.#ident
                })
            }
        }

        Some(explicit_span)
    }

    pub(crate) fn tokens_with_module(&self, module: Option<&syn::Path>) -> Tokens {
        let module = &match module {
            Some(module) => quote!(#module),
            None => match &self.module {
                Some(module) => module.clone(),
                None => {
                    let rune = RUNE;
                    quote!(#rune)
                }
            },
        };

        Tokens {
            any: quote!(#module::Any),
            context_error: quote!(#module::compile::ContextError),
            from_value: quote!(#module::runtime::FromValue),
            hash: quote!(#module::Hash),
            id: quote!(#module::parse::Id),
            install_with: quote!(#module::compile::InstallWith),
            macro_context: quote!(#module::macros::MacroContext),
            module: quote!(#module::compile::Module),
            named: quote!(#module::compile::Named),
            object: quote!(#module::runtime::Object),
            opaque: quote!(#module::parse::Opaque),
            option_spanned: quote!(#module::ast::OptionSpanned),
            parse_error: quote!(#module::parse::ParseError),
            parse: quote!(#module::parse::Parse),
            parser: quote!(#module::parse::Parser),
            pointer_guard: quote!(#module::runtime::SharedPointerGuard),
            protocol: quote!(#module::runtime::Protocol),
            raw_into_mut: quote!(#module::runtime::RawMut),
            raw_into_ref: quote!(#module::runtime::RawRef),
            raw_str: quote!(#module::runtime::RawStr),
            shared: quote!(#module::runtime::Shared),
            span: quote!(#module::ast::Span),
            spanned: quote!(#module::ast::Spanned),
            to_tokens: quote!(#module::macros::ToTokens),
            to_value: quote!(#module::runtime::ToValue),
            token_stream: quote!(#module::macros::TokenStream),
            tuple: quote!(#module::runtime::Tuple),
            type_info: quote!(#module::runtime::TypeInfo),
            type_of: quote!(#module::runtime::TypeOf),
            unit_struct: quote!(#module::runtime::UnitStruct),
            unsafe_from_value: quote!(#module::runtime::UnsafeFromValue),
            unsafe_to_value: quote!(#module::runtime::UnsafeToValue),
            value: quote!(#module::runtime::Value),
            variant_data: quote!(#module::runtime::VariantData),
            vm_error_kind: quote!(#module::runtime::VmErrorKind),
            vm_error: quote!(#module::runtime::VmError),
        }
    }
}

pub(crate) struct Tokens {
    pub(crate) any: TokenStream,
    pub(crate) context_error: TokenStream,
    pub(crate) from_value: TokenStream,
    pub(crate) hash: TokenStream,
    pub(crate) id: TokenStream,
    pub(crate) install_with: TokenStream,
    pub(crate) macro_context: TokenStream,
    pub(crate) module: TokenStream,
    pub(crate) named: TokenStream,
    pub(crate) object: TokenStream,
    pub(crate) opaque: TokenStream,
    pub(crate) option_spanned: TokenStream,
    pub(crate) parse_error: TokenStream,
    pub(crate) parse: TokenStream,
    pub(crate) parser: TokenStream,
    pub(crate) pointer_guard: TokenStream,
    pub(crate) protocol: TokenStream,
    pub(crate) raw_into_mut: TokenStream,
    pub(crate) raw_into_ref: TokenStream,
    pub(crate) raw_str: TokenStream,
    pub(crate) shared: TokenStream,
    pub(crate) span: TokenStream,
    pub(crate) spanned: TokenStream,
    pub(crate) to_tokens: TokenStream,
    pub(crate) to_value: TokenStream,
    pub(crate) token_stream: TokenStream,
    pub(crate) tuple: TokenStream,
    pub(crate) type_info: TokenStream,
    pub(crate) type_of: TokenStream,
    pub(crate) unit_struct: TokenStream,
    pub(crate) unsafe_from_value: TokenStream,
    pub(crate) unsafe_to_value: TokenStream,
    pub(crate) value: TokenStream,
    pub(crate) variant_data: TokenStream,
    pub(crate) vm_error_kind: TokenStream,
    pub(crate) vm_error: TokenStream,
}

impl Tokens {
    /// Define a tokenstream for the specified protocol
    pub(crate) fn protocol(&self, sym: Symbol) -> TokenStream {
        let protocol = &self.protocol;
        quote!(#protocol::#sym)
    }
}
