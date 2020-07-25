use anyhow::Result;

use st_frontend::Encode as _;
use st_frontend_rune::{ast, parse_all};

/// Helper function to run the specified program to completion.
pub async fn run_program<'a, A, T>(source: &str, name: &str, args: A) -> Result<T>
where
    A: st::IntoArgs,
    T: st::ReflectFromValue,
{
    let mut vm = st::Vm::new();
    assert_eq!(vm.stack.len(), 0);

    let unit = parse_all::<ast::File>(&source)?;
    let unit = unit.encode()?;

    let functions = st::Functions::new();

    let task: st::Task<T> = vm.call_function(&functions, &unit, name, args)?;

    let output = task.run_to_completion().await?;

    // assert_eq!(vm.stack.len(), A::count());
    Ok(output)
}
