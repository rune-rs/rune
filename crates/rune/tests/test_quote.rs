use rune::ast::{CopySource, Delimiter, Kind, LitStrSource, NumberSource, StringSource, Token};
use rune::macros::{with_context, MacroContext};
use rune::quote;

use runestick::Span;
use Kind::*;

macro_rules! assert_quote {
    ([$($expected:expr),* $(,)?], $quote:expr) => {
        assert_eq!(vec![$(token($expected),)*], $quote.into_token_stream());
    }
}

fn token(kind: Kind) -> Token {
    Token {
        span: Span::default(),
        kind,
    }
}

#[test]
fn test_tokens() {
    let ctx = MacroContext::empty();

    with_context(ctx, || {
        assert_quote!([Amp], quote!(&));
        assert_quote!([Abstract], quote!(abstract));
        assert_quote!([AlignOf], quote!(alignof));
        assert_quote!([Amp], quote!(&));
        assert_quote!([AmpAmp], quote!(&&));
        assert_quote!([AmpEq], quote!(&=));
        assert_quote!([Arrow], quote!(->));
        assert_quote!([As], quote!(as));
        assert_quote!([Async], quote!(async));
        assert_quote!([At], quote!(@));
        assert_quote!([Await], quote!(await));
        assert_quote!([Bang], quote!(!));
        assert_quote!([BangEq], quote!(!=));
        assert_quote!([Become], quote!(become));
        assert_quote!([Break], quote!(break));
        assert_quote!([Caret], quote!(^));
        assert_quote!([CaretEq], quote!(^=));
        assert_quote!([Colon], quote!(:));
        assert_quote!([ColonColon], quote!(::));
        assert_quote!([Comma], quote!(,));
        assert_quote!([Const], quote!(const));
        assert_quote!([Crate], quote!(crate));
        assert_quote!([Dash], quote!(-));
        assert_quote!([DashEq], quote!(-=));
        assert_quote!([Default], quote!(default));
        assert_quote!([Div], quote!(/));
        assert_quote!([Do], quote!(do));
        assert_quote!([Dollar], quote!($));
        assert_quote!([Dot], quote!(.));
        assert_quote!([DotDot], quote!(..));
        assert_quote!([Else], quote!(else));
        assert_quote!([Enum], quote!(enum));
        assert_quote!([Eq], quote!(=));
        assert_quote!([EqEq], quote!(==));
        assert_quote!([Extern], quote!(extern));
        assert_quote!([False], quote!(false));
        assert_quote!([Final], quote!(final));
        assert_quote!([Fn], quote!(fn));
        assert_quote!([For], quote!(for));
        assert_quote!([Gt], quote!(>));
        assert_quote!([GtEq], quote!(>=));
        assert_quote!([GtGt], quote!(>>));
        assert_quote!([GtGtEq], quote!(>>=));
        assert_quote!([If], quote!(if));
        assert_quote!([Impl], quote!(impl));
        assert_quote!([In], quote!(in));
        assert_quote!([Is], quote!(is));
        assert_quote!([Let], quote!(let));
        assert_quote!([Loop], quote!(loop));
        assert_quote!([Lt], quote!(<));
        assert_quote!([LtEq], quote!(<=));
        assert_quote!([LtLt], quote!(<<));
        assert_quote!([LtLtEq], quote!(<<=));
        assert_quote!([Macro], quote!(macro));
        assert_quote!([Match], quote!(match));
        assert_quote!([Mod], quote!(mod));
        assert_quote!([Move], quote!(move));
        assert_quote!([Not], quote!(not));
        assert_quote!([OffsetOf], quote!(offsetof));
        assert_quote!([Override], quote!(override));
        assert_quote!([Perc], quote!(%));
        assert_quote!([PercEq], quote!(%=));
        assert_quote!([Pipe], quote!(|));
        assert_quote!([PipeEq], quote!(|=));
        assert_quote!([PipePipe], quote!(||));
        assert_quote!([Plus], quote!(+));
        assert_quote!([PlusEq], quote!(+=));
        assert_quote!([Pound], quote!(#));
        assert_quote!([Priv], quote!(priv));
        assert_quote!([Proc], quote!(proc));
        assert_quote!([Pub], quote!(pub));
        assert_quote!([Pure], quote!(pure));
        assert_quote!([QuestionMark], quote!(?));
        assert_quote!([Ref], quote!(ref));
        assert_quote!([Return], quote!(return));
        assert_quote!([Rocket], quote!(=>));
        assert_quote!([Select], quote!(select));
        assert_quote!([SelfType], quote!(Self));
        assert_quote!([SelfValue], quote!(self));
        assert_quote!([SemiColon], quote!(;));
        assert_quote!([SizeOf], quote!(sizeof));
        assert_quote!([SlashEq], quote!(/=));
        assert_quote!([Star], quote!(*));
        assert_quote!([StarEq], quote!(*=));
        assert_quote!([Static], quote!(static));
        assert_quote!([Struct], quote!(struct));
        assert_quote!([Super], quote!(super));
        assert_quote!([Template], quote!(template));
        assert_quote!([Tilde], quote!(~));
        assert_quote!([True], quote!(true));
        assert_quote!([TypeOf], quote!(typeof));
        assert_quote!([Underscore], quote!(_));
        assert_quote!([Unsafe], quote!(unsafe));
        assert_quote!([Use], quote!(use));
        assert_quote!([Virtual], quote!(virtual));
        assert_quote!([While], quote!(while));
        assert_quote!([Yield], quote!(yield));
    });
}

#[test]
fn test_synthetic() {
    let ctx = MacroContext::empty();

    with_context(ctx, || {
        assert_quote!([Ident(StringSource::Synthetic(0))], quote!(hello));
        assert_quote!([LitByteStr(LitStrSource::Synthetic(0))], quote!(b"hello"));
        assert_quote!([LitStr(LitStrSource::Synthetic(0))], quote!("hello"));
        assert_quote!([LitNumber(NumberSource::Synthetic(0))], quote!(0));
        assert_quote!([LitNumber(NumberSource::Synthetic(1))], quote!(42.0));
        assert_quote!([LitChar(CopySource::Inline('a'))], quote!('a'));
        assert_quote!([LitByte(CopySource::Inline(b'a'))], quote!(b'a'));
    });
}

#[test]
fn test_interpolate() {
    let ctx = MacroContext::empty();

    with_context(ctx, || {
        let outer = quote!(self struct enum);
        assert_quote!([SelfValue, Struct, Enum], quote!(#outer));
    });
}

#[test]
fn test_attribute() {
    let ctx = MacroContext::empty();

    with_context(ctx, || {
        assert_quote!(
            [
                Pound,
                Open(Delimiter::Bracket),
                Ident(StringSource::Synthetic(0)),
                Close(Delimiter::Bracket),
            ],
            quote!(#[test])
        );
    });
}

#[test]
fn test_object() {
    let ctx = MacroContext::empty();

    with_context(ctx, || {
        assert_quote!(
            [
                Pound,
                Open(Delimiter::Brace),
                Ident(StringSource::Synthetic(0)),
                Colon,
                LitNumber(NumberSource::Synthetic(0)),
                Close(Delimiter::Brace),
            ],
            quote!(#{test: 42})
        );
    });
}
