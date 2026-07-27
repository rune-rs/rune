//! Sharing state between virtual machines on different threads through a
//! `static`.
//!
//! Static storage cannot cross a thread boundary, so every thread builds its
//! own [`Globals`] out of the same [`Unit`]. What makes the state shared is the
//! *value* placed in the slot: a host type whose interior is an
//! `Arc<Mutex<..>>`, cloned once per thread.

use std::sync::{Arc as StdArc, Mutex};
use std::thread;

use rune::runtime::Globals;
use rune::sync::Arc;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Any, ContextError, Diagnostics, Module, Vm};

/// A counter shared by every virtual machine, on every thread.
///
/// Cloning it produces another handle to the same interior, which is what makes
/// it usable from more than one thread at a time.
#[derive(Debug, Clone, Any)]
struct Counter {
    inner: StdArc<Mutex<i64>>,
}

impl Counter {
    /// Bump the counter and return its new value.
    #[rune::function]
    fn increment(&self) -> i64 {
        let mut count = self.inner.lock().unwrap();
        *count += 1;
        *count
    }
}

fn module() -> Result<Module, ContextError> {
    let mut m = Module::new();
    m.ty::<Counter>()?;
    m.function_meta(Counter::increment)?;
    Ok(m)
}

const THREADS: usize = 4;
const CALLS: usize = 1000;

fn main() -> rune::support::Result<()> {
    let mut context = rune_modules::default_context()?;
    context.install(module()?)?;
    let runtime = Arc::try_new(context.runtime()?)?;

    let mut sources = rune::sources!(
        entry => {
            static COUNTER;

            pub fn main() {
                COUNTER.increment()
            }
        }
    );

    let mut diagnostics = Diagnostics::new();

    let result = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        diagnostics.emit(&mut writer, &sources)?;
    }

    // The unit and the runtime are `Send + Sync`, so they are compiled once and
    // shared. The storage and the virtual machines are not, so they are built
    // per thread.
    let unit = Arc::try_new(result?)?;

    let counter = Counter {
        inner: StdArc::new(Mutex::new(0)),
    };

    let mut handles = Vec::new();

    for _ in 0..THREADS {
        let unit = unit.clone();
        let runtime = runtime.clone();
        // A new handle to the same interior.
        let counter = counter.clone();

        handles.push(thread::spawn(move || -> rune::support::Result<()> {
            let globals = Globals::new(unit.clone())?;
            globals.set(["COUNTER"], rune::to_value(counter)?)?;

            let mut vm = Vm::new(runtime, unit).with_globals(globals);

            for _ in 0..CALLS {
                vm.call(["main"], ())?;
            }

            Ok(())
        }));
    }

    for handle in handles {
        handle.join().unwrap()?;
    }

    let total = *counter.inner.lock().unwrap();
    println!("total: {total}");
    Ok(())
}
