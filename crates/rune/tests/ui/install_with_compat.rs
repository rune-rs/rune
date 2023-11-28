use rune::Any;

// Generates a warning that the path should no longer be using a string literal.
#[derive(Any)]
#[rune(install_with = "foo::bar")]
struct Struct {}

fn main() {}
