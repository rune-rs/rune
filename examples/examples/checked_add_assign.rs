use runestick::{Any, Context, Module, VmError, VmErrorKind};
use std::sync::Arc;

#[derive(Any)]
struct External {
    #[rune(add_assign = "External::value_add_assign")]
    value: i64,
}

impl External {
    fn value_add_assign(&mut self, other: i64) -> Result<(), VmError> {
        self.value = self.value.checked_add(other).ok_or(VmErrorKind::Overflow)?;

        Ok(())
    }
}

fn main() -> runestick::Result<()> {
    let mut module = Module::default();
    module.ty::<External>()?;

    let mut context = Context::default();
    context.install(&module)?;
    let context = Arc::new(context);

    let external = External {
        value: i64::max_value(),
    };

    let result = rune_tests::run::<_, _, ()>(
        &context,
        "pub fn main(external) { external.value += 1; }",
        &["main"],
        (external,),
    );

    let error = result.expect_err("expected error");
    let error = error.expect_vm_error("expected vm error");
    println!("Error: {}", error);
    Ok(())
}
