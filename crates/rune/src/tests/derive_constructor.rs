#![allow(unused)]

prelude!();

#[derive(Any)]
#[rune(constructor)]
struct Empty;

#[derive(Any)]
#[rune(constructor)]
struct EmptyNamed {}

#[derive(Any)]
#[rune(constructor)]
struct EmptyUnnamed {}

#[derive(Any)]
#[rune(constructor)]
struct NamedZero {
    a: i32,
    b: String,
}

#[derive(Any)]
#[rune(constructor)]
struct NamedOne {
    #[rune(get)]
    a: i32,
    b: String,
}

#[derive(Any)]
#[rune(constructor)]
struct NamedTwo {
    #[rune(get)]
    a: i32,
    #[rune(get)]
    b: String,
}

#[derive(Any)]
#[rune(constructor)]
struct UnnamedZero(i32, String);

#[derive(Any)]
#[rune(constructor)]
struct UnnamedOne(#[rune(get)] i32, String);

#[derive(Any)]
#[rune(constructor)]
struct UnnamedTwo(#[rune(get)] i32, #[rune(get)] String);

#[derive(Any)]
enum Enum {
    #[rune(constructor)]
    Empty,
    #[rune(constructor)]
    EmptyNamed {},
    #[rune(constructor)]
    EmptyUnnamed {},
    #[rune(constructor)]
    NamedZero { a: i32, b: String },
    #[rune(constructor)]
    NamedOne {
        #[rune(get)]
        a: i32,
        b: String,
    },
    #[rune(constructor)]
    NamedTwo {
        #[rune(get)]
        a: i32,
        #[rune(get)]
        b: String,
    },
    #[rune(constructor)]
    UnnamedZero(i32, String),
    #[rune(constructor)]
    UnnamedOne(#[rune(get)] i32, String),
    #[rune(constructor)]
    UnnamedTwo(#[rune(get)] i32, #[rune(get)] String),
}

#[test]
fn module() {
    let mut m = Module::new();
    m.ty::<Empty>().unwrap();
    m.ty::<EmptyNamed>().unwrap();
    m.ty::<EmptyUnnamed>().unwrap();

    m.ty::<NamedZero>().unwrap();
    m.ty::<NamedOne>().unwrap();
    m.ty::<NamedTwo>().unwrap();

    m.ty::<UnnamedZero>().unwrap();
    m.ty::<UnnamedOne>().unwrap();
    m.ty::<UnnamedTwo>().unwrap();

    m.ty::<Enum>().unwrap();

    let mut c = Context::new();
    c.install(m).unwrap();
}
