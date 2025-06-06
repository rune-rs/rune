use rune::sync::Arc;
use rune::{Context, Vm};

fn main() -> rune::support::Result<()> {
    let context = Context::with_default_modules()?;
    let context = Arc::try_new(context.runtime()?)?;

    let mut sources = rune::sources! {
        entry => {
            pub fn max(a, b) {
                if a > b {
                    a
                } else {
                    b
                }
            }
        }
    };

    let unit = rune::prepare(&mut sources).build()?;
    let unit = Arc::try_new(unit)?;
    let vm = Vm::new(context, unit);

    // Looking up an item from the source.
    let dynamic_max = vm.lookup_function(["max"])?;

    let value = dynamic_max.call::<i64>((10, 20))?;
    assert_eq!(value, 20);

    let item = rune::item!(::std::i64::max);
    let max = vm.lookup_function(item)?;

    let value = max.call::<i64>((10, 20))?;
    assert_eq!(value, 20);
    Ok(())
}
