//! Tests for `const` items declared in script `impl` blocks.

prelude!();

/// An impl constant is an item under the type, so without indexing it there
/// `Foo::X` names nothing.
#[test]
fn an_impl_constant_is_reachable_through_its_type() {
    let out: i64 = eval(
        r#"
        struct Foo;

        impl Foo {
            const X = 10;
        }

        Foo::X
        "#,
    );
    assert_eq!(out, 10);
}

/// The visibility a constant takes at the top level is accepted on one in an
/// impl block as well, rather than rejected the way it is on an impl function.
#[test]
fn an_impl_constant_may_be_public() {
    let out: i64 = eval(
        r#"
        struct Foo;

        impl Foo {
            pub const X = 10;
        }

        Foo::X
        "#,
    );
    assert_eq!(out, 10);
}

/// `Self` inside an impl function names the type, so it reaches the type's
/// constants the same way it reaches its functions.
#[test]
fn an_impl_constant_is_reachable_through_self_in_an_impl_function() {
    let out: i64 = eval(
        r#"
        struct Foo;

        impl Foo {
            const X = 10;

            fn x() { Self::X }
        }

        Foo::x()
        "#,
    );
    assert_eq!(out, 10);
}

/// One impl constant may be defined in terms of another, by `Self` or by the
/// type's name, in either order.
#[test]
fn an_impl_constant_may_refer_to_another() {
    let out: i64 = eval(
        r#"
        struct Foo;

        impl Foo {
            const B = Self::A * 2;
            const A = 21;
            const C = Foo::B + 1;
        }

        Foo::C
        "#,
    );
    assert_eq!(out, 43);
}

/// An impl constant is a constant, so it can be used where one is required.
#[test]
fn an_impl_constant_can_be_used_in_a_pattern() {
    let out: i64 = eval(
        r#"
        struct Foo;

        impl Foo {
            const X = 10;
        }

        match 10 { Foo::X => 1, _ => 0 }
        "#,
    );
    assert_eq!(out, 1);
}

/// Without this a constant and a function of the same name under one type
/// would silently shadow each other. It is reported even when neither is used,
/// the way two top-level items of the same name are.
#[test]
fn an_impl_constant_and_function_of_the_same_name_is_an_error() {
    assert_errors! {
        "struct Foo; impl Foo { const X = 10; fn X() { 1 } } 0",
        span!(33, 35), ErrorKind::AmbiguousItem { item, .. } => {
            assert_eq!(item.to_string(), "Foo::X");
        }
    };
}

/// Two impl blocks for one type share its namespace, so a constant declared in
/// both is as ambiguous as one declared twice in the same block.
#[test]
fn an_impl_constant_declared_in_two_impl_blocks_is_an_error() {
    assert_errors! {
        "struct Foo; impl Foo { const X = 10; } impl Foo { const X = 11; } 0",
        span!(33, 35), ErrorKind::AmbiguousItem { item, .. } => {
            assert_eq!(item.to_string(), "Foo::X");
        }
    };
}

/// A constant which refers to itself cannot be evaluated, and that is a
/// compile error rather than an overflow. It reads as a missing item, which is
/// what a cycle between top-level constants is reported as too.
#[test]
fn an_impl_constant_cycle_is_an_error() {
    assert_errors! {
        "struct Foo; impl Foo { const A = Self::B; const B = Self::A; } 0",
        _, ErrorKind::MissingItem { item } => {
            assert!(matches!(item.to_string().as_str(), "Foo::A" | "Foo::B"), "{item}");
        }
    };
}

/// The motivating example: a constant in an impl block whose value is an
/// instance of the type it is declared on.
#[test]
fn an_impl_constant_may_hold_a_tuple_struct() {
    let out: bool = eval(
        r#"
        pub struct Color(a, b, c, d);

        impl Color {
            pub const RED = Self(255, 0, 0, 255);
        }

        Color::RED.0 == 255 && Color::RED.3 == 255 && Color::RED.1 == 0
        "#,
    );
    assert!(out);
}

/// The same value is accepted at the top level, built through the type's name.
#[test]
fn a_constant_may_hold_a_tuple_struct() {
    let out: i64 = eval(
        r#"
        struct Color(a, b, c, d);

        const RED = Color(255, 0, 0, 255);

        RED.0 + RED.3
        "#,
    );
    assert_eq!(out, 510);
}

/// A struct with named fields keeps its fields by name, whatever order they
/// were written in.
#[test]
fn a_constant_may_hold_a_struct_with_named_fields() {
    let out: i64 = eval(
        r#"
        struct Point { x, y }

        impl Point {
            const ORIGIN = Self { y: 2, x: 1 };
        }

        const AWAY = Point { x: 10, y: 20 };

        Point::ORIGIN.x * 1000 + Point::ORIGIN.y * 100 + AWAY.x + AWAY.y
        "#,
    );
    assert_eq!(out, 1230);
}

/// A unit struct is a constant too.
#[test]
fn a_constant_may_hold_a_unit_struct() {
    let out: bool = eval(
        r#"
        struct Marker;

        impl Marker {
            const IT = Self;
        }

        const ALSO = Marker;

        match (Marker::IT, ALSO) {
            (Marker, Marker) => true,
            _ => false,
        }
        "#,
    );
    assert!(out);
}

/// Variants of every shape, at the top level and in the enum's own impl.
#[test]
fn a_constant_may_hold_an_enum_variant() {
    let out: i64 = eval(
        r#"
        enum Shape {
            Circle(radius),
            Rect { w, h },
            Empty,
        }

        impl Shape {
            const UNIT = Self::Circle(1);
            const SQUARE = Shape::Rect { h: 2, w: 2 };
        }

        const NOTHING = Shape::Empty;

        fn area(shape) {
            match shape {
                Shape::Circle(r) => r * r * 3,
                Shape::Rect { w, h } => w * h,
                Shape::Empty => 0,
            }
        }

        area(Shape::UNIT) * 100 + area(Shape::SQUARE) * 10 + area(NOTHING)
        "#,
    );
    assert_eq!(out, 340);
}

/// Script types nest inside each other and inside the containers constants
/// already supported.
#[test]
fn a_constant_may_nest_script_types() {
    let out: i64 = eval(
        r#"
        struct Point { x, y }
        struct Line(from, to);
        enum Maybe { Just(value), Nothing }

        const LINE = Line(Point { x: 1, y: 2 }, Point { x: 3, y: 4 });
        const ALL = [Some(Maybe::Just(LINE)), None, Some(Maybe::Nothing)];

        let Some(Maybe::Just(line)) = ALL[0] else {
            return 0;
        };

        line.0.x + line.0.y * 10 + line.1.x * 100 + line.1.y * 1000
        "#,
    );
    assert_eq!(out, 4321);
}

/// Every use of a constant is a fresh value, so changing one does not change
/// what the constant is the next time it is read.
#[test]
fn a_script_type_constant_is_built_fresh_on_every_use() {
    let out: i64 = eval(
        r#"
        struct Point { x, y }

        impl Point {
            const ORIGIN = Self { x: 0, y: 0 };
        }

        let p = Point::ORIGIN;
        p.x = 10;
        p.x + Point::ORIGIN.x
        "#,
    );
    assert_eq!(out, 10);
}

/// A constant whose value is a script type is matched against the way the
/// value it stands for would be.
#[test]
fn a_script_type_constant_may_be_used_as_a_pattern() {
    let out: i64 = eval(
        r#"
        struct Color(a, b, c, d);
        struct Point { x, y }
        enum Shape { Circle(radius), Rect { w, h }, Empty }

        impl Color {
            const RED = Self(255, 0, 0, 255);
        }

        impl Point {
            const ORIGIN = Self { x: 0, y: 0 };
        }

        const UNIT = Shape::Circle(1);
        const SQUARE = Shape::Rect { w: 2, h: 2 };
        const NOTHING = Shape::Empty;

        fn color(c) {
            match c {
                Color::RED => 1,
                _ => 0,
            }
        }

        fn point(p) {
            match p {
                Point::ORIGIN => 1,
                _ => 0,
            }
        }

        fn shape(s) {
            match s {
                UNIT => 1,
                SQUARE => 2,
                NOTHING => 3,
                _ => 0,
            }
        }

        let n = 0;
        n = n * 10 + color(Color(255, 0, 0, 255));
        n = n * 10 + color(Color(255, 0, 0, 0));
        n = n * 10 + point(Point { y: 0, x: 0 });
        n = n * 10 + point(Point { x: 0, y: 1 });
        n = n * 10 + shape(Shape::Circle(1));
        n = n * 10 + shape(Shape::Circle(2));
        n = n * 10 + shape(Shape::Rect { h: 2, w: 2 });
        n = n * 10 + shape(Shape::Empty);
        n
        "#,
    );
    assert_eq!(out, 10101023);
}

/// A constant made of a script type may be handed to a constant function.
#[test]
fn a_script_type_constant_may_be_passed_to_a_const_fn() {
    let out: i64 = eval(
        r#"
        struct Color(a, b, c, d);

        const fn red(color) {
            color.0
        }

        const RED = Color(255, 0, 0, 255);
        const R = red(RED);

        R
        "#,
    );
    assert_eq!(out, 255);
}
