use criterion::Criterion;
use rune::Hash;

criterion::criterion_group!(benches, entry);

fn entry(b: &mut Criterion) {
    let mut group = b.benchmark_group("primes");

    group.bench_function("rhai", |b| {
        let ast = rhai_ast! {
            const MAX_NUMBER_TO_CHECK = 10_000;

            let prime_mask = [];
            prime_mask.pad(MAX_NUMBER_TO_CHECK, true);

            prime_mask[0] = false;
            prime_mask[1] = false;

            let total_primes_found = 0;

            for p in 2..MAX_NUMBER_TO_CHECK {
                if prime_mask[p] {
                    total_primes_found += 1;
                    let i = 2 * p;

                    while i < MAX_NUMBER_TO_CHECK {
                        prime_mask[i] = false;
                        i += p;
                    }
                }
            }

            total_primes_found
        };

        b.iter(|| {
            let value = ast.eval::<i64>();
            assert_eq!(value, 1229);
            value
        });
    });

    group.bench_function("rune", |b| {
        let mut vm = rune_vm! {
            const MAX_NUMBER_TO_CHECK = 10_000;

            let prime_mask = [];
            prime_mask.resize(MAX_NUMBER_TO_CHECK, true);

            prime_mask[0] = false;
            prime_mask[1] = false;

            let total_primes_found = 0;

            for p in 2..MAX_NUMBER_TO_CHECK {
                if prime_mask[p] {
                    total_primes_found += 1;
                    let i = 2 * p;

                    while i < MAX_NUMBER_TO_CHECK {
                        prime_mask[i] = false;
                        i += p;
                    }
                }
            }

            total_primes_found
        };

        b.iter(|| {
            let value = vm.call(Hash::EMPTY, ()).unwrap();
            let value: i64 = rune::from_value(value).unwrap();
            assert_eq!(value, 1229);
            value
        })
    });
}
