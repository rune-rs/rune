use rune::ast::{CopySource, Delimiter, Kind, NumberSource, StrSource, StringSource, Token};
use rune::macros::MacroContext;
use rune::quote;

use runestick::Span;
use Kind::*;

macro_rules! assert_quote {
    ($ctx:expr, [$($expected:expr),* $(,)?], $quote:expr) => {
        assert_eq!(vec![$(token($expected),)*], $quote.into_token_stream($ctx));
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
    MacroContext::test(|ctx| {
        assert_quote!(ctx, [Amp], quote!(&));
        assert_quote!(ctx, [Abstract], quote!(abstract));
        assert_quote!(ctx, [AlignOf], quote!(alignof));
        assert_quote!(ctx, [Amp], quote!(&));
        assert_quote!(ctx, [AmpAmp], quote!(&&));
        assert_quote!(ctx, [AmpEq], quote!(&=));
        assert_quote!(ctx, [Arrow], quote!(->));
        assert_quote!(ctx, [As], quote!(as));
        assert_quote!(ctx, [Async], quote!(async));
        assert_quote!(ctx, [At], quote!(@));
        assert_quote!(ctx, [Await], quote!(await));
        assert_quote!(ctx, [Bang], quote!(!));
        assert_quote!(ctx, [BangEq], quote!(!=));
        assert_quote!(ctx, [Become], quote!(become));
        assert_quote!(ctx, [Break], quote!(break));
        assert_quote!(ctx, [Caret], quote!(^));
        assert_quote!(ctx, [CaretEq], quote!(^=));
        assert_quote!(ctx, [Colon], quote!(:));
        assert_quote!(ctx, [ColonColon], quote!(::));
        assert_quote!(ctx, [Comma], quote!(,));
        assert_quote!(ctx, [Const], quote!(const));
        assert_quote!(ctx, [Crate], quote!(crate));
        assert_quote!(ctx, [Dash], quote!(-));
        assert_quote!(ctx, [DashEq], quote!(-=));
        assert_quote!(ctx, [Default], quote!(default));
        assert_quote!(ctx, [Div], quote!(/));
        assert_quote!(ctx, [Do], quote!(do));
        assert_quote!(ctx, [Dollar], quote!($));
        assert_quote!(ctx, [Dot], quote!(.));
        assert_quote!(ctx, [DotDot], quote!(..));
        assert_quote!(ctx, [Else], quote!(else));
        assert_quote!(ctx, [Enum], quote!(enum));
        assert_quote!(ctx, [Eq], quote!(=));
        assert_quote!(ctx, [EqEq], quote!(==));
        assert_quote!(ctx, [Extern], quote!(extern));
        assert_quote!(ctx, [False], quote!(false));
        assert_quote!(ctx, [Final], quote!(final));
        assert_quote!(ctx, [Fn], quote!(fn));
        assert_quote!(ctx, [For], quote!(for));
        assert_quote!(ctx, [Gt], quote!(>));
        assert_quote!(ctx, [GtEq], quote!(>=));
        assert_quote!(ctx, [GtGt], quote!(>>));
        assert_quote!(ctx, [GtGtEq], quote!(>>=));
        assert_quote!(ctx, [If], quote!(if));
        assert_quote!(ctx, [Impl], quote!(impl));
        assert_quote!(ctx, [In], quote!(in));
        assert_quote!(ctx, [Is], quote!(is));
        assert_quote!(ctx, [Let], quote!(let));
        assert_quote!(ctx, [Loop], quote!(loop));
        assert_quote!(ctx, [Lt], quote!(<));
        assert_quote!(ctx, [LtEq], quote!(<=));
        assert_quote!(ctx, [LtLt], quote!(<<));
        assert_quote!(ctx, [LtLtEq], quote!(<<=));
        assert_quote!(ctx, [Macro], quote!(macro));
        assert_quote!(ctx, [Match], quote!(match));
        assert_quote!(ctx, [Mod], quote!(mod));
        assert_quote!(ctx, [Move], quote!(move));
        assert_quote!(ctx, [Not], quote!(not));
        assert_quote!(ctx, [OffsetOf], quote!(offsetof));
        assert_quote!(ctx, [Override], quote!(override));
        assert_quote!(ctx, [Perc], quote!(%));
        assert_quote!(ctx, [PercEq], quote!(%=));
        assert_quote!(ctx, [Pipe], quote!(|));
        assert_quote!(ctx, [PipeEq], quote!(|=));
        assert_quote!(ctx, [PipePipe], quote!(||));
        assert_quote!(ctx, [Plus], quote!(+));
        assert_quote!(ctx, [PlusEq], quote!(+=));
        assert_quote!(ctx, [Pound], quote!(#));
        assert_quote!(ctx, [Priv], quote!(priv));
        assert_quote!(ctx, [Proc], quote!(proc));
        assert_quote!(ctx, [Pub], quote!(pub));
        assert_quote!(ctx, [Pure], quote!(pure));
        assert_quote!(ctx, [QuestionMark], quote!(?));
        assert_quote!(ctx, [Ref], quote!(ref));
        assert_quote!(ctx, [Return], quote!(return));
        assert_quote!(ctx, [Rocket], quote!(=>));
        assert_quote!(ctx, [Select], quote!(select));
        assert_quote!(ctx, [SelfType], quote!(Self));
        assert_quote!(ctx, [SelfValue], quote!(self));
        assert_quote!(ctx, [SemiColon], quote!(;));
        assert_quote!(ctx, [SizeOf], quote!(sizeof));
        assert_quote!(ctx, [SlashEq], quote!(/=));
        assert_quote!(ctx, [Star], quote!(*));
        assert_quote!(ctx, [StarEq], quote!(*=));
        assert_quote!(ctx, [Static], quote!(static));
        assert_quote!(ctx, [Struct], quote!(struct));
        assert_quote!(ctx, [Super], quote!(super));
        assert_quote!(ctx, [Tilde], quote!(~));
        assert_quote!(ctx, [True], quote!(true));
        assert_quote!(ctx, [TypeOf], quote!(typeof));
        assert_quote!(ctx, [Underscore], quote!(_));
        assert_quote!(ctx, [Unsafe], quote!(unsafe));
        assert_quote!(ctx, [Use], quote!(use));
        assert_quote!(ctx, [Virtual], quote!(virtual));
        assert_quote!(ctx, [While], quote!(while));
        assert_quote!(ctx, [Yield], quote!(yield));
    });
}

#[test]
fn test_synthetic() {
    MacroContext::test(|ctx| {
        assert_quote!(ctx, [Ident(StringSource::Synthetic(0))], quote!(hello));
        assert_quote!(ctx, [ByteStr(StrSource::Synthetic(0))], quote!(b"hello"));
        assert_quote!(ctx, [Str(StrSource::Synthetic(0))], quote!("hello"));
        assert_quote!(ctx, [Number(NumberSource::Synthetic(0))], quote!(0));
        assert_quote!(ctx, [Number(NumberSource::Synthetic(1))], quote!(42.0));
        assert_quote!(ctx, [Char(CopySource::Inline('a'))], quote!('a'));
        assert_quote!(ctx, [Byte(CopySource::Inline(b'a'))], quote!(b'a'));
    });
}

#[test]
fn test_interpolate() {
    MacroContext::test(|ctx| {
        let outer = quote!(self struct enum);
        assert_quote!(ctx, [SelfValue, Struct, Enum], quote!(#outer));
    });
}

#[test]
fn test_attribute() {
    MacroContext::test(|ctx| {
        assert_quote!(ctx, 
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
    MacroContext::test(|ctx| {
        assert_quote!(ctx, 
            [
                Pound,
                Open(Delimiter::Brace),
                Ident(StringSource::Synthetic(0)),
                Colon,
                Number(NumberSource::Synthetic(0)),
                Close(Delimiter::Brace),
            ],
            quote!(#{test: 42})
        );
    });
}
