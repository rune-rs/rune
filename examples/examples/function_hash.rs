use rune::compile::Item;
use rune::Hash;

fn main() {
    println!("{}", Hash::type_hash(&Item::with_item(&["Foo", "new"])));
    println!("{}", Hash::type_hash(&["Foo", "new"]));
}
