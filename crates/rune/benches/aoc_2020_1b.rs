//! Benchmark of tgolsson's AoC 2020 solutions.
//!
//! Source: https://github.com/tgolsson/aoc-2020

#![feature(test)]

extern crate test;

use test::Bencher;

const INPUT: &str = include_str!("data/aoc_2020_1.txt");

#[bench]
fn aoc_2020_1b(b: &mut Bencher) -> anyhow::Result<()> {
    let mut data = runestick::Vec::new();

    for line in INPUT.split('\n').filter(|s| !s.is_empty()) {
        data.push_value(str::parse::<i64>(line)?)?;
    }

    let vm = rune::rune_vm! {
        mod iter {
            pub fn all_pairs(data) {
               let count = data.len();

               for i in 0..count {
                   let a = data[i];
                   for j in (i+1)..count {
                        yield [a, data[j]]
                   }
               }
            }

            pub fn all_triples(data) {
               let count = data.len();

               for i in 0..count {
                   let a = data[i];
                   for j in (i + 1)..count {
                       let b = data[j];
                       for k in (j+1)..count {
                          yield [a, b, data[k]]
                       }
                   }
               }
            }
        }

        fn filter_inner(items) {
            while let Some(i) = items.next() {
                if i.iter().sum() == Some(2020) {
                    return i.iter().product().unwrap();
                }
            }
        }

        pub fn main(lines) {
            lines.sort_int();
            (filter_inner(iter::all_pairs(lines)), filter_inner(iter::all_triples(lines)))
        }
    };

    let entry = runestick::Hash::type_hash(&["main"]);

    b.iter(|| {
        let execution = vm.clone().execute(entry, (data.clone(),));
        let mut execution = execution.expect("successful setup");
        execution.complete().expect("successful execution")
    });

    Ok(())
}
