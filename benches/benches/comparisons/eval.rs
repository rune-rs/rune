use criterion::Criterion;
use rune::Hash;

criterion::criterion_group!(benches, entry);

fn entry(b: &mut Criterion) {
    let mut group = b.benchmark_group("eval");

    group.bench_function("rhai", |b| {
        let ast = rhai_ast! {
            None {
                let x = #{
                    a: 1,
                    b: 2.345,
                    c:"hello",
                    d: true,
                    e: #{ x: 42, "y$@#%": (), z: [ 1, 2, 3, #{}, #{ "hey": "jude" }]}
                };

                x["e"].z[4].hey
            }
        };

        b.iter(|| {
            let out = ast.eval::<String>();
            assert_eq!(out, "jude");
            out
        });
    });

    group.bench_function("rune", |b| {
        let mut vm = rune_vm! {
            let x = #{
                a: 1,
                b: 2.345,
                c: "hello",
                d: true,
                e: #{ x: 42, "y$@#%": (), z: [ 1, 2, 3, #{}, #{ "hey": "jude" }]}
            };

            x["e"].z[4].hey
        };

        b.iter(|| {
            let value = vm.call(Hash::EMPTY, ())?;
            let value = rune::from_value::<String>(value)?;
            assert_eq!(value, "jude");
            Ok::<_, rune::runtime::RuntimeError>(value)
        })
    });
}
