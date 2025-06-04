use std::cell::RefCell;

use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote_spanned;
use quote::{quote, ToTokens};
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned as _;
use syn::Token;

use rune_core::protocol::Protocol;

use super::RUNE;

#[derive(Default, Clone)]
pub(super) enum CloneWith {
    /// Use `rune::alloc::clone::TryClone`.
    #[default]
    TryClone,
    /// Use `::alloc::clone::Clone`.
    Clone,
    /// The field is `Copy`.
    Copy,
    /// The field can be cloned with the given function.
    With(syn::Path),
    /// The field can be fallibly cloned with the given function.
    TryWith(syn::Path),
}

impl CloneWith {
    pub(super) fn decorate(&self, tokens: &Tokens, access: TokenStream) -> TokenStream {
        let Tokens {
            try_clone, clone, ..
        } = tokens;

        match self {
            Self::TryClone => {
                quote!(#try_clone::try_clone(#access)?)
            }
            Self::Clone => {
                quote!(#clone::clone(#access))
            }
            Self::Copy => {
                quote!(*#access)
            }
            Self::With(path) => {
                quote!(#path(#access))
            }
            Self::TryWith(path) => {
                quote!(#path(#access)?)
            }
        }
    }
}

/// Parsed `#[rune(..)]` field attributes.
#[derive(Default)]
#[must_use = "Attributes must be used or explicitly ignored"]
pub(crate) struct FieldAttr {
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
    /// Custom parser `#[rune(parse_with)]`.
    pub(crate) parse_with: Option<syn::Path>,
    /// `#[rune(..)]` to generate a protocol function.
    pub(crate) protocols: Vec<FieldProtocol>,
    /// The method to use when cloning the field.
    pub(crate) clone_with: CloneWith,
    /// Whether this field should be known at compile time or not.
    pub(crate) field: bool,
}

impl FieldAttr {
    /// Indicate if the field should be skipped.
    pub(crate) fn skip(&self) -> bool {
        self.skip.is_some() || self.id.is_some()
    }
}

/// Parsed #[const_value(..)] field attributes.
#[derive(Default)]
#[must_use = "Attributes must be used or explicitly ignored"]
pub(crate) struct ConstValueFieldAttrs {
    /// Define a custom parsing method.
    pub(crate) with: Option<syn::Path>,
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

#[derive(Default)]
pub(crate) enum TypeFields {
    #[default]
    Default,
    Empty,
    Unnamed(syn::LitInt),
}

#[derive(Default)]
pub(crate) enum TypeConstructor {
    #[default]
    None,
    /// An implicit constructor using all visible fields.
    Implicit(Span),
    /// Path to an explicit constructor.
    Explicit(syn::Path),
}

impl TypeConstructor {
    /// Get the explicit type constructor, or generate an implicit if specified.
    #[inline]
    pub(crate) fn or_implicit(&self, f: impl FnOnce() -> TokenStream) -> Option<TokenStream> {
        match self {
            Self::None => None,
            Self::Implicit(..) => Some(f()),
            Self::Explicit(path) => Some(quote! { #path }),
        }
    }

    /// Get the explicit type constructor, if any.
    #[inline]
    pub(crate) fn as_explicit(&self) -> Option<TokenStream> {
        match self {
            Self::Explicit(path) => Some(quote! { #path }),
            _ => None,
        }
    }

    /// Get the span of the constructor, if any.
    pub(crate) fn as_span(&self) -> Option<Span> {
        match self {
            Self::None => None,
            Self::Implicit(span) => Some(*span),
            Self::Explicit(path) => Some(path.span()),
        }
    }
}

/// Parsed field attributes.
#[derive(Default)]
#[must_use = "Attributes must be used or explicitly ignored"]
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
    /// `#[rune(fields)]` to suppress default metadata.
    pub(crate) fields: TypeFields,
    /// `#[rune(constructor)]`.
    pub(crate) constructor: TypeConstructor,
    /// Parsed documentation.
    pub(crate) docs: Vec<syn::Expr>,
}

/// Parsed #[const_value(..)] field attributes.
#[derive(Default)]
#[must_use = "Attributes must be used or explicitly ignored"]
pub(crate) struct ConstValueTypeAttr {
    /// `#[const_value(module = <path>)]`.
    pub(crate) module: Option<syn::Path>,
}

/// Parsed variant attributes.
#[derive(Default)]
#[must_use = "Attributes must be used or explicitly ignored"]
pub(crate) struct VariantAttrs {
    /// `#[rune(fields)]` to suppress default metadata.
    pub(crate) fields: TypeFields,
    /// `#[rune(constructor)]`.
    pub(crate) constructor: TypeConstructor,
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
    pub(crate) attr: &'a FieldAttr,
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

    /// Helper to build using a function that takes a context.
    pub(super) fn build(f: impl FnOnce(&Self) -> Result<TokenStream, ()>) -> TokenStream {
        let cx = Self::new();
        cx.build_inner(f)
    }

    /// Helper to build using a function that takes a context internally.
    pub(super) fn build_with_crate(
        f: impl FnOnce(&Self) -> Result<TokenStream, ()>,
    ) -> TokenStream {
        let cx = Self::with_crate();
        cx.build_inner(f)
    }

    fn build_inner(self, f: impl FnOnce(&Self) -> Result<TokenStream, ()>) -> TokenStream {
        fn to_compile_errors<I>(errors: I) -> TokenStream
        where
            I: IntoIterator<Item = syn::Error>,
        {
            let mut stream = TokenStream::default();

            for error in errors {
                stream.extend(error.into_compile_error());
            }

            stream
        }

        let Ok(builder) = f(&self) else {
            return to_compile_errors(self.errors.into_inner());
        };

        let errors = self.errors.into_inner();

        if !errors.is_empty() {
            return to_compile_errors(errors);
        }

        builder
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

    pub(crate) fn const_value_field_attrs(&self, input: &[syn::Attribute]) -> ConstValueFieldAttrs {
        let mut attr = ConstValueFieldAttrs::default();

        for a in input {
            if !a.path().is_ident("const_value") {
                continue;
            }

            let result = a.parse_nested_meta(|meta| {
                if meta.path.is_ident("with") {
                    meta.input.parse::<Token![=]>()?;
                    attr.with = Some(meta.input.parse::<syn::Path>()?);
                    return Ok(());
                }

                Err(syn::Error::new_spanned(
                    &meta.path,
                    "Unsupported field attribute",
                ))
            });

            if let Err(e) = result {
                self.error(e);
            };
        }

        attr
    }

    /// Parse field attributes.
    pub(crate) fn field_attrs(&self, input: &[syn::Attribute]) -> FieldAttr {
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

                    let protocol_field = g.tokens.protocol(&Protocol::$proto);

                    match target {
                        GenerateTarget::Named { field_ident, field_name } => {
                            if let Some(custom) = &protocol.custom {
                                quote_spanned! { field.span() =>
                                    module.field_function(&#protocol_field, #field_name, #custom)?;
                                }
                            } else {
                                quote_spanned! { field.span() =>
                                    module.field_function(&#protocol_field, #field_name, |s: &mut Self, value: #ty| {
                                        s.#field_ident $op value;
                                    })?;
                                }
                            }
                        }
                        GenerateTarget::Numbered { field_index } => {
                            if let Some(custom) = &protocol.custom {
                                quote_spanned! { field.span() =>
                                    module.index_function(&#protocol_field, #field_index, #custom)?;
                                }
                            } else {
                                quote_spanned! { field.span() =>
                                    module.index_function(&#protocol_field, #field_index, |s: &mut Self, value: #ty| {
                                        s.#field_index $op value;
                                    })?;
                                }
                            }
                        }
                    }
                }
            };
        }

        macro_rules! generate {
            ($proto:ident, $op:tt) => {
                |g| {
                    let Generate {
                        ty,
                        target,
                        field,
                        protocol,
                        ..
                    } = g;

                    let protocol_field = g.tokens.protocol(&Protocol::$proto);

                    match target {
                        GenerateTarget::Named { field_ident, field_name } => {
                            if let Some(custom) = &protocol.custom {
                                quote_spanned! { field.span() =>
                                    module.field_function(&#protocol_field, #field_name, #custom)?;
                                }
                            } else {
                                quote_spanned! { field.span() =>
                                    module.field_function(&#protocol_field, #field_name, |s: &mut Self, value: #ty| {
                                        s.#field_ident $op value
                                    })?;
                                }
                            }
                        }
                        GenerateTarget::Numbered { field_index } => {
                            if let Some(custom) = &protocol.custom {
                                quote_spanned! { field.span() =>
                                    module.index_function(&#protocol_field, #field_index, #custom)?;
                                }
                            } else {
                                quote_spanned! { field.span() =>
                                    module.index_function(&#protocol_field, #field_index, |s: &mut Self, value: #ty| {
                                        s.#field_index $op value
                                    })?;
                                }
                            }
                        }
                    }
                }
            };
        }

        let mut attr = FieldAttr::default();

        for a in input {
            if !a.path().is_ident(RUNE) {
                continue;
            }

            let result = a.parse_nested_meta(|meta| {
                macro_rules! field_functions {
                    (
                        $(
                            $assign:literal, $assign_proto:ident, [$($assign_op:tt)*],
                            $op:literal, $op_proto:ident, [$($op_op:tt)*],
                        )*
                    ) => {{
                        $(
                            if meta.path.is_ident($assign) {
                                attr.protocols.push(FieldProtocol {
                                    custom: self.parse_field_custom(meta.input)?,
                                    generate: generate_assign!($assign_proto, $($assign_op)*),
                                });

                                return Ok(());
                            }

                            if meta.path.is_ident($op) {
                                attr.protocols.push(FieldProtocol {
                                    custom: self.parse_field_custom(meta.input)?,
                                    generate: generate!($op_proto, $($op_op)*),
                                });

                                return Ok(());
                            }
                        )*
                    }};
                }

                if meta.path.is_ident("id") {
                    attr.id = Some(meta.path.span());
                    return Ok(());
                }

                if meta.path.is_ident("iter") {
                    attr.iter = Some(meta.path.span());
                    return Ok(());
                }

                if meta.path.is_ident("skip") {
                    attr.skip = Some(meta.path.span());
                    return Ok(());
                }

                if meta.path.is_ident("option") {
                    attr.option = Some(meta.path.span());
                    return Ok(());
                }

                if meta.path.is_ident("meta") {
                    attr.meta = Some(meta.path.span());
                    return Ok(());
                }

                if meta.path.is_ident("span") {
                    attr.span = Some(meta.path.span());
                    return Ok(());
                }

                if meta.path.is_ident("copy") {
                    attr.clone_with = CloneWith::Copy;
                    return Ok(());
                }

                if meta.path.is_ident("clone") {
                    attr.clone_with = CloneWith::Clone;
                    return Ok(());
                }

                if meta.path.is_ident("clone_with") {
                    meta.input.parse::<Token![=]>()?;
                    attr.clone_with = CloneWith::With(meta.input.parse()?);
                    return Ok(());
                }

                if meta.path.is_ident("try_clone_with") {
                    meta.input.parse::<Token![=]>()?;
                    attr.clone_with = CloneWith::TryWith(meta.input.parse()?);
                    return Ok(());
                }

                if meta.path.is_ident("parse_with") {
                    if let Some(old) = &attr.parse_with {
                        let mut error = syn::Error::new_spanned(
                            &meta.path,
                            "#[rune(parse_with)] can only be used once",
                        );

                        error.combine(syn::Error::new_spanned(old, "previously defined here"));
                        return Err(error);
                    }

                    meta.input.parse::<Token![=]>()?;
                    attr.parse_with = Some(meta.input.parse()?);
                    return Ok(());
                }

                if meta.path.is_ident("get") {
                    attr.field = true;
                    attr.protocols.push(FieldProtocol {
                        custom: self.parse_field_custom(meta.input)?,
                        generate: |g| {
                            let Generate {
                                target,
                                ..
                            } = g;

                            let Tokens {
                                result,
                                runtime_error,
                                ..
                            } = g.tokens;

                            match target {
                                GenerateTarget::Named { field_ident, field_name } => {
                                    let access = g.attr.clone_with.decorate(g.tokens, quote!(&s.#field_ident));
                                    let protocol = g.tokens.protocol(&Protocol::GET);

                                    quote_spanned! { g.field.span() =>
                                        module.field_function(&#protocol, #field_name, |s: &Self| #result::<_, #runtime_error>::Ok(#access))?;
                                    }
                                }
                                GenerateTarget::Numbered { field_index } => {
                                    let access = g.attr.clone_with.decorate(g.tokens, quote!(&s.#field_index));
                                    let protocol = g.tokens.protocol(&Protocol::GET);

                                    quote_spanned! { g.field.span() =>
                                        module.index_function(&#protocol, #field_index, |s: &Self| #result::<_, #runtime_error>::Ok(#access))?;
                                    }
                                }
                            }
                        },
                    });

                    return Ok(());
                }

                if meta.path.is_ident("set") {
                    attr.protocols.push(FieldProtocol {
                        custom: self.parse_field_custom(meta.input)?,
                        generate: |g| {
                            let Generate {
                                ty,
                                target,
                                ..
                            } = g;

                            let protocol = g.tokens.protocol(&Protocol::SET);

                            match target {
                                GenerateTarget::Named { field_ident, field_name } => {
                                    quote_spanned! { g.field.span() =>
                                        module.field_function(&#protocol, #field_name, |s: &mut Self, value: #ty| {
                                            s.#field_ident = value;
                                        })?;
                                    }
                                }
                                GenerateTarget::Numbered { field_index } => {
                                    quote_spanned! { g.field.span() =>
                                        module.index_function(&#protocol, #field_index, |s: &mut Self, value: #ty| {
                                            s.#field_index = value;
                                        })?;
                                    }
                                }
                            }
                        },
                    });

                    return Ok(());
                }

                field_functions! {
                    "add_assign", ADD_ASSIGN, [+=], "add", ADD, [+],
                    "sub_assign", SUB_ASSIGN, [-=], "sub", SUB, [-],
                    "div_assign", DIV_ASSIGN, [/=], "div", DIV, [/],
                    "mul_assign", MUL_ASSIGN, [*=], "mul", MUL, [*],
                    "rem_assign", REM_ASSIGN, [%=], "rem", REM, [%],
                    "bit_and_assign", BIT_AND_ASSIGN, [&=], "bit_and", BIT_AND, [&],
                    "bit_or_assign", BIT_OR_ASSIGN, [|=], "bit_or", BIT_OR, [|],
                    "bit_xor_assign", BIT_XOR_ASSIGN, [^=], "bit_xor", BIT_XOR, [^],
                    "shl_assign", SHL_ASSIGN, [<<=], "shl", SHL, [<<],
                    "shr_assign", SHR_ASSIGN, [>>=], "shr", SHR, [>>],
                }

                Err(syn::Error::new_spanned(&meta.path, "Unsupported attribute"))
            });

            if let Err(e) = result {
                self.error(e);
            }
        }

        attr
    }

    pub(crate) fn const_value_type_attrs(&self, input: &[syn::Attribute]) -> ConstValueTypeAttr {
        let mut attr = ConstValueTypeAttr::default();

        for a in input {
            if !a.path().is_ident("const_value") {
                continue;
            }

            let result = a.parse_nested_meta(|meta| {
                if meta.path.is_ident("module") || meta.path.is_ident("crate") {
                    if meta.input.parse::<Option<Token![=]>>()?.is_some() {
                        attr.module = Some(parse_path_compat(meta.input)?);
                    } else {
                        attr.module = Some(syn::parse_quote!(crate));
                    }

                    return Ok(());
                }

                Err(syn::Error::new_spanned(
                    &meta.path,
                    "Unsupported type attribute",
                ))
            });

            if let Err(e) = result {
                self.error(e);
            };
        }

        attr
    }

    /// Parse field attributes.
    pub(crate) fn type_attrs(&self, input: &[syn::Attribute]) -> TypeAttr {
        let mut attr = TypeAttr::default();

        for a in input {
            if a.path().is_ident("doc") {
                if let syn::Meta::NameValue(meta) = &a.meta {
                    attr.docs.push(meta.value.clone());
                }

                continue;
            }

            if !a.path().is_ident(RUNE) {
                continue;
            }

            let result = a.parse_nested_meta(|meta| {
                if meta.path.is_ident("parse") {
                    meta.input.parse::<Token![=]>()?;
                    let s: syn::LitStr = meta.input.parse()?;

                    match s.value().as_str() {
                        "meta_only" => {
                            attr.parse = ParseKind::MetaOnly;
                        }
                        other => {
                            return Err(syn::Error::new(
                                meta.input.span(),
                                format!("Unsupported `#[rune(parse = ..)]` argument `{}`", other),
                            ));
                        }
                    };

                    return Ok(());
                }

                if meta.path.is_ident("item") {
                    meta.input.parse::<Token![=]>()?;
                    attr.item = Some(meta.input.parse()?);
                    return Ok(());
                }

                if meta.path.is_ident("empty") {
                    attr.fields = TypeFields::Empty;
                    return Ok(());
                }

                if meta.path.is_ident("unnamed") {
                    let input;
                    _ = syn::parenthesized!(input in meta.input);
                    attr.fields = TypeFields::Unnamed(input.parse::<syn::LitInt>()?);
                    return Ok(());
                }

                if meta.path.is_ident("name") {
                    meta.input.parse::<Token![=]>()?;
                    attr.name = Some(meta.input.parse()?);
                    return Ok(());
                }

                if meta.path.is_ident("module") || meta.path.is_ident("crate") {
                    if meta.input.parse::<Option<Token![=]>>()?.is_some() {
                        attr.module = Some(parse_path_compat(meta.input)?);
                    } else {
                        attr.module = Some(syn::parse_quote!(crate));
                    }

                    return Ok(());
                }

                if meta.path.is_ident("install_with") {
                    meta.input.parse::<Token![=]>()?;
                    attr.install_with = Some(parse_path_compat(meta.input)?);
                    return Ok(());
                }

                if meta.path.is_ident("constructor") {
                    if let Some(span) = attr.constructor.as_span() {
                        let mut error = syn::Error::new(
                            meta.path.span(),
                            "#[rune(constructor)] must only be used once",
                        );

                        error.combine(syn::Error::new(span, "Previously defined here"));
                        return Err(error);
                    }

                    if meta.input.parse::<Option<Token![=]>>()?.is_some() {
                        attr.constructor = TypeConstructor::Explicit(meta.input.parse()?);
                    } else {
                        attr.constructor = TypeConstructor::Implicit(meta.path.span());
                    }

                    return Ok(());
                }

                Err(syn::Error::new_spanned(
                    &meta.path,
                    "Unsupported type attribute",
                ))
            });

            if let Err(e) = result {
                self.error(e);
            };
        }

        attr
    }

    /// Parse and extract variant attributes.
    pub(crate) fn variant_attr(&self, input: &[syn::Attribute]) -> VariantAttrs {
        let mut attr = VariantAttrs::default();

        for a in input {
            if a.path().is_ident("doc") {
                if let syn::Meta::NameValue(meta) = &a.meta {
                    attr.docs.push(meta.value.clone());
                }

                continue;
            }

            if !a.path().is_ident(RUNE) {
                continue;
            }

            let result = a.parse_nested_meta(|meta| {
                if meta.path.is_ident("empty") {
                    attr.fields = TypeFields::Empty;
                    return Ok(());
                }

                if meta.path.is_ident("unnamed") {
                    let input;
                    _ = syn::parenthesized!(input in meta.input);
                    attr.fields = TypeFields::Unnamed(input.parse::<syn::LitInt>()?);
                    return Ok(());
                }

                if meta.path.is_ident("constructor") {
                    if let Some(span) = attr.constructor.as_span() {
                        let mut error = syn::Error::new(
                            meta.path.span(),
                            "#[rune(constructor)] must only be used once",
                        );

                        error.combine(syn::Error::new(span, "Previously defined here"));
                        return Err(error);
                    }

                    if meta.input.parse::<Option<Token![=]>>()?.is_some() {
                        attr.constructor = TypeConstructor::Explicit(meta.input.parse()?);
                    } else {
                        attr.constructor = TypeConstructor::Implicit(meta.path.span());
                    }

                    return Ok(());
                }

                Err(syn::Error::new_spanned(&meta.path, "Unsupported attribute"))
            });

            if let Err(e) = result {
                self.error(e);
            }
        }

        attr
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
                        .push(syn::PathSegment::from(syn::Ident::new(
                            RUNE,
                            Span::call_site(),
                        )));
                    &default_module
                }
            },
        };

        let core = &core;

        Tokens {
            alloc: path(m, ["alloc"]),
            any_marker_t: path(m, ["__priv", "AnyMarker"]),
            any_t: path(m, ["Any"]),
            any_type_info: path(m, ["__priv", "AnyTypeInfo"]),
            arc: path(m, ["__priv", "Arc"]),
            clone: path(m, ["__priv", "Clone"]),
            compile_error: path(m, ["compile", "Error"]),
            const_construct_t: path(m, ["__priv", "ConstConstruct"]),
            const_value: path(m, ["__priv", "ConstValue"]),
            context_error: path(m, ["compile", "ContextError"]),
            default: path(core, ["default", "Default"]),
            double_ended_iterator: path(core, ["iter", "DoubleEndedIterator"]),
            errors: path(m, ["__priv", "e"]),
            fmt: path(core, ["fmt"]),
            from_const_value_t: path(m, ["__priv", "FromConstValue"]),
            from_value: path(m, ["__priv", "FromValue"]),
            hash: path(m, ["Hash"]),
            id: path(m, ["parse", "Id"]),
            install_with: path(m, ["__priv", "InstallWith"]),
            into_iterator: path(core, ["iter", "IntoIterator"]),
            item: path(m, ["__priv", "Item"]),
            iterator: path(core, ["iter", "Iterator"]),
            macro_context: path(m, ["macros", "MacroContext"]),
            maybe_type_of: path(m, ["__priv", "MaybeTypeOf"]),
            meta: path(m, ["compile", "meta"]),
            module: path(m, ["__priv", "Module"]),
            named: path(m, ["compile", "Named"]),
            non_null: path(core, ["ptr", "NonNull"]),
            object: path(m, ["__priv", "Object"]),
            opaque: path(m, ["parse", "Opaque"]),
            option_spanned: path(m, ["ast", "OptionSpanned"]),
            option: path(core, ["option", "Option"]),
            owned_tuple: path(m, ["__priv", "OwnedTuple"]),
            parse: path(m, ["parse", "Parse"]),
            parser: path(m, ["parse", "Parser"]),
            protocol: path(m, ["__priv", "Protocol"]),
            raw_value_guard: path(m, ["__priv", "RawValueGuard"]),
            result: path(core, ["result", "Result"]),
            runtime_error: path(m, ["__priv", "RuntimeError"]),
            span: path(m, ["ast", "Span"]),
            spanned: path(m, ["ast", "Spanned"]),
            to_const_value_t: path(m, ["__priv", "ToConstValue"]),
            to_tokens: path(m, ["macros", "ToTokens"]),
            to_value: path(m, ["__priv", "ToValue"]),
            token_stream: path(m, ["macros", "TokenStream"]),
            try_clone: path(m, ["alloc", "clone", "TryClone"]),
            try_from: path(core, ["convert", "TryFrom"]),
            tuple: path(m, ["__priv", "Tuple"]),
            type_hash_t: path(m, ["__priv", "TypeHash"]),
            type_name: path(core, ["any", "type_name"]),
            type_of: path(m, ["__priv", "TypeOf"]),
            type_value: path(m, ["__priv", "TypeValue"]),
            unsafe_to_mut: path(m, ["__priv", "UnsafeToMut"]),
            unsafe_to_ref: path(m, ["__priv", "UnsafeToRef"]),
            unsafe_to_value: path(m, ["__priv", "UnsafeToValue"]),
            value_mut_guard: path(m, ["__priv", "ValueMutGuard"]),
            value_ref_guard: path(m, ["__priv", "ValueRefGuard"]),
            value: path(m, ["__priv", "Value"]),
            write: path(core, ["write"]),
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
    pub(crate) alloc: syn::Path,
    pub(crate) any_marker_t: syn::Path,
    pub(crate) any_t: syn::Path,
    pub(crate) any_type_info: syn::Path,
    pub(crate) arc: syn::Path,
    pub(crate) clone: syn::Path,
    pub(crate) compile_error: syn::Path,
    pub(crate) const_construct_t: syn::Path,
    pub(crate) const_value: syn::Path,
    pub(crate) context_error: syn::Path,
    pub(crate) default: syn::Path,
    pub(crate) double_ended_iterator: syn::Path,
    pub(crate) errors: syn::Path,
    pub(crate) fmt: syn::Path,
    pub(crate) from_const_value_t: syn::Path,
    pub(crate) from_value: syn::Path,
    pub(crate) hash: syn::Path,
    pub(crate) id: syn::Path,
    pub(crate) install_with: syn::Path,
    pub(crate) into_iterator: syn::Path,
    pub(crate) item: syn::Path,
    pub(crate) iterator: syn::Path,
    pub(crate) macro_context: syn::Path,
    pub(crate) maybe_type_of: syn::Path,
    pub(crate) meta: syn::Path,
    pub(crate) module: syn::Path,
    pub(crate) named: syn::Path,
    pub(crate) non_null: syn::Path,
    pub(crate) object: syn::Path,
    pub(crate) opaque: syn::Path,
    pub(crate) option_spanned: syn::Path,
    pub(crate) option: syn::Path,
    pub(crate) owned_tuple: syn::Path,
    pub(crate) parse: syn::Path,
    pub(crate) parser: syn::Path,
    pub(crate) protocol: syn::Path,
    pub(crate) raw_value_guard: syn::Path,
    pub(crate) result: syn::Path,
    pub(crate) runtime_error: syn::Path,
    pub(crate) span: syn::Path,
    pub(crate) spanned: syn::Path,
    pub(crate) to_const_value_t: syn::Path,
    pub(crate) to_tokens: syn::Path,
    pub(crate) to_value: syn::Path,
    pub(crate) token_stream: syn::Path,
    pub(crate) try_clone: syn::Path,
    pub(crate) try_from: syn::Path,
    pub(crate) tuple: syn::Path,
    pub(crate) type_hash_t: syn::Path,
    pub(crate) type_name: syn::Path,
    pub(crate) type_of: syn::Path,
    pub(crate) type_value: syn::Path,
    pub(crate) unsafe_to_mut: syn::Path,
    pub(crate) unsafe_to_ref: syn::Path,
    pub(crate) unsafe_to_value: syn::Path,
    pub(crate) value_mut_guard: syn::Path,
    pub(crate) value_ref_guard: syn::Path,
    pub(crate) value: syn::Path,
    pub(crate) write: syn::Path,
}

impl Tokens {
    /// Define a tokenstream for the specified protocol
    pub(crate) fn protocol(&self, sym: &Protocol) -> TokenStream {
        let mut stream = TokenStream::default();
        self.protocol.to_tokens(&mut stream);
        <Token![::]>::default().to_tokens(&mut stream);
        syn::Ident::new(sym.name, Span::call_site()).to_tokens(&mut stream);
        stream
    }
}
