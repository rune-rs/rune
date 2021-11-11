use rune::{Hash, Item};

fn main() {
    println!("{}", Hash::type_hash(&Item::with_item(&["Foo", "new"])));
    println!("{}", Hash::type_hash(&["Foo", "new"]));
}
