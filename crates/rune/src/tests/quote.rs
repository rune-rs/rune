prelude!();

use ast::Kind::*;
use ast::{CopySource, Delimiter, LitSource, NumberSource, StrSource};
use macros::quote;

macro_rules! assert_quote {
    ($cx:expr, [$($expected:pat),* $(,)?], $quote:expr) => {
        let ts = $quote.into_token_stream($cx).unwrap();
        let mut it = ts.into_iter();

        $(
            let tok = it.next().expect("expected produced token");
            assert_matches!(tok.kind, $expected);
        )*

        assert!(it.next().is_none(), "got extra tokens");
    }
}

#[test]
fn test_tokens() -> Result<()> {
    macros::test(|cx| {
        assert_quote!(cx, [Amp], quote!(&));
        assert_quote!(cx, [Abstract], quote!(abstract));
        assert_quote!(cx, [AlignOf], quote!(alignof));
        assert_quote!(cx, [Amp], quote!(&));
        assert_quote!(cx, [AmpAmp], quote!(&&));
        assert_quote!(cx, [AmpEq], quote!(&=));
        assert_quote!(cx, [Arrow], quote!(->));
        assert_quote!(cx, [As], quote!(as));
        assert_quote!(cx, [Async], quote!(async));
        assert_quote!(cx, [At], quote!(@));
        assert_quote!(cx, [Await], quote!(await));
        assert_quote!(cx, [Bang], quote!(!));
        assert_quote!(cx, [BangEq], quote!(!=));
        assert_quote!(cx, [Become], quote!(become));
        assert_quote!(cx, [Break], quote!(break));
        assert_quote!(cx, [Caret], quote!(^));
        assert_quote!(cx, [CaretEq], quote!(^=));
        assert_quote!(cx, [Colon], quote!(:));
        assert_quote!(cx, [ColonColon], quote!(::));
        assert_quote!(cx, [Comma], quote!(,));
        assert_quote!(cx, [Const], quote!(const));
        assert_quote!(cx, [Crate], quote!(crate));
        assert_quote!(cx, [Dash], quote!(-));
        assert_quote!(cx, [DashEq], quote!(-=));
        assert_quote!(cx, [Default], quote!(default));
        assert_quote!(cx, [Div], quote!(/));
        assert_quote!(cx, [Do], quote!(do));
        assert_quote!(cx, [Dollar], quote!($));
        assert_quote!(cx, [Dot], quote!(.));
        assert_quote!(cx, [DotDot], quote!(..));
        assert_quote!(cx, [Else], quote!(else));
        assert_quote!(cx, [Enum], quote!(enum));
        assert_quote!(cx, [Eq], quote!(=));
        assert_quote!(cx, [EqEq], quote!(==));
        assert_quote!(cx, [Extern], quote!(extern));
        assert_quote!(cx, [False], quote!(false));
        assert_quote!(cx, [Final], quote!(final));
        assert_quote!(cx, [Fn], quote!(fn));
        assert_quote!(cx, [For], quote!(for));
        assert_quote!(cx, [Gt], quote!(>));
        assert_quote!(cx, [GtEq], quote!(>=));
        assert_quote!(cx, [GtGt], quote!(>>));
        assert_quote!(cx, [GtGtEq], quote!(>>=));
        assert_quote!(cx, [If], quote!(if));
        assert_quote!(cx, [Impl], quote!(impl));
        assert_quote!(cx, [In], quote!(in));
        assert_quote!(cx, [Is], quote!(is));
        assert_quote!(cx, [Let], quote!(let));
        assert_quote!(cx, [Loop], quote!(loop));
        assert_quote!(cx, [Lt], quote!(<));
        assert_quote!(cx, [LtEq], quote!(<=));
        assert_quote!(cx, [LtLt], quote!(<<));
        assert_quote!(cx, [LtLtEq], quote!(<<=));
        assert_quote!(cx, [Macro], quote!(macro));
        assert_quote!(cx, [Match], quote!(match));
        assert_quote!(cx, [Mod], quote!(mod));
        assert_quote!(cx, [Move], quote!(move));
        assert_quote!(cx, [Not], quote!(not));
        assert_quote!(cx, [OffsetOf], quote!(offsetof));
        assert_quote!(cx, [Override], quote!(override));
        assert_quote!(cx, [Perc], quote!(%));
        assert_quote!(cx, [PercEq], quote!(%=));
        assert_quote!(cx, [Pipe], quote!(|));
        assert_quote!(cx, [PipeEq], quote!(|=));
        assert_quote!(cx, [PipePipe], quote!(||));
        assert_quote!(cx, [Plus], quote!(+));
        assert_quote!(cx, [PlusEq], quote!(+=));
        assert_quote!(cx, [Pound], quote!(#));
        assert_quote!(cx, [Priv], quote!(priv));
        assert_quote!(cx, [Proc], quote!(proc));
        assert_quote!(cx, [Pub], quote!(pub));
        assert_quote!(cx, [Pure], quote!(pure));
        assert_quote!(cx, [QuestionMark], quote!(?));
        assert_quote!(cx, [Ref], quote!(ref));
        assert_quote!(cx, [Return], quote!(return));
        assert_quote!(cx, [Rocket], quote!(=>));
        assert_quote!(cx, [Select], quote!(select));
        assert_quote!(cx, [SelfType], quote!(Self));
        assert_quote!(cx, [SelfValue], quote!(self));
        assert_quote!(cx, [SemiColon], quote!(;));
        assert_quote!(cx, [SizeOf], quote!(sizeof));
        assert_quote!(cx, [SlashEq], quote!(/=));
        assert_quote!(cx, [Star], quote!(*));
        assert_quote!(cx, [StarEq], quote!(*=));
        assert_quote!(cx, [Static], quote!(static));
        assert_quote!(cx, [Struct], quote!(struct));
        assert_quote!(cx, [Super], quote!(super));
        assert_quote!(cx, [Tilde], quote!(~));
        assert_quote!(cx, [True], quote!(true));
        assert_quote!(cx, [TypeOf], quote!(typeof));
        assert_quote!(cx, [Underscore], quote!(_));
        assert_quote!(cx, [Unsafe], quote!(unsafe));
        assert_quote!(cx, [Use], quote!(use));
        assert_quote!(cx, [Virtual], quote!(virtual));
        assert_quote!(cx, [While], quote!(while));
        assert_quote!(cx, [Yield], quote!(yield));
        Ok(())
    })?;

    Ok(())
}

#[test]
fn test_synthetic() -> Result<()> {
    macros::test(|cx| {
        assert_quote!(cx, [Ident(LitSource::Synthetic(..))], quote!(hello));
        assert_quote!(cx, [ByteStr(StrSource::Synthetic(..))], quote!(b"hello"));
        assert_quote!(cx, [Str(StrSource::Synthetic(..))], quote!("hello"));
        assert_quote!(cx, [Number(NumberSource::Synthetic(..))], quote!(0));
        assert_quote!(cx, [Number(NumberSource::Synthetic(..))], quote!(42.0));
        assert_quote!(cx, [Char(CopySource::Inline('a'))], quote!('a'));
        assert_quote!(cx, [Byte(CopySource::Inline(b'a'))], quote!(b'a'));
        Ok(())
    })?;

    Ok(())
}

#[test]
fn test_interpolate() -> Result<()> {
    macros::test(|cx| {
        let outer = quote!(self struct enum);
        assert_quote!(cx, [SelfValue, Struct, Enum], quote!(#outer));
        Ok(())
    })?;

    Ok(())
}

#[test]
fn test_attribute() -> Result<()> {
    macros::test(|cx| {
        assert_quote!(
            cx,
            [
                Pound,
                Open(Delimiter::Bracket),
                Ident(LitSource::Synthetic(..)),
                Close(Delimiter::Bracket),
            ],
            quote!(#[test])
        );

        Ok(())
    })?;

    Ok(())
}

#[test]
fn test_object() -> Result<()> {
    macros::test(|cx| {
        assert_quote!(
            cx,
            [
                Pound,
                Open(Delimiter::Brace),
                Ident(LitSource::Synthetic(..)),
                Colon,
                Number(NumberSource::Synthetic(..)),
                Close(Delimiter::Brace),
            ],
            quote!(#{test: 42})
        );

        Ok(())
    })?;

    Ok(())
}
