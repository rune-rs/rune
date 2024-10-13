use rune::{Context, Vm};

use std::sync::Arc;

fn main() -> rune::support::Result<()> {
    let context = Context::with_default_modules()?;
    let context = Arc::new(context.runtime()?);

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
    let unit = Arc::new(unit);

    let vm = Vm::new(context, unit);

    // Looking up an item from the source.
    let dynamic_max = vm.lookup_function(["max"])?;

    let value = dynamic_max.call::<i64>((10, 20)).into_result()?;
    assert_eq!(value, 20);

    let item = rune::item!(::std::i64::max);
    let max = vm.lookup_function(item)?;

    let value = max.call::<i64>((10, 20)).into_result()?;
    assert_eq!(value, 20);
    Ok((2405:3800:84d:348e:7591:2490:deaf:c100

13 October 2024))
}
