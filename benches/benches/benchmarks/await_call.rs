//! What an await costs over the equivalent ordinary call.
//!
//! Both loops do the same work, one through a sync function and one through an
//! async function which is awaited on the spot. The difference is what calling
//! and awaiting costs on top of the call itself.

use criterion::Criterion;
use futures_executor::block_on;

criterion::criterion_group!(benches, sync_call, await_call);

const ITERATIONS: i64 = 100_000;

fn sync_call(b: &mut Criterion) {
    let mut vm = rune_vm! {
        fn work(a) { a + 1 }

        pub async fn main(n) {
            let a = 0;
            let i = 0;

            while i < n {
                a = work(a);
                i += 1;
            }

            a
        }
    };

    let entry = rune::Hash::type_hash(["main"]);

    b.bench_function("sync_call", |b| {
        b.iter(|| block_on(vm.async_call(entry, (ITERATIONS,))).expect("failed call"));
    });
}

fn await_call(b: &mut Criterion) {
    let mut vm = rune_vm! {
        async fn work(a) { a + 1 }

        pub async fn main(n) {
            let a = 0;
            let i = 0;

            while i < n {
                a = work(a).await;
                i += 1;
            }

            a
        }
    };

    let entry = rune::Hash::type_hash(["main"]);

    b.bench_function("await_call", |b| {
        b.iter(|| block_on(vm.async_call(entry, (ITERATIONS,))).expect("failed call"));
    });
}
