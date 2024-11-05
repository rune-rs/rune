use super::Slots;

macro_rules! slab_eq {
    ($slab:expr, $expected:expr) => {{
        let expected: &[usize] = &$expected[..];

        if !$slab.iter().eq(expected.iter().copied()) {
            panic!("{:?} != {:?}", $slab, expected);
        }
    }};
}

#[test]
fn iter() {
    let mut slab = Slots::new();

    assert_eq!(slab.insert(), Ok(0));
    assert_eq!(slab.insert(), Ok(1));
    assert_eq!(slab.insert(), Ok(2));
    assert_eq!(slab.insert(), Ok(3));
    assert_eq!(slab.insert(), Ok(4));
    slab_eq!(slab, [0, 1, 2, 3, 4]);

    assert_eq!(slab.remove(2), true);
    slab_eq!(slab, [0, 1, 3, 4]);

    assert_eq!(slab.remove(3), true);
    slab_eq!(slab, [0, 1, 4]);

    assert_eq!(slab.remove(0), true);
    slab_eq!(slab, [1, 4]);

    assert_eq!(slab.remove(1), true);
    slab_eq!(slab, [4]);

    assert_eq!(slab.remove(4), true);
    slab_eq!(slab, []);

    assert_eq!(slab.insert(), Ok(0));
}

#[test]
fn insert() {
    let mut slab = Slots::new();
    assert_eq!(slab.insert(), Ok(0));
    assert_eq!(slab.insert(), Ok(1));
    assert_eq!(slab.insert(), Ok(2));
    assert_eq!(slab.remove(1), true);
    assert_eq!(slab.remove(1), false);
    assert_eq!(slab.insert(), Ok(1));
    assert_eq!(slab.insert(), Ok(3));
    assert_eq!(slab.insert(), Ok(4));
}

#[test]
fn insert_boundary() {
    let mut slab = Slots::new();

    for n in 0..167 {
        assert_eq!(slab.push(), Ok(n));
    }

    for n in 167..1024 {
        assert_eq!(slab.insert(), Ok(n));
    }

    for n in 128..256 {
        assert!(slab.remove(n));
    }

    assert_eq!(slab.push(), Ok(1024));
    assert_eq!(slab.push(), Ok(1025));

    for n in (128..256).chain(1026..2047) {
        assert_eq!(slab.insert(), Ok(n));
    }

    for n in 2047..3000 {
        assert_eq!(slab.push(), Ok(n));
    }
}

#[test]
fn push() {
    let mut slab = Slots::new();
    assert_eq!(slab.insert(), Ok(0));
    assert_eq!(slab.push(), Ok(1));
    assert_eq!(slab.push(), Ok(2));
    assert_eq!(slab.remove(0), true);
    assert_eq!(slab.remove(0), false);
    assert_eq!(slab.insert(), Ok(0));
    assert_eq!(slab.insert(), Ok(3));
    assert_eq!(slab.remove(2), true);
    assert_eq!(slab.remove(0), true);
    assert_eq!(slab.insert(), Ok(0));
    assert_eq!(slab.insert(), Ok(2));
    assert_eq!(slab.insert(), Ok(4));
    assert_eq!(slab.push(), Ok(5));
}

#[test]
fn push_tail_hole() {
    let mut slab = Slots::new();
    assert_eq!(slab.insert(), Ok(0));
    assert_eq!(slab.insert(), Ok(1));
    assert_eq!(slab.insert(), Ok(2));

    assert_eq!(slab.remove(1), true);
    assert_eq!(slab.remove(2), true);
    assert_eq!(slab.remove(2), false);

    assert_eq!(slab.push(), Ok(1));
    assert_eq!(slab.push(), Ok(2));
}

#[test]
fn push_pop() {
    let mut slab = Slots::new();
    assert_eq!(slab.insert(), Ok(0));
    assert_eq!(slab.insert(), Ok(1));
    assert_eq!(slab.insert(), Ok(2));
    assert_eq!(slab.remove(1), true);

    assert_eq!(slab.push(), Ok(3));
    assert_eq!(slab.push(), Ok(4));
    assert_eq!(slab.push(), Ok(5));
    assert_eq!(slab.insert(), Ok(1));

    assert_eq!(slab.remove(2), true);

    assert_eq!(slab.remove(5), true);
    assert_eq!(slab.remove(4), true);
    assert_eq!(slab.remove(3), true);
    assert_eq!(slab.remove(1), true);
    assert_eq!(slab.remove(0), true);
    assert_eq!(slab.remove(0), false);
}

#[test]
fn bad_test() {
    let mut slab = Slots::new();
    assert_eq!(slab.insert(), Ok(0));
    assert_eq!(slab.insert(), Ok(1));
    assert_eq!(slab.insert(), Ok(2));
    assert_eq!(slab.insert(), Ok(3));

    assert_eq!(slab.remove(2), true);
    assert_eq!(slab.remove(3), true);

    assert_eq!(slab.insert(), Ok(2));
}

#[test]
fn bug1() {
    let mut slab = Slots::new();
    assert_eq!(slab.insert(), Ok(0));
    assert_eq!(slab.insert(), Ok(1));
    assert_eq!(slab.insert(), Ok(2));

    assert_eq!(slab.remove(2), true);
    assert_eq!(slab.remove(1), true);

    assert_eq!(slab.insert(), Ok(1));
}

#[test]
fn push_first() {
    let mut slab = Slots::new();
    assert_eq!(slab.push(), Ok(0));
    assert_eq!(slab.insert(), Ok(1));
    assert_eq!(slab.push(), Ok(2));
}

#[test]
fn test_bug() {
    let mut slab = Slots::new();
    assert_eq!(slab.insert(), Ok(0));
    assert_eq!(slab.remove(0), true);
    assert_eq!(slab.push(), Ok(0));
    assert_eq!(slab.insert(), Ok(1));
    assert_eq!(slab.push(), Ok(2));
    assert_eq!(slab.remove(2), true);
    assert_eq!(slab.insert(), Ok(2));
    assert_eq!(slab.remove(2), true);
    assert_eq!(slab.remove(0), true);
    assert_eq!(slab.remove(0), false);
}
