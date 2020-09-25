/// Macro helper function for quoting the token stream as macro output.
///
/// Is capable of quoting everything in Rune, except for the following:
/// * Pound signs (`#`), which can be inserted by using `#(ast::Pound)` instead.
///   These are used in object literal, and are a limitiation in the quote macro
///   because we use the pound sign to delimit variables.
/// * Labels, which must be created using [crate::MacroContext::label].
/// * Template strings, which must be created using [crate::MacroContext::template_string].
///
/// ## Interpolating values
///
/// Values are interpolated with `#value`, or `#(value + 1)` for expressions.
///
/// ## Iterators
///
/// Anything that can be used as an iterator can be iterated over with
/// `#(iter)*`. A token can also be used to join inbetween each iteration, like
/// `#(iter),*`.
#[macro_export]
macro_rules! quote {
    ($ctx:expr => $($tt:tt)*) => {{
        let mut stream = $ctx.token_stream();

        {
            let stream = &mut stream;
            $crate::quote!(@push $ctx, stream => $($tt)*);
        }

        stream
    }};

    (@wrap $ctx:expr, $s:expr, $variant:ident => $($tt:tt)*) => {{
        $crate::ToTokens::to_tokens(&$crate::ast::Kind::Open($crate::ast::Delimiter::$variant), $ctx, $s);
        $crate::quote!(@push $ctx, $s => $($tt)*);
        $crate::ToTokens::to_tokens(&$crate::ast::Kind::Close($crate::ast::Delimiter::$variant), $ctx, $s);
    }};

    (@token $ctx:expr, $s:expr, $variant:ident => $($tt:tt)*) => {{
        $crate::ToTokens::to_tokens(&$crate::ast::Kind::$variant, $ctx, $s);
        $crate::quote!(@push $ctx, $s => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => #$var:ident $($tt:tt)*) => {{
        $crate::ToTokens::to_tokens(&$var, $ctx, $s);
        $crate::quote!(@push $ctx, $s => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => #($expr:expr) $repeat:tt * $($tt:tt)*) => {{
        let mut it = std::iter::IntoIterator::into_iter($expr).peekable();

        while let Some(v) = it.next() {
            $crate::ToTokens::to_tokens(&v, $ctx, $s);

            if it.peek().is_some() {
                $crate::quote!(@push $ctx, $s => $repeat);
            }
        }

        $crate::quote!(@push $ctx, $s => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => #($expr:expr)* $($tt:tt)*) => {{
        for v in $expr {
            $crate::ToTokens::to_tokens(&v, $ctx, $s);
        }

        $crate::quote!(@push $ctx, $s => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => self $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Self_ => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => macro $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Macro => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => fn $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Fn => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => enum $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Enum => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => struct $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Struct => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => is $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Is => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => not $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Not => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => let $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Let => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => if $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, If => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => match $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Match => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => else $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Else => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => use $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Use => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => while $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, While => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => loop $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Loop => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => for $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, For => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => in $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, In => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => true $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, True => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => false $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, False => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => break $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Break => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => yield $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Yield => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => return $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Return => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => await $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Await => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => async $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Async => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => select $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Select => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => default $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Default => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => impl $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Impl => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => mod $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Mod => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => # $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Pound => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => . $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Dot => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => :: $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, ColonColon => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => _ $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Underscore => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => , $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Comma => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => : $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Colon => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => ; $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, SemiColon => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => + $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Plus => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => - $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Dash => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => / $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Div => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => * $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Star => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => & $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Amp => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => = $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Eq => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => == $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, EqEq => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => != $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, BangEq => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => => $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Rocket => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => < $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Lt => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => > $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Gt => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => <= $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, LtEq => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => >= $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, GtEq => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => ! $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Bang => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => ? $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, QuestionMark => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => .. $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, DotDot => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => && $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, AmpAmp => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => || $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, PipePipe => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => | $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Pipe => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => % $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Perc => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => << $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, LtLt => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => >> $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, GtGt => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => ^ $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, Caret => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => += $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, PlusEq => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => -= $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, DashEq => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => *= $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, StarEq => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => /= $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, SlashEq => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => %= $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, PercEq => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => %= $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, PercEq => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => &= $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, AmpEq => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => ^= $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, CaretEq => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => |= $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, PipeEq => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => <<= $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, LtLtEq => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => >>= $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $s, GtGtEq => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => { $($tt:tt)* } $($rest:tt)*) => {{
        $crate::quote!(@wrap $ctx, $s, Brace => $($tt)*);
        $crate::quote!(@push $ctx, $s => $($rest)*);
    }};

    (@push $ctx:expr, $s:expr => [ $($tt:tt)* ] $($rest:tt)*) => {{
        $crate::quote!(@wrap $ctx, $s, Bracket => $($tt)*);
        $crate::quote!(@push $ctx, $s => $($rest)*);
    }};

    (@push $ctx:expr, $s:expr => ( $($tt:tt)* ) $($rest:tt)*) => {{
        $crate::quote!(@wrap $ctx, $s, Parenthesis => $($tt)*);
        $crate::quote!(@push $ctx, $s => $($rest)*);
    }};

    (@push $ctx:expr, $s:expr => ( $($tt:tt)* ) $($rest:tt)*) => {{
        $crate::quote!(@wrap $ctx, $s, Parenthesis => $($tt)*);
        $crate::quote!(@push $ctx, $s => $($rest)*);
    }};

    (@push $ctx:expr, $s:expr => $ident:ident $($tt:tt)*) => {{
        let kind = $ctx.ident(stringify!($ident));
        $crate::ToTokens::to_tokens(&kind, $ctx, $s);
        $crate::quote!(@push $ctx, $s => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr => $lit:literal $($tt:tt)*) => {{
        let token = $ctx.lit($lit);
        $crate::ToTokens::to_tokens(&token, $ctx, $s);
        $crate::quote!(@push $ctx, $s => $($tt)*);
    }};

    (@push $ctx:expr, $s:expr =>) => {};
}

#[cfg(test)]
mod tests {
    use crate::ast::{
        CopySource, Kind, LitByteStrSource, LitStrSource, NumberSource, StringSource, Token,
    };
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
        let ctx = &mut MacroContext::empty();

        assert_eq!(vec![token(Self_)], quote!(ctx => self));
        assert_eq!(vec![token(Macro)], quote!(ctx => macro));
        assert_eq!(vec![token(Fn)], quote!(ctx => fn));
        assert_eq!(vec![token(Enum)], quote!(ctx => enum));
        assert_eq!(vec![token(Struct)], quote!(ctx => struct));
        assert_eq!(vec![token(Is)], quote!(ctx => is));
        assert_eq!(vec![token(Not)], quote!(ctx => not));
        assert_eq!(vec![token(Let)], quote!(ctx => let));
        assert_eq!(vec![token(If)], quote!(ctx => if));
        assert_eq!(vec![token(Match)], quote!(ctx => match));
        assert_eq!(vec![token(Else)], quote!(ctx => else));
        assert_eq!(vec![token(Use)], quote!(ctx => use));
        assert_eq!(vec![token(While)], quote!(ctx => while));
        assert_eq!(vec![token(Loop)], quote!(ctx => loop));
        assert_eq!(vec![token(For)], quote!(ctx => for));
        assert_eq!(vec![token(In)], quote!(ctx => in));
        assert_eq!(vec![token(True)], quote!(ctx => true));
        assert_eq!(vec![token(False)], quote!(ctx => false));
        assert_eq!(vec![token(Break)], quote!(ctx => break));
        assert_eq!(vec![token(Yield)], quote!(ctx => yield));
        assert_eq!(vec![token(Return)], quote!(ctx => return));
        assert_eq!(vec![token(Await)], quote!(ctx => await));
        assert_eq!(vec![token(Async)], quote!(ctx => async));
        assert_eq!(vec![token(Select)], quote!(ctx => select));
        assert_eq!(vec![token(Default)], quote!(ctx => default));
        assert_eq!(vec![token(Impl)], quote!(ctx => impl));
        assert_eq!(vec![token(Mod)], quote!(ctx => mod));
        assert_eq!(vec![token(Pound)], quote!(ctx => #));
        assert_eq!(vec![token(Dot)], quote!(ctx => .));
        assert_eq!(vec![token(ColonColon)], quote!(ctx => ::));
        assert_eq!(vec![token(Underscore)], quote!(ctx => _));
        assert_eq!(vec![token(Comma)], quote!(ctx => ,));
        assert_eq!(vec![token(Colon)], quote!(ctx => :));
        assert_eq!(vec![token(SemiColon)], quote!(ctx => ;));
        assert_eq!(vec![token(Plus)], quote!(ctx => +));
        assert_eq!(vec![token(Dash)], quote!(ctx => -));
        assert_eq!(vec![token(Div)], quote!(ctx => /));
        assert_eq!(vec![token(Star)], quote!(ctx => *));
        assert_eq!(vec![token(Amp)], quote!(ctx => &));
        assert_eq!(vec![token(Eq)], quote!(ctx => =));
        assert_eq!(vec![token(EqEq)], quote!(ctx => ==));
        assert_eq!(vec![token(BangEq)], quote!(ctx => !=));
        assert_eq!(vec![token(Rocket)], quote!(ctx => =>));
        assert_eq!(vec![token(Lt)], quote!(ctx => <));
        assert_eq!(vec![token(Gt)], quote!(ctx => >));
        assert_eq!(vec![token(LtEq)], quote!(ctx => <=));
        assert_eq!(vec![token(GtEq)], quote!(ctx => >=));
        assert_eq!(vec![token(Bang)], quote!(ctx => !));
        assert_eq!(vec![token(QuestionMark)], quote!(ctx => ?));
        assert_eq!(vec![token(DotDot)], quote!(ctx => ..));
        assert_eq!(vec![token(AmpAmp)], quote!(ctx => &&));
        assert_eq!(vec![token(PipePipe)], quote!(ctx => ||));
        assert_eq!(vec![token(Pipe)], quote!(ctx => |));
        assert_eq!(vec![token(Perc)], quote!(ctx => %));
        assert_eq!(vec![token(LtLt)], quote!(ctx => <<));
        assert_eq!(vec![token(GtGt)], quote!(ctx => >>));
        assert_eq!(vec![token(Caret)], quote!(ctx => ^));
        assert_eq!(vec![token(PlusEq)], quote!(ctx => +=));
        assert_eq!(vec![token(DashEq)], quote!(ctx => -=));
        assert_eq!(vec![token(StarEq)], quote!(ctx => *=));
        assert_eq!(vec![token(SlashEq)], quote!(ctx => /=));
        assert_eq!(vec![token(PercEq)], quote!(ctx => %=));
        assert_eq!(vec![token(AmpEq)], quote!(ctx => &=));
        assert_eq!(vec![token(CaretEq)], quote!(ctx => ^=));
        assert_eq!(vec![token(PipeEq)], quote!(ctx => |=));
        assert_eq!(vec![token(LtLtEq)], quote!(ctx => <<=));
        assert_eq!(vec![token(GtGtEq)], quote!(ctx => >>=));
    }

    #[test]
    fn test_synthetic() {
        let ctx = &mut MacroContext::empty();

        assert_eq!(
            vec![token(Ident(StringSource::Synthetic(0)))],
            quote!(ctx => hello)
        );
        assert_eq!(
            vec![token(LitByteStr(LitByteStrSource::Synthetic(0)))],
            quote!(ctx => b"hello")
        );
        assert_eq!(
            vec![token(LitStr(LitStrSource::Synthetic(0)))],
            quote!(ctx => "hello")
        );
        assert_eq!(
            vec![token(LitNumber(NumberSource::Synthetic(0)))],
            quote!(ctx => 0)
        );
        assert_eq!(
            vec![token(LitNumber(NumberSource::Synthetic(1)))],
            quote!(ctx => 42.0)
        );
        assert_eq!(
            vec![token(LitChar(CopySource::Inline('a')))],
            quote!(ctx => 'a')
        );
        assert_eq!(
            vec![token(LitByte(CopySource::Inline(b'a')))],
            quote!(ctx => b'a')
        );
    }

    #[test]
    fn test_iterator_iter() {
        let ctx = &mut MacroContext::empty();
        let iter = quote!(ctx => self struct enum);

        assert_eq!(
            vec![token(Self_), token(Struct), token(Enum)],
            quote!(ctx => #(iter)*)
        );
    }

    #[test]
    fn test_iterator_join() {
        let ctx = &mut MacroContext::empty();
        let iter = quote!(ctx => self struct enum);

        assert_eq!(
            vec![
                token(Self_),
                token(Comma),
                token(Struct),
                token(Comma),
                token(Enum)
            ],
            quote!(ctx => #(iter),*)
        );
    }
}
