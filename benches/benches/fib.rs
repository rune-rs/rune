#![feature(test)]

extern crate test;

use test::Bencher;

#[bench]
fn fib_15(b: &mut Bencher) -> rune::Result<()> {
    let mut vm = rune_tests::rune_vm! {
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

    let entry = rune::Hash::type_hash(&["main"]);

    b.iter(|| vm.call(entry, (15,)).expect("successful execution"));
    Ok(())
}

#[bench]
fn fib_20(b: &mut Bencher) -> rune::Result<()> {
    let mut vm = rune_tests::rune_vm! {
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

    let entry = rune::Hash::type_hash(&["main"]);

    b.iter(|| vm.call(entry, (20,)).expect("successful execution"));
    Ok(())
}
