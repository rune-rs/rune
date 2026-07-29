//! Tests for the modifiers a block carries.
//!
//! A modifier is written in front of the expression a block belongs to, so it
//! has to survive the chain which is built around that block when one is
//! applied to it. As in Rust, `async { .. }.await` is the same block as
//! `async { .. }`, and `const { .. }` is const wherever it appears.

prelude!();

use ErrorKind::*;

#[test]
fn async_block() {
    let out: i64 = eval(
        r#"
        let block = async { 1 };
        block.await
        "#,
    );

    assert_eq!(out, 1);
}

/// The block is chained into, so the `async` in front of it has a chain
/// between it and the block it applies to.
#[test]
fn async_block_awaited_in_place() {
    let out: i64 = eval(r#"async { 1 }.await"#);

    assert_eq!(out, 1);
}

#[test]
fn async_block_chained_twice() {
    let out: i64 = eval(r#"async { async { 1 } }.await.await"#);

    assert_eq!(out, 1);
}

/// A moved variable is only reported if the `move` reached the block, so this
/// is what tells the chained block apart from a plain one.
#[test]
fn async_move_block_chained_moves() {
    assert_errors!(
        r#"
        pub async fn main() {
            let o = [];
            let a = async move {
                o.push(42);
                o
            }.await;

            o.push(42);
            a
        }
        "#,
        span!(168, 169),
        VariableMoved {
            moved_at: span!(86, 147)
        }
    )
}

#[test]
fn const_block() {
    let out: i64 = eval(
        r#"
        let value = const { 1 + 2 };
        value
        "#,
    );

    assert_eq!(out, 3);
}

#[test]
fn const_block_chained() {
    let out: String = eval(r#"const { 1 + 2 }.to_string()"#);

    assert_eq!(out, "3");
}

/// A block nested in one which carries a modifier is a plain block, since the
/// modifier was consumed by the block it was written for.
#[test]
fn nested_block_is_plain() {
    let out: i64 = eval(r#"async { { 1 } }.await"#);

    assert_eq!(out, 1);
}

#[test]
fn async_closure() {
    let out: i64 = eval(
        r#"
        let f = async || 1;
        f().await
        "#,
    );

    assert_eq!(out, 1);
}

#[test]
fn async_move_closure_moves() {
    assert_errors!(
        r#"
        pub async fn main() {
            let o = [];
            let a = async move || {
                o.push(42);
                o
            };

            o.push(42);
            a().await
        }
        "#,
        span!(165, 166),
        VariableMoved {
            moved_at: span!(75, 150)
        }
    )
}

/// A block which can be broken out of converges at its label whatever its body
/// did, since `break` is a jump to that label.
///
/// Reporting the block as diverging left the label at the end of what had been
/// assembled, so breaking out of it ran off the end of the instructions.
#[test]
fn breaking_out_of_a_block_which_always_breaks() {
    let out: i64 = eval(r#"let a = 'l: { break 'l 1; }; a"#);
    assert_eq!(out, 1);

    // What follows the block is still reached.
    let out: i64 = eval(r#"let a = 'l: { break 'l 1; }; 2"#);
    assert_eq!(out, 2);

    // The value the block breaks with is what it evaluates to, including when
    // it is bound and then written over.
    let out: (i64, i64) = eval(r#"let v = 5; let a = 'l: { break 'l v; }; a = 9; (a, v)"#);
    assert_eq!(out, (9, 5));

    // Without a value, and with statements which follow the break.
    let out: i64 = eval(r#"'l: { break 'l; }; 3"#);
    assert_eq!(out, 3);

    let out: i64 = eval(r#"let a = 'l: { let b = 1; break 'l b; 2 }; a"#);
    assert_eq!(out, 1);

    // Inside a function, where the return which follows must still be
    // assembled.
    let out: i64 = eval(r#"fn f() { 'l: { break 'l 1; } } f()"#);
    assert_eq!(out, 1);

    // Nested, where the inner block breaks out of the outer one.
    let out: i64 = eval(r#"let a = 'a: { 'b: { break 'a 1; } 2 }; a"#);
    assert_eq!(out, 1);
}
