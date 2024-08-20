#[macro_use]
mod macros;

#[test]
fn format_literals() {
    assert_format!(
        r#"
        -100;
        100;
        100.0;
        -100.0;
        100.0e10;
        -100.0e10;
        true;
        false;
        "hello world";
        b"hello world";
        "#
    )
}

#[test]
fn test_layout_string() {
    assert_format!(
        r#"
        let x = 1; let y = 2; x + y
        "#,
        r#"
        let x = 1;
        let y = 2;

        x + y
        "#
    );
}

#[test]
fn fmt_comment_line() {
    assert_format!(
        r#"
        pub fn main() {
            // test 1
            if true {
                // test 2.1
                let a = 1;
                // test 2.2
            }
            // test 3
        }
        "#
    );
}

#[test]
fn avoid_block_inject() {
    assert_format!(
        r#"
        pub fn main() {
            if true {
                let a = 1;
            }
        }
        "#
    );
}

/// https://github.com/rune-rs/rune/issues/684
#[test]
fn bug_684() {
    assert_format!(
        r#"
        pub fn main() {
            /*
            test
            */
        }
        "#
    );

    assert_format!(
        r#"
        pub fn main() {/*
            test
            */}
        "#,
        r#"
        pub fn main() { /*
            test
            */
        }
        "#,
    );
}

#[test]
fn keep_trailing_comments() {
    assert_format!(
        r#"
        pub fn main() {
            let var = 42; // THIS is really important.
        }
        "#
    );

    assert_format!(
        r#"
        const DOVECOT_UNCHANGED = [
            //("dovecot-core", "dovecot-db.conf.ext"),
            ("dovecot-core", "dovecot-dict-auth.conf.ext"),
            ("dovecot-core", "dovecot-dict-sql.conf.ext"),
            ("dovecot-core", "dovecot-sql.conf.ext"),
            //("dovecot-core", "dovecot.conf"),
        ];
        "#
    );
}

#[test]
fn fmt_block_comment_indent2() {
    assert_format!(
        r#"
        /* HI BOB */

        let a = 42;

        // HI BOB

        let a = 42;

        // HI BOB
        let a = 42;

        /* HI BOB */
        let a = 42;
        "#
    );

    assert_format!(
        r#"
        /* HI BOB */

        pub fn test() {
            /* test1
            test2 */

            if true {
                /*
                if false {
                    // test3
                }
                */
            }

            /* else {
                // test 4
            } */
        }
        /* test 5.1
            test 5.2
                test 5.3
        */
        "#
    );
}

/// https://github.com/rune-rs/rune/issues/703
#[test]
fn bug_703() {
    const EXPECTED: &str = r#"
    pub fn main() {
        const TEST = 1;
    }
    "#;

    assert_format!(EXPECTED, EXPECTED);
}

#[test]
fn fmt_global_const() {
    const INPUT: &str = r#"
    const TEST1 = 1;const TEST2 = 2;
    const TEST3 = 1;
    "#;

    const EXPECTED: &str = r#"
    const TEST1 = 1;
    const TEST2 = 2;
    const TEST3 = 1;
    "#;

    assert_format!(INPUT, EXPECTED)
}

/// This tests that we have "reasonable defaults" when we have something to
/// format which contains no existing line hints.
#[test]
fn format_compact() {
    assert_format!(
        r#"
        pub fn main(){let value=0;while value<100{if value>=50{break;}value=value+1;}println!("The value is {}",value); // => The value is 50
        }
        "#,
        r#"
        pub fn main() {
            let value = 0;

            while value < 100 {
                if value >= 50 {
                    break;
                }

                value = value + 1;
            }

            println!("The value is {}", value); // => The value is 50
        }
        "#
    );
}

#[test]
fn format_items() {
    assert_format!(
        r#"
        struct    Foo
        struct    Foo    ;;;
        "#,
        r#"
        struct Foo;
        struct Foo;
        "#
    );

    assert_format!(
        r#"
        let a=10;
        let b/*stuck in the middle with you*/=10;
        let c=10;

        struct     Foo { foo,,, }
        struct     Foo  (foo,,,)
        struct    Foo
        struct    Foo    ;;;
        enum Foo {Bar}
        enum Foo

        /// Hello!   
        fn hello(foo, bar,   baz,,,    ) {
            fn bar() {
            let                  a = 42;
            // Hello world
            // WHAT
            // BYE
            }
        }

        // HI BOB
        "#,
        r#"
        let a = 10;
        let b /*stuck in the middle with you*/ = 10;
        let c = 10;

        struct Foo {
            foo,
        }
        struct Foo(foo);
        struct Foo;
        struct Foo;
        enum Foo {
            Bar,
        }
        enum Foo {}

        /// Hello!
        fn hello(foo, bar, baz) {
            fn bar() {
                let a = 42;
                // Hello world
                // WHAT
                // BYE
            }
        }

        // HI BOB
        "#
    );
}

#[test]
fn items() {
    assert_format!("const ITEM = 42;", "const ITEM = 42;");

    assert_format!(
        r#"
        enum Foo {
            Bar,
            Baz(tuple),
            Struct {
                a,
                b,
            },
        }
        "#
    );

    assert_format!(
        r#"
        struct Bar;

        struct Foo(tuple);

        struct Foo {
            field,
        }
        "#
    );
}

#[test]
fn patterns() {
    assert_format!("let (a,,,) = (a,,,,);", "let (a,) = (a,);");
    assert_format!("let () = ()", "let () = ();");
    assert_format!("let ::a::b::c = #{ foo: 42 };");
    assert_format!(
        "let ::   a::b   ::c = ::   a::b   ::c",
        "let ::a::b::c = ::a::b::c;"
    );
    assert_format!(
        "let ::a   ::b::<::b, ::c    ::d>::c = 42;",
        "let ::a::b::<::b, ::c::d>::c = 42;"
    );
    assert_format!(
        "for _ in 121/10..=1*2-100{}",
        r#"
        for _ in 121 / 10..=1 * 2 - 100 {
        }
        "#
    );
    assert_format!("let _ = |(a,,d,),,,a| 42;", "let _ = |(a, d), a| 42;");
    assert_format!("let _ = |    |    42    ;", "let _ = || 42;");
    assert_format!("let _ = ||    42    ;", "let _ = || 42;");
    assert_format!("let #[ignore] (a,) = (42,);");
    assert_format!(
        r#"
        let #{ a, b, c: d } = value;
        let #{} = value;
        let #{ a } = value;
        let #{ a: b } = value;
        let Foo { a } = value;
        let Foo {} = value;
        let Foo { a: b } = value;
        let Foo { a: b, .. } = value;
        let Foo { a: _, .. } = value;
        "#
    );
}

#[test]
fn expressions() {
    assert_format!("let a = #{};");
    assert_format!(
        "let a = Foo{foo,bar,,baz:42};",
        "let a = Foo { foo, bar, baz: 42 };"
    );
    assert_format!("let a = #{ foo: 42, bar };");
    assert_format!(
        "let a = #{   foo: 42    ,,,,  bar};",
        "let a = #{ foo: 42, bar };"
    );
    assert_format!("let a=(42*20)+52/2;", "let a = (42 * 20) + 52 / 2;");

    assert_format!(
        "if let var = 10 {20} else if true {5*1} else {20+30}",
        r#"
        if let var = 10 {
            20
        } else if true {
            5 * 1
        } else {
            20 + 30
        }
        "#
    );

    assert_format!(
        "while let var = 10{1+2}",
        r#"
        while let var = 10 {
            1 + 2
        }
        "#
    );

    assert_format!(
        "loop{1+2}",
        r#"
        loop {
            1 + 2
        }
        "#
    );

    assert_format!(
        "for(a,b)in iter {v = a + b}",
        r#"
        for (a, b) in iter {
            v = a + b
        }
        "#
    );

    assert_format!(
        "match foo {(1, 2)=>{true},true=>false,}",
        r#"
        match foo {
            (1, 2) => {
                true
            }
            true => false,
        }"#
    );

    assert_format!("value .   field    [   42]", "value.field[42]");

    assert_format!(
        "value   .   field   (1, 2, 3      ,,,,4)",
        "value.field(1, 2, 3, 4)"
    );

    assert_format!(
        "value   .   field   (1, 2, 3      ,,,,4)    .   await   ?",
        "value.field(1, 2, 3, 4).await?"
    );

    assert_format!(
        r#"
        value   .   field   (1, 2, 3      ,,,,4);;;
        let var = 42
        "#,
        r#"
        value.field(1, 2, 3, 4);
        let var = 42;
        "#
    );

    assert_format!(
        r#"
        #[test] a.b.c(42)
        "#
    );

    assert_format!(
        r#"
        #[test] a.b.c(#[cfg(ignore)] 42)
        "#
    );

    assert_format!(
        "1..2 1.. .. ..1 1..=2 ..=1",
        r#"
        1..2
        1..
        ..
        ..1
        1..=2
        ..=1
        "#
    );

    assert_format!(
        r#"
        1 * 2..1 * 2
        1 * 2..
        ..
        ..1 * 2
        1 * 4..=2
        ..=1 * 2
        "#
    );

    assert_format!(
        r#"
        -foo
        *foo
        &foo
        !foo
        "#,
        r#"
        -foo * foo & foo
        !foo
        "#
    );

    assert_format!(
        r#"
        let c = |a| |b| |c| a + b + c;

        (dx * dx + dy * dy + dz * dz).sqrt();
        bj.vx = bj.vx + (dx * bm);
        "#
    );

    assert_format!(
        r#"
        let _ = #{ a, b, c: d };
        let _ = #{};
        let _ = #{ a };
        let _ = #{ a: b };
        let _ = Foo { a };
        let _ = Foo {};
        let _ = Foo { a: b };
        "#
    );

    assert_format!(
        r#"
        'label: 'label: 'label: 42
        'label: 'label: 'label: {
            1 + 2
        }
        "#
    );

    assert_format!(
        r#"
        return;
        return 10;
        break 'label;
        break 'label 10;
        break 'label 'label 20;
        continue;
        continue 'label;
        continue 'label 'label;
        yield;
        yield 10;
        "#
    );

    assert_format!(
        r#"
        let value = match value {
            Some(value) => value,
            Some(value) => value,
        };
        "#
    );

    assert_format!(
        r#"
        let value = select {
            Some(value) = a.next() => value,
            Some(value) = b.next() => value,
            default => value,
        };

        let value = select {
            Some(value) => value,
            Some(value) = b.next() => value,
            default => value,
        };
        "#
    );

    assert_format!(
        r#"
        mod test {
            fn foo() {
                42
            }
        }

        mod bar;
        "#
    );

    assert_format!(
        r#"
        while path < 10 {
        }
        "#
    );

    assert_format!(
        r#"
        [
            // hello
            #{ a: 42 },
        ]
        "#
    );

    assert_format!(r#"`http://httpstat.us/200?sleep=${timeout}`"#);

    assert_format!(
        r#"
        let _ = async {};
        let _ = async || {};
        let _ = async |a| {};
        let _ = async |a, b| {};
        let _ = async {
            a + b
        };
        "#
    );

    assert_format!(
        r#"
        if values - current_joltage < 4 {
        }
        "#
    );
}

#[test]
fn paths() {
    assert_format!("foo::bar::<self, baz>");
    assert_format!("self");
    assert_format!("bar");
}

#[test]
fn superflous_commas() {
    assert_format!(
        r#"
        fn foo(,,,a,,,) {
        }
        "#,
        r#"
        fn foo(a) {
        }
        "#
    );
}

#[test]
fn use_statements() {
    assert_format!(
        r#"
        use foo::bar::baz;
        use foo as bar;
        use foo::{bar as baz, biz as buz};
        "#
    );

    assert_format!(
        r#"
        use std::collections::{HashMap,,,,,,,, hash_map::*};
        use std::collections::{HashMap};
        "#,
        r#"
        use std::collections::{HashMap, hash_map::*};
        use std::collections::HashMap;
        "#
    );
}

#[test]
fn programs() {
    assert_format!(
        r#"pub fn main(){println!("The value is {}",42);}"#,
        r#"
        pub fn main() {
            println!("The value is {}", 42);
        }
        "#
    );

    assert_format!(
        r#"
        pub fn main() {
            let var = 1;
        }
        "#
    );

    assert_format!(
        r#"
        use std::collections::HashMap;

        impl Foo::Bar::Baz {
            fn this(self) {
                42
            }
        }

        impl List {
            fn append(self, item) {
                self.item = item;
            }
        }
        "#
    );

    assert_format!(
        r#"
        struct Timeout;

        async fn request(timeout) {
            let request = http::get(`http://httpstat.us/200?sleep=${timeout}`);
            let timeout = time::sleep(time::Duration::from_secs(2));

            let result = select {
                _ = timeout => Err(Timeout),
                res = request => res,
            }?;

            println!("{}", result.status());
            Ok(())
        }

        pub async fn main() {
            if let Err(Timeout) = request(1000).await {
                println("Request timed out!");
            }

            if let Err(Timeout) = request(4000).await {
                println("Request timed out!");
            }
        }
        "#
    );

    assert_format!(
        r#"
        fn main() {
            let x=1;let y=2;   x + y
        }

        fn foo() {
            let x=1;let y=2;   x + y
        }
        "#,
        r#"
        fn main() {
            let x = 1;
            let y = 2;

            x + y
        }

        fn foo() {
            let x = 1;
            let y = 2;

            x + y
        }
        "#
    );

    // Test that we keep existing line hints.
    assert_format!(
        r#"
        fn foo() {
            let x = 1;
            let y = 2;
            x + y
        }
        "#
    );

    assert_format!(
        r#"
        fn main() {
            foo!();

            1 + 2
        }
        "#
    );
}

#[test]
fn disambiguate() {
    assert_format!("path < 10");
}

#[test]
fn labeled_block() {
    assert_format!(
        r#"
        let out = 'block1: {
            'block2: {
                if value == 0 {
                    break 'block1 10;
                }
            }

            77
        };
        "#
    )
}

#[test]
fn field_generics() {
    assert_format!(
        r#"
        let a = [1, 2].iter().collect::<Vec>();
        "#
    )
}

#[test]
fn comments() {
    assert_format!(
        r#"
        match foo {
            // Hello World!
            () => 10,
            // Bye!
        }
        "#
    );

    assert_format!(
        r#"
        //test1
        /*test2*/
        "#
    );

    assert_format!(
        r#"
        struct Test {
            a,
            /* test1
            test2
            test 3 */
        }
        "#
    );

    // https://github.com/rune-rs/rune/issues/710
    assert_format!(
        r#"
        pub fn main() {
            if true {
                // test
            }
        }
        "#
    );

    assert_format!(
        r#"
        (/* a */);
        (/* a */ a /* b */);
        [/* a */] /* c */ /* d */;
        [/* a */ /* b */] /* c */;
        [/* a */ a /* b */];

        fn /* a */ foo(/* a */ arg /* b */) /* d */ {
        }

        struct /* a */ Struct(/* a */ arg /* b */) /* d */;
        struct /* a */ Struct(/* b */) /* d */;
        struct /* a */ Struct(/* b */ /* c */) /* d */;
        struct /* a */ Struct() /* d */;
        "#
    );

    assert_format!(
        r#"
        match self {
            // A
            Self::A => {}
            // B
            Self::B => {},
            // C
            Self::C => {},
            // D
        }
        "#,
        r#"
        match self {
            // A
            Self::A => {}
            // B
            Self::B => {}
            // C
            Self::C => {}
            // D
        }
        "#
    );
}

#[test]
fn test_macro_function_like() {
    assert_format!(
        r#"
        println!("Hello",42,,,100);
        "#,
        r#"
        println!("Hello", 42,,, 100);
        "#,
    );

    assert_format!(
        r#"
        println!(                  );
        "#,
        r#"
        println!();
        "#,
    );

    assert_format!(
        r#"
        println!{};
        "#
    );

    assert_format!(
        r#"
        println!{1,2,3,4,};
        "#,
        r#"
        println!{
            1,
            2,
            3,
            4,
        };
        "#,
    );

    assert_format!(
        r#"
        fn main() {
            foo!();

            1 + 2
        }
        "#
    );

    assert_format!(
        r#"
        make_function!(root_fn => {"Hello World!"});
        // NB: we put the import in the bottom to test that import resolution isn't order-dependent.
        "#
    );

    assert_format!(
        r#"
        make_function!(root_fn => {"Hello World!"});

        // NB: we put the import in the bottom to test that import resolution isn't order-dependent.
        "#
    );

    assert_format!(
        r#"
        make_function!(root_fn => {"Hello World!"});


        // NB: we put the import in the bottom to test that import resolution isn't order-dependent.
        "#,
        r#"
        make_function!(root_fn => {"Hello World!"});

        // NB: we put the import in the bottom to test that import resolution isn't order-dependent.
        "#
    );

    assert_format!(
        r#"
        println!{
            hello,,,, world { 1 + 2 },
        };
        "#,
        r#"
        println!{
            hello,,,,
            world { 1 + 2 },
        };
        "#,
    );
}

#[test]
fn glued_path_token() {
    assert_format!(
        r#"
        if ctx.features.kdump {
            if ctx.system.hw_memory < 8 * 1024 * 1024 {
                cmd_line_default.push("crashkernel=128M");
            } else {
                cmd_line_default.push("crashkernel=256M");
            }
        }
        "#
    );
}

#[test]
fn preserve_newlines() {
    assert_format!(
        r#"
        [
        ];

        [
        ];
        "#,
        r#"
        [];

        [];
        "#
    );

    assert_format!(
        r#"
        foo(
            [1, 2, 3, 4]
        );

        foo(
            [1, 2, 3, 4]
        );
        "#,
        r#"
        foo([1, 2, 3, 4]);

        foo([1, 2, 3, 4]);
        "#
    );
}

#[test]
fn runefmt_skip() {
    assert_format!(
        r#"
        #[runefmt::skip]
        let var = [
            1, 2,
            3, 4,
            5, 6,
            7, 8,
            9, 10
        ];

        let var = [
            1, 2,
            3, 4,
            5,
        ];

        #[runefmt::skip]
        let var = [
            1, 2,
            3, 4,
            5, 6,
            7, 8,
            9, 10
        ];
        "#,
        r#"
        #[runefmt::skip]
        let var = [
            1, 2,
            3, 4,
            5, 6,
            7, 8,
            9, 10
        ];

        let var = [1, 2, 3, 4, 5];

        #[runefmt::skip]
        let var = [
            1, 2,
            3, 4,
            5, 6,
            7, 8,
            9, 10
        ];
        "#
    );

    assert_format!(
        r#"
        #[runefmt::skip]
        struct     Struct     {
          }
        "#
    );

    assert_format!(
        r#"
        #[runefmt::skip] 1 +                       2
        "#
    );
}

// Test that we can format syntactically invalid sequences.
#[test]
fn test_error_patterns() {
    assert_format_with!({ "fmt.error-recovery=true" }, "let var = +/-=;",);

    assert_format_with!(
        { "fmt.error-recovery=true" },
        r#"
        let var = +/-=;

        struct Foo;
        "#
    );

    assert_format_with!(
        { "fmt.error-recovery=true" },
        r#"
        let var = +/-=; // Hi Bob

        struct Foo;
        "#
    );
}

#[test]
fn braced_expression() {
    assert_format!(
        r#"
        match yield test {
            1 => 2,
            3 => 4,
        }

        match yield {
            1 => 2,
            3 => 4,
        }
        "#
    );

    assert_format!(
        r#"
        match break test {
            1 => 2,
            3 => 4,
        }

        match break {
            1 => 2,
            3 => 4,
        }
        "#
    );

    assert_format!(
        r#"
        match return test {
            1 => 2,
            3 => 4,
        }

        match return {
            1 => 2,
            3 => 4,
        }
        "#
    );
}

#[test]
fn modifiers() {
    assert_format!(
        r#"
        pub(crate) mod iter {
        }

        pub(super) mod iter {
        }

        pub(self) mod iter {
        }

        pub(in foo::bar) mod iter {
        }

        const mod iter {
        }

        move mod iter {
        }

        async mod iter {
        }
        "#
    );
}

#[test]
fn associated_paths() {
    assert_format!(
        r#"
        struct Foo;

        impl Foo {
            fn new() {
                Self { field: 42 }
            }
        }
        "#
    );
}

#[test]
fn expressions_in_group() {
    assert_format!(
        r#"
        (match value {});

        (match value {
            _ => 2,
        });

        (if true {
        });

        (while true {
        });

        (for _ in value {
        });

        (loop {
        });

        (select {
        });
        "#
    )
}

#[test]
fn test_expanded_chain() {
    assert_format!(
        r#"
        let graph = HashMap::from_iter(abcd, abcd, abcd, abcd, abcd, abcd, abcd, abcd, abcd, abcd, abcd, abcd, abcd).bar(abcd, abcd, abcd, abcd, abcd, abcd, abcd, abcd, abcd, abcd, abcd, abcd, abcd);

        let var = 10;
        "#,
        r#"
        let graph = HashMap::from_iter(
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
            )
            .bar(
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
            );

        let var = 10;
        "#
    );

    assert_format!(
        r#"
        let graph = HashMap::from_iter(abcd, abcd, abcd, abcd, abcd, abcd, abcd, abcd, abcd, abcd, abcd, abcd, abcd);

        let var = 10;
        "#,
        r#"
        let graph = HashMap::from_iter(
            abcd,
            abcd,
            abcd,
            abcd,
            abcd,
            abcd,
            abcd,
            abcd,
            abcd,
            abcd,
            abcd,
            abcd,
            abcd,
        );

        let var = 10;
        "#
    );

    assert_format!(
        r#"
        let graph = value.foo.bar.await?(
            abcd,
            abcd,
            abcd,
            abcd,
            abcd,
            abcd,
            abcd,
            abcd,
            abcd,
            abcd,
            abcd,
            abcd,
            abcd,
        );

        let var = 10;
        "#
    );

    assert_format!(
        r#"
        let graph = value
            .foo
            .bar
            .await?(
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
                abcd,
            )
            .bar?;

        let var = 10;
        "#
    );
}
