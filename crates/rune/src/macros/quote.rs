/// Macro helper function for quoting the token stream as macro output.
///
/// Is capable of quoting everything in Rune, except for the following:
/// * Pound signs (`#`), which can be inserted by using `#(ast::Pound)` instead.
///   These are used in object literal, and are a limitiation in the quote macro
///   because we use the pound sign to delimit variables.
/// * Labels, which must be created using [crate::MacroContext::label].
/// * Template strings, which must be created using [crate::MacroContext::template_string].
///
/// # Panics
///
/// Calling this macro will panic if called outside of a macro context.
/// A macro context can be setup using [with_context](crate::macros::with_context).
///
/// ```rust
/// use rune::macros::{with_context, MacroContext};
/// let ctx = MacroContext::empty();
///
/// with_context(ctx, || {
///     rune::quote!(hello self);
/// });
/// ```
///
/// # Interpolating values
///
/// Values are interpolated with `#value`, or `#(value + 1)` for expressions.
///
/// # Iterators
///
/// Anything that can be used as an iterator can be iterated over with
/// `#(iter)*`. A token can also be used to join inbetween each iteration, like
/// `#(iter),*`.
#[macro_export]
macro_rules! quote {
    ($($tt:tt)*) => {{
        let mut stream = $crate::macros::TokenStream::empty();

        {
            let stream = &mut stream;
            $crate::__quote_inner!(@push stream => $($tt)*);
        }

        stream
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __quote_inner {
    (@wrap $s:expr, $variant:ident => $($tt:tt)*) => {{
        $crate::macros::to_tokens(&$crate::ast::Kind::Open($crate::ast::Delimiter::$variant), $s);
        $crate::__quote_inner!(@push $s => $($tt)*);
        $crate::macros::to_tokens(&$crate::ast::Kind::Close($crate::ast::Delimiter::$variant), $s);
    }};

    (@token $s:expr, $variant:ident => $($tt:tt)*) => {{
        $crate::macros::to_tokens(&$crate::ast::Kind::$variant, $s);
        $crate::__quote_inner!(@push $s => $($tt)*);
    }};

    (@push $s:expr => #$var:ident $($tt:tt)*) => {{
        $crate::macros::to_tokens(&$var, $s);
        $crate::__quote_inner!(@push $s => $($tt)*);
    }};

    (@push $s:expr => #($expr:expr) $repeat:tt * $($tt:tt)*) => {{
        let mut it = std::iter::IntoIterator::into_iter($expr).peekable();

        while let Some(v) = it.next() {
            $crate::macros::to_tokens(&v, $s);

            if it.peek().is_some() {
                $crate::__quote_inner!(@push $s => $repeat);
            }
        }

        $crate::__quote_inner!(@push $s => $($tt)*);
    }};

    (@push $s:expr => #($expr:expr)* $($tt:tt)*) => {{
        for v in $expr {
            $crate::macros::to_tokens(&v, $s);
        }

        $crate::__quote_inner!(@push $s => $($tt)*);
    }};

    (@push $s:expr => _ $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Underscore => $($tt)*); };
    (@push $s:expr => - $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Dash => $($tt)*); };
    (@push $s:expr => -= $($tt:tt)*) => { $crate::__quote_inner!(@token $s, DashEq => $($tt)*); };
    (@push $s:expr => -> $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Arrow => $($tt)*); };
    (@push $s:expr => , $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Comma => $($tt)*); };
    (@push $s:expr => ; $($tt:tt)*) => { $crate::__quote_inner!(@token $s, SemiColon => $($tt)*); };
    (@push $s:expr => : $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Colon => $($tt)*); };
    (@push $s:expr => :: $($tt:tt)*) => { $crate::__quote_inner!(@token $s, ColonColon => $($tt)*); };
    (@push $s:expr => ! $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Bang => $($tt)*); };
    (@push $s:expr => != $($tt:tt)*) => { $crate::__quote_inner!(@token $s, BangEq => $($tt)*); };
    (@push $s:expr => ? $($tt:tt)*) => { $crate::__quote_inner!(@token $s, QuestionMark => $($tt)*); };
    (@push $s:expr => . $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Dot => $($tt)*); };
    (@push $s:expr => .. $($tt:tt)*) => { $crate::__quote_inner!(@token $s, DotDot => $($tt)*); };
    (@push $s:expr => .. $($tt:tt)*) => { $crate::__quote_inner!(@token $s, DotDot => $($tt)*); };
    (@push $s:expr => * $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Star => $($tt)*); };
    (@push $s:expr => *= $($tt:tt)*) => { $crate::__quote_inner!(@token $s, StarEq => $($tt)*); };
    (@push $s:expr => / $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Div => $($tt)*); };
    (@push $s:expr => /= $($tt:tt)*) => { $crate::__quote_inner!(@token $s, SlashEq => $($tt)*); };
    (@push $s:expr => & $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Amp => $($tt)*); };
    (@push $s:expr => && $($tt:tt)*) => { $crate::__quote_inner!(@token $s, AmpAmp => $($tt)*); };
    (@push $s:expr => &= $($tt:tt)*) => { $crate::__quote_inner!(@token $s, AmpEq => $($tt)*); };
    (@push $s:expr => # $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Pound => $($tt)*); };
    (@push $s:expr => % $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Perc => $($tt)*); };
    (@push $s:expr => %= $($tt:tt)*) => { $crate::__quote_inner!(@token $s, PercEq => $($tt)*); };
    (@push $s:expr => %= $($tt:tt)*) => { $crate::__quote_inner!(@token $s, PercEq => $($tt)*); };
    (@push $s:expr => ^ $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Caret => $($tt)*); };
    (@push $s:expr => ^= $($tt:tt)*) => { $crate::__quote_inner!(@token $s, CaretEq => $($tt)*); };
    (@push $s:expr => + $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Plus => $($tt)*); };
    (@push $s:expr => += $($tt:tt)*) => { $crate::__quote_inner!(@token $s, PlusEq => $($tt)*); };
    (@push $s:expr => < $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Lt => $($tt)*); };
    (@push $s:expr => << $($tt:tt)*) => { $crate::__quote_inner!(@token $s, LtLt => $($tt)*); };
    (@push $s:expr => <<= $($tt:tt)*) => { $crate::__quote_inner!(@token $s, LtLtEq => $($tt)*);};
    (@push $s:expr => <= $($tt:tt)*) => { $crate::__quote_inner!(@token $s, LtEq => $($tt)*); };
    (@push $s:expr => = $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Eq => $($tt)*); };
    (@push $s:expr => == $($tt:tt)*) => { $crate::__quote_inner!(@token $s, EqEq => $($tt)*); };
    (@push $s:expr => => $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Rocket => $($tt)*); };
    (@push $s:expr => > $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Gt => $($tt)*); };
    (@push $s:expr => >= $($tt:tt)*) => { $crate::__quote_inner!(@token $s, GtEq => $($tt)*); };
    (@push $s:expr => >> $($tt:tt)*) => { $crate::__quote_inner!(@token $s, GtGt => $($tt)*); };
    (@push $s:expr => >>= $($tt:tt)*) => { $crate::__quote_inner!(@token $s, GtGtEq => $($tt)*);};
    (@push $s:expr => | $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Pipe => $($tt)*); };
    (@push $s:expr => |= $($tt:tt)*) => { $crate::__quote_inner!(@token $s, PipeEq => $($tt)*); };
    (@push $s:expr => || $($tt:tt)*) => { $crate::__quote_inner!(@token $s, PipePipe => $($tt)*); };
    (@push $s:expr => ~ $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Tilde => $($tt)*); };
    (@push $s:expr => abstract $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Abstract => $($tt)*); };
    (@push $s:expr => alignof $($tt:tt)*) => { $crate::__quote_inner!(@token $s, AlignOf => $($tt)*); };
    (@push $s:expr => as $($tt:tt)*) => { $crate::__quote_inner!(@token $s, As => $($tt)*); };
    (@push $s:expr => async $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Async => $($tt)*); };
    (@push $s:expr => at $($tt:tt)*) => { $crate::__quote_inner!(@token $s, At => $($tt)*); };
    (@push $s:expr => await $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Await => $($tt)*); };
    (@push $s:expr => become $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Become => $($tt)*); };
    (@push $s:expr => break $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Break => $($tt)*); };
    (@push $s:expr => const $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Const => $($tt)*); };
    (@push $s:expr => crate $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Crate => $($tt)*); };
    (@push $s:expr => default $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Default => $($tt)*); };
    (@push $s:expr => do $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Do => $($tt)*); };
    (@push $s:expr => else $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Else => $($tt)*); };
    (@push $s:expr => enum $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Enum => $($tt)*); };
    (@push $s:expr => extern $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Extern => $($tt)*); };
    (@push $s:expr => false $($tt:tt)*) => { $crate::__quote_inner!(@token $s, False => $($tt)*); };
    (@push $s:expr => final $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Final => $($tt)*); };
    (@push $s:expr => fn $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Fn => $($tt)*); };
    (@push $s:expr => for $($tt:tt)*) => { $crate::__quote_inner!(@token $s, For => $($tt)*); };
    (@push $s:expr => if $($tt:tt)*) => { $crate::__quote_inner!(@token $s, If => $($tt)*); };
    (@push $s:expr => impl $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Impl => $($tt)*); };
    (@push $s:expr => in $($tt:tt)*) => { $crate::__quote_inner!(@token $s, In => $($tt)*); };
    (@push $s:expr => is $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Is => $($tt)*); };
    (@push $s:expr => let $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Let => $($tt)*); };
    (@push $s:expr => loop $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Loop => $($tt)*); };
    (@push $s:expr => macro $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Macro => $($tt)*); };
    (@push $s:expr => match $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Match => $($tt)*); };
    (@push $s:expr => mod $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Mod => $($tt)*); };
    (@push $s:expr => move $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Move => $($tt)*); };
    (@push $s:expr => not $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Not => $($tt)*); };
    (@push $s:expr => offsetof $($tt:tt)*) => { $crate::__quote_inner!(@token $s, OffsetOf => $($tt)*); };
    (@push $s:expr => override $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Override => $($tt)*); };
    (@push $s:expr => priv $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Priv => $($tt)*); };
    (@push $s:expr => proc $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Proc => $($tt)*); };
    (@push $s:expr => pub $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Pub => $($tt)*); };
    (@push $s:expr => pure $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Pure => $($tt)*); };
    (@push $s:expr => ref $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Ref => $($tt)*); };
    (@push $s:expr => return $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Return => $($tt)*); };
    (@push $s:expr => select $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Select => $($tt)*); };
    (@push $s:expr => Self $($tt:tt)*) => { $crate::__quote_inner!(@token $s, SelfType => $($tt)*); };
    (@push $s:expr => self $($tt:tt)*) => { $crate::__quote_inner!(@token $s, SelfValue => $($tt)*); };
    (@push $s:expr => sizeof $($tt:tt)*) => { $crate::__quote_inner!(@token $s, SizeOf => $($tt)*); };
    (@push $s:expr => static $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Static => $($tt)*); };
    (@push $s:expr => struct $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Struct => $($tt)*); };
    (@push $s:expr => super $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Super => $($tt)*); };
    (@push $s:expr => template $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Template => $($tt)*); };
    (@push $s:expr => true $($tt:tt)*) => { $crate::__quote_inner!(@token $s, True => $($tt)*); };
    (@push $s:expr => typeof $($tt:tt)*) => { $crate::__quote_inner!(@token $s, TypeOf => $($tt)*); };
    (@push $s:expr => unsafe $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Unsafe => $($tt)*); };
    (@push $s:expr => use $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Use => $($tt)*); };
    (@push $s:expr => virtual $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Virtual => $($tt)*); };
    (@push $s:expr => while $($tt:tt)*) => { $crate::__quote_inner!(@token $s, While => $($tt)*); };
    (@push $s:expr => yield $($tt:tt)*) => { $crate::__quote_inner!(@token $s, Yield => $($tt)*); };

    (@push $s:expr => { $($tt:tt)* } $($rest:tt)*) => {{
        $crate::__quote_inner!(@wrap $s, Brace => $($tt)*);
        $crate::__quote_inner!(@push $s => $($rest)*);
    }};

    (@push $s:expr => [ $($tt:tt)* ] $($rest:tt)*) => {{
        $crate::__quote_inner!(@wrap $s, Bracket => $($tt)*);
        $crate::__quote_inner!(@push $s => $($rest)*);
    }};

    (@push $s:expr => ( $($tt:tt)* ) $($rest:tt)*) => {{
        $crate::__quote_inner!(@wrap $s, Parenthesis => $($tt)*);
        $crate::__quote_inner!(@push $s => $($rest)*);
    }};

    (@push $s:expr => ( $($tt:tt)* ) $($rest:tt)*) => {{
        $crate::__quote_inner!(@wrap $s, Parenthesis => $($tt)*);
        $crate::__quote_inner!(@push $s => $($rest)*);
    }};

    (@push $s:expr => $ident:ident $($tt:tt)*) => {{
        let kind = $crate::ast::Ident::new(stringify!($ident));
        $crate::macros::to_tokens(&kind, $s);
        $crate::__quote_inner!(@push $s => $($tt)*);
    }};

    (@push $s:expr => $lit:literal $($tt:tt)*) => {{
        let token = $crate::ast::Lit::new($lit);
        $crate::macros::to_tokens(&token, $s);
        $crate::__quote_inner!(@push $s => $($tt)*);
    }};

    (@push $s:expr =>) => {};
}

#[cfg(test)]
mod tests {
    use crate::ast::{CopySource, Kind, LitStrSource, NumberSource, StringSource, Token};
    use crate::MacroContext;
    use runestick::Span;
    use Kind::*;

    fn token(kind: Kind) -> Token {
        Token {
            span: Span::default(),
            kind,
        }
    }

    #[test]
    fn test_tokens() {
        let ctx = MacroContext::empty();

        crate::macros::with_context(ctx, || {
            assert_eq!(vec![token(Amp)], quote!(&));
            assert_eq!(vec![token(Abstract)], quote!(abstract));
            assert_eq!(vec![token(AlignOf)], quote!(alignof));
            assert_eq!(vec![token(Amp)], quote!(&));
            assert_eq!(vec![token(AmpAmp)], quote!(&&));
            assert_eq!(vec![token(AmpEq)], quote!(&=));
            assert_eq!(vec![token(Arrow)], quote!(->));
            assert_eq!(vec![token(As)], quote!(as));
            assert_eq!(vec![token(Async)], quote!(async));
            assert_eq!(vec![token(At)], quote!(at));
            assert_eq!(vec![token(Await)], quote!(await));
            assert_eq!(vec![token(Bang)], quote!(!));
            assert_eq!(vec![token(BangEq)], quote!(!=));
            assert_eq!(vec![token(Become)], quote!(become));
            assert_eq!(vec![token(Break)], quote!(break));
            assert_eq!(vec![token(Caret)], quote!(^));
            assert_eq!(vec![token(CaretEq)], quote!(^=));
            assert_eq!(vec![token(Colon)], quote!(:));
            assert_eq!(vec![token(ColonColon)], quote!(::));
            assert_eq!(vec![token(Comma)], quote!(,));
            assert_eq!(vec![token(Const)], quote!(const));
            assert_eq!(vec![token(Crate)], quote!(crate));
            assert_eq!(vec![token(Dash)], quote!(-));
            assert_eq!(vec![token(DashEq)], quote!(-=));
            assert_eq!(vec![token(Default)], quote!(default));
            assert_eq!(vec![token(Div)], quote!(/));
            assert_eq!(vec![token(Do)], quote!(do));
            // NB: `$` is reserved as a designator in Rust.
            // assert_eq!(vec![token(Dollar)], quote!($));
            assert_eq!(vec![token(Dot)], quote!(.));
            assert_eq!(vec![token(DotDot)], quote!(..));
            assert_eq!(vec![token(Else)], quote!(else));
            assert_eq!(vec![token(Enum)], quote!(enum));
            assert_eq!(vec![token(Eq)], quote!(=));
            assert_eq!(vec![token(EqEq)], quote!(==));
            assert_eq!(vec![token(Extern)], quote!(extern));
            assert_eq!(vec![token(False)], quote!(false));
            assert_eq!(vec![token(Final)], quote!(final));
            assert_eq!(vec![token(Fn)], quote!(fn));
            assert_eq!(vec![token(For)], quote!(for));
            assert_eq!(vec![token(Gt)], quote!(>));
            assert_eq!(vec![token(GtEq)], quote!(>=));
            assert_eq!(vec![token(GtGt)], quote!(>>));
            assert_eq!(vec![token(GtGtEq)], quote!(>>=));
            assert_eq!(vec![token(If)], quote!(if));
            assert_eq!(vec![token(Impl)], quote!(impl));
            assert_eq!(vec![token(In)], quote!(in));
            assert_eq!(vec![token(Is)], quote!(is));
            assert_eq!(vec![token(Let)], quote!(let));
            assert_eq!(vec![token(Loop)], quote!(loop));
            assert_eq!(vec![token(Lt)], quote!(<));
            assert_eq!(vec![token(LtEq)], quote!(<=));
            assert_eq!(vec![token(LtLt)], quote!(<<));
            assert_eq!(vec![token(LtLtEq)], quote!(<<=));
            assert_eq!(vec![token(Macro)], quote!(macro));
            assert_eq!(vec![token(Match)], quote!(match));
            assert_eq!(vec![token(Mod)], quote!(mod));
            assert_eq!(vec![token(Move)], quote!(move));
            assert_eq!(vec![token(Not)], quote!(not));
            assert_eq!(vec![token(OffsetOf)], quote!(offsetof));
            assert_eq!(vec![token(Override)], quote!(override));
            assert_eq!(vec![token(Perc)], quote!(%));
            assert_eq!(vec![token(PercEq)], quote!(%=));
            assert_eq!(vec![token(Pipe)], quote!(|));
            assert_eq!(vec![token(PipeEq)], quote!(|=));
            assert_eq!(vec![token(PipePipe)], quote!(||));
            assert_eq!(vec![token(Plus)], quote!(+));
            assert_eq!(vec![token(PlusEq)], quote!(+=));
            assert_eq!(vec![token(Pound)], quote!(#));
            assert_eq!(vec![token(Priv)], quote!(priv));
            assert_eq!(vec![token(Proc)], quote!(proc));
            assert_eq!(vec![token(Pub)], quote!(pub));
            assert_eq!(vec![token(Pure)], quote!(pure));
            assert_eq!(vec![token(QuestionMark)], quote!(?));
            assert_eq!(vec![token(Ref)], quote!(ref));
            assert_eq!(vec![token(Return)], quote!(return));
            assert_eq!(vec![token(Rocket)], quote!(=>));
            assert_eq!(vec![token(Select)], quote!(select));
            assert_eq!(vec![token(SelfType)], quote!(Self));
            assert_eq!(vec![token(SelfValue)], quote!(self));
            assert_eq!(vec![token(SemiColon)], quote!(;));
            assert_eq!(vec![token(SizeOf)], quote!(sizeof));
            assert_eq!(vec![token(SlashEq)], quote!(/=));
            assert_eq!(vec![token(Star)], quote!(*));
            assert_eq!(vec![token(StarEq)], quote!(*=));
            assert_eq!(vec![token(Static)], quote!(static));
            assert_eq!(vec![token(Struct)], quote!(struct));
            assert_eq!(vec![token(Super)], quote!(super));
            assert_eq!(vec![token(Template)], quote!(template));
            assert_eq!(vec![token(Tilde)], quote!(~));
            assert_eq!(vec![token(True)], quote!(true));
            assert_eq!(vec![token(TypeOf)], quote!(typeof));
            assert_eq!(vec![token(Underscore)], quote!(_));
            assert_eq!(vec![token(Unsafe)], quote!(unsafe));
            assert_eq!(vec![token(Use)], quote!(use));
            assert_eq!(vec![token(Virtual)], quote!(virtual));
            assert_eq!(vec![token(While)], quote!(while));
            assert_eq!(vec![token(Yield)], quote!(yield));
        });
    }

    #[test]
    fn test_synthetic() {
        let ctx = MacroContext::empty();

        crate::macros::with_context(ctx, || {
            assert_eq!(
                vec![token(Ident(StringSource::Synthetic(0)))],
                quote!(hello)
            );
            assert_eq!(
                vec![token(LitByteStr(LitStrSource::Synthetic(0)))],
                quote!(b"hello")
            );
            assert_eq!(
                vec![token(LitStr(LitStrSource::Synthetic(0)))],
                quote!("hello")
            );
            assert_eq!(
                vec![token(LitNumber(NumberSource::Synthetic(0)))],
                quote!(0)
            );
            assert_eq!(
                vec![token(LitNumber(NumberSource::Synthetic(1)))],
                quote!(42.0)
            );
            assert_eq!(vec![token(LitChar(CopySource::Inline('a')))], quote!('a'));
            assert_eq!(vec![token(LitByte(CopySource::Inline(b'a')))], quote!(b'a'));
        });
    }

    #[test]
    fn test_iterator_iter() {
        let ctx = MacroContext::empty();

        crate::macros::with_context(ctx, || {
            let iter = quote!(self struct enum);

            assert_eq!(
                vec![token(SelfValue), token(Struct), token(Enum)],
                quote!(#(iter)*)
            );
        });
    }

    #[test]
    fn test_iterator_join() {
        let ctx = MacroContext::empty();

        crate::macros::with_context(ctx, || {
            let iter = quote!(self struct enum);

            assert_eq!(
                vec![
                    token(SelfValue),
                    token(Comma),
                    token(Struct),
                    token(Comma),
                    token(Enum)
                ],
                quote!(#(iter),*)
            );
        });
    }
}
