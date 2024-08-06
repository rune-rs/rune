use rune::{Hash, ItemBuf};

fn main() -> rune::support::Result<()> {
    println!("{}", Hash::type_hash(&ItemBuf::with_item(["Foo", "new"])?));
    println!("{}", Hash::type_hash(["Foo", "new"]));
    Ok(())
}
