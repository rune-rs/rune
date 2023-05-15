#![no_std]
#![feature(alloc_error_handler, start, core_intrinsics, lang_items, link_cfg)]

extern crate alloc;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[cfg(all(windows, target_env = "msvc"))]
#[link(name = "msvcrt")]
extern "C" {}

#[alloc_error_handler]
fn err_handler(_: core::alloc::Layout) -> ! {
    core::intrinsics::abort();
}

#[panic_handler]
#[lang = "panic_impl"]
extern "C" fn rust_begin_panic(_: &core::panic::PanicInfo) -> ! {
    core::intrinsics::abort();
}

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

use alloc::sync::Arc;
use core::mem::replace;

use rune::no_std::RawEnv;
use rune::{Diagnostics, Vm};

static mut BUDGET: usize = usize::MAX;
static mut RAW_ENV: RawEnv = RawEnv::null();

/// Necessary hook to abort the current process.
#[no_mangle]
extern "C" fn __rune_abort() -> ! {
    core::intrinsics::abort()
}

#[no_mangle]
extern "C" fn __rune_budget_take() -> bool {
    // SAFETY: this is only ever executed in a singlethreaded environment.
    unsafe {
        if BUDGET == usize::MAX {
            return true;
        } else {
            BUDGET = BUDGET.saturating_sub(1);
            BUDGET != 0
        }
    }
}

#[no_mangle]
extern "C" fn __rune_budget_replace(value: usize) -> usize {
    // SAFETY: this is only ever executed in a singlethreaded environment.
    unsafe { replace(&mut BUDGET, value) }
}

#[no_mangle]
extern "C" fn __rune_budget_get() -> usize {
    // SAFETY: this is only ever executed in a singlethreaded environment.
    unsafe { BUDGET }
}

#[no_mangle]
extern "C" fn __rune_env_get() -> RawEnv {
    // SAFETY: this is only ever executed in a singlethreaded environment.
    unsafe { RAW_ENV }
}

#[no_mangle]
extern "C" fn __rune_env_replace(env: RawEnv) -> RawEnv {
    // SAFETY: this is only ever executed in a singlethreaded environment.
    unsafe { replace(&mut RAW_ENV, env) }
}

#[start]
fn main(_argc: isize, _argv: *const *const u8) -> isize {
    match inner_main() {
        Ok(output) => output as isize,
        Err(..) => -1,
    }
}

fn inner_main() -> rune::Result<i32> {
    let context = rune::Context::with_default_modules()?;

    let mut sources = rune::sources!(
        entry => {
            pub fn main(number) {
                number + 10
            }
        }
    );

    let mut diagnostics = Diagnostics::new();

    let result = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build();

    let unit = result?;

    let mut vm = Vm::new(Arc::new(context.runtime()), Arc::new(unit));
    let output = vm.execute(["main"], (33i64,))?.complete().into_result()?;
    let output: i32 = rune::from_value(output)?;
    Ok(output)
}
