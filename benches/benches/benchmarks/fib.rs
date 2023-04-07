use criterion::Criterion;

criterion::criterion_group!(benches, fib_15, fib_20);

fn fib_15(b: &mut Criterion) {
    let mut vm = rune_vm! {
        fn fib(n) {
            if n <= 1 {
                n
            } else {
                fib(n - 2) + fib(n - 1)
            }
        }

        pub fn main(v) {
            fib(v)
        }
    };

    let entry = rune::Hash::type_hash(["main"]);

    b.bench_function("fib_15", |b| {
        b.iter(|| vm.call(entry, (15,)).expect("failed call"));
    });
}

fn fib_20(b: &mut Criterion) {
    let mut vm = rune_vm! {
        fn fib(n) {
            if n <= 1 {
                n
            } else {
                fib(n - 2) + fib(n - 1)
            }
        }

        pub fn main(v) {
            fib(v)
        }
    };

    let entry = rune::Hash::type_hash(["main"]);

    b.bench_function("fib_20", |b| {
        b.iter(|| vm.call(entry, (20,)).expect("failed call"));
    });
}
