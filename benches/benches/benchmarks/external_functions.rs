//! Benchmark external function calls.

use criterion::Criterion;

criterion::criterion_group!(benches, external_functions);

fn external_functions(b: &mut Criterion) {
    let mut vm1 = rune_vm! {
        fn a() {
            79
        }

        fn b(f) {
            f()
        }

        pub fn main() {
            (a, b)
        }
    };

    let mut vm2 = rune_vm! {
        pub fn main(argument) {
            let (a, b) = argument;
            assert_eq!(b(a), 79);
        }
    };

    let entry = rune::Hash::type_hash(["main"]);

    b.bench_function("external_functions", |b| {
        let output = vm1.call(entry, ()).expect("failed to fetch function");

        b.iter(|| vm2.call(entry, (output.clone(),)).expect("failed call"));
    });
}
