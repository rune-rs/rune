/// Macro helper function for quoting the token stream as macro output.
#[macro_export]
macro_rules! quote {
    ($ctx:expr => $($tt:tt)*) => {{
        let mut stream = $ctx.token_stream();

        {
            let mut stream = &mut stream;
            $crate::quote!(@push $ctx, stream => $($tt)*);
        }

        stream
    }};

    (@wrap $ctx:expr, $stream:expr, $variant:ident => $($tt:tt)*) => {{
        $crate::IntoTokens::into_tokens($crate::ast::Kind::Open($crate::ast::Delimiter::$variant), $ctx, $stream);
        $crate::quote!(@push $ctx, $stream => $($tt)*);
        $crate::IntoTokens::into_tokens($crate::ast::Kind::Close($crate::ast::Delimiter::$variant), $ctx, $stream);
    }};

    (@token $ctx:expr, $stream:expr, $variant:ident => $($tt:tt)*) => {{
        $crate::IntoTokens::into_tokens($crate::ast::Kind::$variant, $ctx, $stream);
        $crate::quote!(@push $ctx, $stream => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => #$var:ident $($tt:tt)*) => {{
        $crate::IntoTokens::into_tokens($var, $ctx, $stream);
        $crate::quote!(@push $ctx, $stream => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => #($var:ident) $repeat:tt * $($tt:tt)*) => {{
        let mut it = std::iter::IntoIterator($var).peekable();

        while let Some(v) = it.next() {
            $crate::IntoTokens::into_tokens(v, $ctx, $stream);

            if it.peek().is_some() {
                $crate::quote!(@push $ctx, $stream => $repeat);
            }
        }

        $crate::quote!(@push $ctx, $stream => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => #($var:ident)* $($tt:tt)*) => {{
        for v in $var {
            $crate::IntoTokens::into_tokens(v, $ctx, $stream);
        }

        $crate::quote!(@push $ctx, $stream => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => $ident:ident $($tt:tt)*) => {{
        let ident = $ctx.ident(stringify!($ident));
        $stream.push($stream);
        $crate::quote!(@push $ctx, $stream => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => self $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Self_ => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => if $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, If => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => else $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Else => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => let $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Let => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => use $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Use => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => while $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, While => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => loop $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Loop => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => for $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, For => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => in $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, In => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => match $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Match => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => select $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Select => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => macro $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Macro => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => enum $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Enum => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => struct $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Struct => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => true $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, True => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => false $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, False => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => break $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Break => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => yield $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Yield => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => :: $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, ColonColon => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => || $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, PipePipe => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => | $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Pipe => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => + $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Plus => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => - $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Minus => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => * $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Mul => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => / $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Div => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => , $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Comma => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => ! $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Bang => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => # $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Hash => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => struct $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Struct => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => fn $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Fn => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => async $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Async => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => default $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Default => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => impl $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Impl => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => mod $($tt:tt)*) => {{
        $crate::quote!(@token $ctx, $stream, Mod => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => { $($tt:tt)* }) => {{
        $crate::quote!(@wrap $ctx, $stream, Brace => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => [ $($tt:tt)* ]) => {{
        $crate::quote!(@wrap $ctx, $stream, Bracket => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr => ( $($tt:tt)* )) => {{
        $crate::quote!(@wrap $ctx, $stream, Parenthesis => $($tt)*);
    }};

    (@push $ctx:expr, $stream:expr =>) => {};
}
