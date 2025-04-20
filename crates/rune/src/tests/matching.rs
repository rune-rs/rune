prelude!();

#[derive(Debug, Any)]
struct External {
    #[rune(get, set)]
    value: u32,
}

pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new();
    module.ty::<External>()?;
    Ok(module)
}

#[test]
#[ignore = "fix this"]
fn try_matching() -> crate::support::Result<()> {
    let m = module()?;

    let external = External { value: 1337 };

    let b: bool = rune_n! {
        mod m,
        (&external,),
        pub fn main(external) {
            match external {
                External { value: 1337 } => true,
                _ => false,
            }
        }
    };

    assert!(b);
    Ok(())
}
