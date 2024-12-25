#![no_std]
#![no_main]
#![feature(alloc_error_handler, core_intrinsics, lang_items, link_cfg)]
#![allow(internal_features)]

extern crate alloc;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[cfg(all(windows, target_env = "msvc"))]
#[link(name = "msvcrt")]
extern "C" {}

#[cfg(unix)]
#[link(name = "c")]
extern "C" {}

#[alloc_error_handler]
fn err_handler(_: core::alloc::Layout) -> ! {
    core::intrinsics::abort();
}

#[panic_handler]
#[lang = "panic_impl"]
fn rust_begin_panic(_: &core::panic::PanicInfo) -> ! {
    core::intrinsics::abort();
}

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

#[cfg(unix)]
#[no_mangle]
pub extern "C" fn _Unwind_Resume() {}

use core::ffi::c_int;

use alloc::sync::Arc;

use rune::{Diagnostics, Vm};

rune::no_std::static_env!();

use critical_section::RawRestoreState;

struct MyCriticalSection;

critical_section::set_impl!(MyCriticalSection);

// SAFETY: In this instance, we might assume that the application is
// single-threaded. So no critical sections are used in practice. Therefore a
// dummy implementation is used.
unsafe impl critical_section::Impl for MyCriticalSection {
    unsafe fn acquire() -> RawRestoreState {}
    unsafe fn release(_: RawRestoreState) {}
}

#[no_mangle]
extern "C" fn main(_argc: c_int, _argv: *const *const u8) -> c_int {
    match inner_main() {
        Ok(output) => output as c_int,
        Err(..) => -1,
    }
}

fn inner_main() -> rune::support::Result<i32> {
    let context = rune::Context::with_default_modules()?;

    let mut sources = rune::sources! {
        entry => {
            pub fn main(number) {
                number + 10
            }
        }
    };

    let mut diagnostics = Diagnostics::new();

    let result = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build();

    let unit = result?;

    let mut vm = Vm::new(Arc::new(context.runtime()?), Arc::new(unit));
    let output = vm.execute(["main"], (33i64,))?.complete().into_result()?;
    let output: i32 = rune::from_value(output)?;
    Ok((output != 43).into())
}
