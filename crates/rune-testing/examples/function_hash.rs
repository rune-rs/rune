use runestick::{Hash, Item};

fn main() {
    println!("{}", Hash::type_hash(Item::of(&["Foo", "new"])));
}
