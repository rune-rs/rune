//! Benchmark of udoprog's AoC 2020 solutions.
//!
//! Source: https://github.com/udoprog/aoc2020

use anyhow::Context;
use criterion::Criterion;
use rune::alloc::prelude::*;

criterion::criterion_group!(benches, aoc_2020_1a);

const INPUT: &str = include_str!("data/aoc_2020_1.txt");

fn aoc_2020_1a(b: &mut Criterion) {
    let mut data = rune::runtime::Vec::new();

    for line in INPUT
        .split('\n')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        data.push_value(
            str::parse::<i64>(line)
                .with_context(|| line.to_string())
                .expect("invalid number"),
        )
        .unwrap();
    }

    let mut vm = rune_vm! {
        use std::string;

        struct NoSolution;

        fn part1(v, target) {
            v.sort();

            let a = 0;
            let b = v.len() - 1;

            while a != b {
                match v[a] + v[b] {
                    n if n < target => a += 1,
                    n if n > target => b -= 1,
                    _ => return Ok((a, b))
                }
            }

            Err(NoSolution)
        }

        fn part2(v, target) {
            v.sort();

            let a = 0;
            let c = v.len() - 1;

            while a != c {
                if v[a] + v[c] < target {
                    for c in (a..c).iter().rev() {
                        for b in a + 1..c {
                            match v[a] + v[b] + v[c] {
                                n if n < target => (),
                                n if n > target => break,
                                _ => return Ok((a, b, c)),
                            }
                        }
                    }

                    a += 1;
                } else {
                    c -= 1;
                }
            }

            Err(NoSolution)
        }

        pub fn main(v) {
            let (a, b, c) = part2(v, 2020)?;
            assert_eq!((a, b, c), (0, 3, 19));
            assert_eq!(v[a] + v[b] + v[c], 2020);
            assert_eq!(v[a] * v[b] * v[c], 49880012);
            Ok(())
        }
    };

    let entry = rune::Hash::type_hash(["main"]);

    b.bench_function("aoc_2020_1a", |b| {
        b.iter(|| {
            vm.call(entry, (data.try_clone().unwrap(),))
                .expect("failed call")
        });
    });
}
