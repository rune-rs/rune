use proc_macro2 as p;

pub(crate) const S: Punct = Punct::new("::");
pub(crate) const RUNE: &str = "rune";
pub(crate) const MACROS: RuneModule = RuneModule("macros");
pub(crate) const AST: RuneModule = RuneModule("ast");

pub(crate) trait ToTokens {
    fn to_tokens(self, stream: &mut p::TokenStream, span: p::Span);
}

impl ToTokens for &'static str {
    fn to_tokens(self, stream: &mut p::TokenStream, span: p::Span) {
        stream.extend(Some(p::TokenTree::Ident(p::Ident::new(self, span))))
    }
}

impl ToTokens for char {
    fn to_tokens(self, stream: &mut p::TokenStream, span: p::Span) {
        let mut p = p::Punct::new(self, p::Spacing::Alone);
        p.set_span(span);
        stream.extend(Some(p::TokenTree::Punct(p)));
    }
}

impl ToTokens for p::Literal {
    fn to_tokens(mut self, stream: &mut p::TokenStream, span: p::Span) {
        self.set_span(span);
        stream.extend(Some(p::TokenTree::Literal(self)));
    }
}

macro_rules! impl_tuple {
    () => {};

    ($f_ident:ident $f_var:ident, $($ident:ident $var:ident),* $(,)?) => {
        impl<$f_ident, $( $ident,)*> ToTokens for ($f_ident, $($ident,)*)
        where
            $f_ident: ToTokens,
            $($ident: ToTokens,)*
        {
            fn to_tokens(self, stream: &mut p::TokenStream, span: p::Span) {
                let ($f_var, $($var,)*) = self;
                $f_var.to_tokens(stream, span);
                $($var.to_tokens(stream, span);)*
            }
        }

        impl_tuple!($($ident $var,)*);
    }
}

impl ToTokens for () {
    fn to_tokens(self, _: &mut p::TokenStream, _: p::Span) {}
}

impl_tuple!(A a, B b, C c, D d, E e, F f, G g, H h);

impl ToTokens for p::Ident {
    fn to_tokens(self, stream: &mut p::TokenStream, _: p::Span) {
        stream.extend(std::iter::once(p::TokenTree::Ident(self)));
    }
}

impl ToTokens for p::TokenStream {
    fn to_tokens(self, stream: &mut p::TokenStream, _: p::Span) {
        stream.extend(self);
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RuneModule(&'static str);

impl ToTokens for RuneModule {
    fn to_tokens(self, stream: &mut p::TokenStream, span: p::Span) {
        (RUNE, S, self.0).to_tokens(stream, span);
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ToTokensFn;

impl ToTokens for ToTokensFn {
    fn to_tokens(self, stream: &mut p::TokenStream, span: p::Span) {
        (MACROS, S, "ToTokens", S, "to_tokens").to_tokens(stream, span);
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Kind(pub(crate) &'static str);

impl ToTokens for Kind {
    fn to_tokens(self, stream: &mut p::TokenStream, span: p::Span) {
        (AST, S, "Kind", S, self.0).to_tokens(stream, span);
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Delimiter(pub(crate) &'static str);

impl Delimiter {
    /// Convert from a proc macro.
    pub(crate) fn from_proc_macro(d: p::Delimiter) -> Option<Self> {
        match d {
            p::Delimiter::Parenthesis => Some(Delimiter("Parenthesis")),
            p::Delimiter::Brace => Some(Delimiter("Brace")),
            p::Delimiter::Bracket => Some(Delimiter("Bracket")),
            p::Delimiter::None => None,
        }
    }
}

impl ToTokens for Delimiter {
    fn to_tokens(self, stream: &mut p::TokenStream, span: p::Span) {
        (AST, S, "Delimiter", S).to_tokens(stream, span);
        self.0.to_tokens(stream, span);
    }
}

/// Construct a joined punctuation out of the given string.
#[derive(Clone, Copy)]
pub(crate) struct Punct(&'static str, p::Spacing);

impl Punct {
    pub(crate) const fn new(s: &'static str) -> Punct {
        Punct(s, p::Spacing::Alone)
    }
}

impl ToTokens for Punct {
    fn to_tokens(self, stream: &mut proc_macro2::TokenStream, span: p::Span) {
        let mut it = self.0.chars();
        let last = it.next_back();

        for c in it {
            let mut p = p::Punct::new(c, p::Spacing::Joint);
            p.set_span(span);
            stream.extend(Some(p::TokenTree::Punct(p)));
        }

        if let Some(c) = last {
            let mut p = p::Punct::new(c, self.1);
            p.set_span(span);
            stream.extend(Some(p::TokenTree::Punct(p)));
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Group<T>(p::Delimiter, T);

/// `(T)`.
pub(crate) fn p<T>(inner: T) -> Group<T> {
    Group(p::Delimiter::Parenthesis, inner)
}

/// `{T}`.
pub(crate) fn braced<T>(inner: T) -> Group<T> {
    Group(p::Delimiter::Brace, inner)
}

impl<T> ToTokens for Group<T>
where
    T: ToTokens,
{
    fn to_tokens(self, stream: &mut p::TokenStream, span: p::Span) {
        let mut inner = p::TokenStream::new();
        self.1.to_tokens(&mut inner, span);

        let mut group = p::Group::new(self.0, inner);
        group.set_span(span);
        stream.extend(Some(p::TokenTree::Group(group)));
    }
}

/// An identifier constructor.
pub(crate) struct NewIdent<'a>(pub(crate) &'a str);

impl<'a> ToTokens for NewIdent<'a> {
    fn to_tokens(self, stream: &mut p::TokenStream, span: p::Span) {
        (AST, S, "Ident", S, "new", p(p::Literal::string(self.0))).to_tokens(stream, span);
    }
}

/// An identifier constructor.
pub(crate) struct NewLit(pub(crate) p::Literal);

impl ToTokens for NewLit {
    fn to_tokens(self, stream: &mut p::TokenStream, span: p::Span) {
        (AST, S, "Lit", S, "new", p(self.0)).to_tokens(stream, span);
    }
}
