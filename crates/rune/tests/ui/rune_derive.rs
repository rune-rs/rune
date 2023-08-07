use rune::Any;

// Generates a warning that the path should no longer be using a string literal.
#[derive(Any)]
#[rune_derive(ADD, STRING_DISPLAY, STRING_DEBUG)]
struct UnimplementedTraits;

#[derive(Any)]
#[rune_derive(NON_EXISTENT)]
struct NonExistingProtocol;

fn main() {
}
