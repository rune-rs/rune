prelude!();

use VmErrorKind::*;

#[test]
fn test_range_iter() {
    let values: Vec<i64> = rune! {
        use std::iter::range;

        range(0, 100).map(|n| n * 2).filter(|n| n % 3 == 0).collect::<Vec>()
    };

    assert_eq!(
        values,
        (0..100)
            .map(|n| n * 2)
            .filter(|n| n % 3 == 0)
            .collect::<Vec<i64>>()
    );
}

#[test]
fn test_rev() {
    let values: Vec<i64> = rune! {
        use std::iter::range;

        range(0, 100).map(|n| n * 2).filter(|n| n % 3 == 0).rev().collect::<Vec>()
    };

    let expected = (0..100)
        .map(|n| n * 2)
        .filter(|n| n % 3 == 0)
        .rev()
        .collect::<Vec<i64>>();

    assert_eq!(values, expected);
}

#[test]
fn test_next_back() {
    let _: () = rune! {
        const SOURCE = [1, 2, 3, "foo"];

        let it = SOURCE.iter().rev();
        let v = Vec::new();

        while let Some(n) = it.next_back() {
            v.push(n);
        }

        assert_eq!(v, SOURCE);
    };
}

#[test]
fn test_object_rev_error() {
    assert_vm_error!(
        "#{}.iter().rev()",
        MissingInstanceFunction { hash: rune::hash!(::std::object::Iter.rev), .. } => {
        }
    );
}

#[test]
fn test_chain() {
    let values: Vec<i64> = rune! {
        [1, 2].iter().rev().chain([3, 4].iter()).collect::<Vec>()
    };

    assert_eq!(values, vec![2, 1, 3, 4])
}

#[test]
fn test_enumerate() {
    let values: Vec<(i64, i64)> = rune! {
        let it = [1, 2].iter().rev().chain([3, 4].iter()).enumerate();
        assert_eq!(it.next_back(), Some((3, 4)));
        it.collect::<Vec>()
    };

    assert_eq!(values, vec![(0, 2), (1, 1), (2, 3)])
}

#[test]
fn test_option_iter() {
    let values: Vec<i64> = rune! {
        Some(1).iter().chain(None.iter()).chain(Some(3).iter()).collect::<Vec>()
    };

    assert_eq!(values, vec![1, 3])
}

#[test]
fn test_peekable_take() {
    let actual: Vec<i64> = rune! {
        use std::iter::range;

        let it = range(1, 100).take(40).peekable();
        let out = [];

        while let Some(n) = it.next() {
            out.push(n);

            if it.peek().is_some() {
                out.push(0);
            }
        }

        out
    };

    let mut it = (1..100).take(40).peekable();
    let mut expected = Vec::new();

    while let Some(n) = it.next() {
        expected.push(n);
        expected.extend(it.peek().map(|_| 0));
    }

    assert_eq!(actual, expected);
}

#[test]
fn test_flat_map() {
    let actual: Vec<i64> = rune! {
        use std::iter::range;

        let it = range(1, 10).flat_map(|n| range(1, n + 1));

        let out = [];

        while let (Some(a), Some(b)) = (it.next(), it.next_back()) {
            out.push(a);
            out.push(b);
        }

        out
    };

    let mut it = (1..10).flat_map(|n| 1..=n);
    let mut expected = Vec::new();

    while let (Some(a), Some(b)) = (it.next(), it.next_back()) {
        expected.push(a);
        expected.push(b);
    }

    assert_eq!(actual, expected);
}

/// An iterator over a deque reports what is left of it without consuming
/// itself, so it can be collected and adapted like every other one.
///
/// `SIZE_HINT` and `LEN` took the iterator by value, which took it out of the
/// value they were called on, so anything which asks for the size before
/// walking - `collect`, and every adapter which collects - failed.
#[test]
fn deque_iter_is_not_consumed_by_its_size() {
    let values: Vec<i64> = rune! {
        use std::collections::VecDeque;

        let deque = VecDeque::new();
        deque.push_back(1);
        deque.push_back(2);
        deque.push_front(0);
        deque.iter().collect::<Vec>()
    };

    assert_eq!(values, [0, 1, 2]);

    let values: Vec<i64> = rune! {
        use std::collections::VecDeque;

        let deque = VecDeque::new();
        deque.push_back(1);
        deque.push_back(2);
        deque.iter().map(|v| v * 2).rev().collect::<Vec>()
    };

    assert_eq!(values, [4, 2]);

    // Asking twice hands back the same answer, since neither question takes
    // anything away from the iterator.
    let values: Vec<i64> = rune! {
        use std::collections::VecDeque;

        let deque = VecDeque::new();
        deque.push_back(1);
        deque.push_back(2);

        let it = deque.iter();
        let a = it.len();
        let b = it.len();
        let c = it.next().unwrap();
        let d = it.len();
        [a, b, c, d]
    };

    assert_eq!(values, [2, 2, 1, 1]);
}
