//! Benchmark of udoprog's AoC 2020 solutions.
//!
//! Source: https://github.com/udoprog/aoc2020

#![feature(test)]

extern crate test;

use test::Bencher;

const INPUT: &str = include_str!("data/aoc_2020_1.txt");

#[bench]
fn aoc_2020_1a(b: &mut Bencher) -> runestick::Result<()> {
    let mut data = runestick::Vec::new();

    for line in INPUT.split('\n').filter(|s| !s.is_empty()) {
        data.push_value(str::parse::<i64>(line)?)?;
    }

    let mut vm = rune_tests::rune_vm! {
        use std::string;

        struct NoSolution;

        fn part1(v, target) {
            v.sort_int();

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
            v.sort_int();

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

    let entry = runestick::Hash::type_hash(&["main"]);

    b.iter(|| {
        let execution = vm.execute(entry, (data.clone(),));
        let mut execution = execution.expect("successful setup");
        execution.complete().expect("successful execution")
    });

    Ok(())
}
