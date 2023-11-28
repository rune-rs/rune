prelude!();

use ErrorKind::*;

#[test]
fn test_continue_label() {
    rune! {
        pub fn main() {
            let n = 0;
            let not_used = true;

            'outer:
            loop {
                // NB: contaminate the local scope a bit to make sure it's
                // properly cleaned up.
                let local1 = not_used;
                n += 1;
                let local2 = not_used;

                while local1 && local2 {
                    not_used = false;
                    continue 'outer;
                }

                break;
            }

            assert_eq!(n, 2);
        }
    };
}

#[test]
fn while_continue() {
    rune! {
        pub fn main() {
            let n = 0;
            let condition = true;

            while n < 10 && n > -10 {
                let a = condition;
                let b = !condition;

                if a {
                    n += 1;
                    continue;
                }

                n -= 1;
            }

            assert_eq!(n, 10);
        }
    };
}

#[test]
fn loop_continue() {
    rune! {
        pub fn main() {
            let n = 0;
            let condition = true;

            loop {
                let a = n < 10 && n > -10;

                if !a {
                    break;
                }

                let a = condition;

                if a {
                    n += 1;
                    continue;
                }

                n -= 1;
            }

            assert_eq!(n, 10);
        }
    };
}

#[test]
fn for_continue() {
    rune! {
        pub fn main() {
            let n = 0;
            let condition = true;

            for ign in 0..10 {
                let a = condition;

                if a {
                    n += 1;
                    continue;
                }

                break;
            }

            assert_eq!(n, 10);
        }
    };
}

#[test]
fn test_continue_not_in_loop() {
    assert_errors! {
        r#"pub fn main() { continue }"#,
        span!(16, 24), ContinueOutsideOfLoop
    };
}

#[test]
fn test_continue_missing_label() {
    assert_errors! {
        r#"pub fn main() { 'existing: loop { loop { continue 'missing; } } }"#,
        span!(41, 58), MissingLoopLabel { label } => {
            assert_eq!(&*label, "missing");
        }
    };
}
