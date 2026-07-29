//! Awaiting an async call which has yet to run does not drive it as a machine
//! of its own.
//!
//! A future produced by an async call holds nothing but the arguments of that
//! call until something polls it. It is not a suspended computation, it is a
//! call which has not happened, so the machine which awaits it takes it over -
//! its values move to the top of the awaiting stack and an ordinary call frame
//! opens over them. `bounded_recursion` covers what that buys; these cover the
//! cases where the future is *not* a fresh call, and so must keep behaving the
//! way they always did.

prelude!();

/// A future which is held before it is awaited is still unstarted, so it
/// splices - and the order it is awaited in is the order it runs in, which is
/// not the order the calls were written in.
#[test]
fn futures_are_held_until_awaited() {
    let out: i64 = rune! {
        async fn inc(a) { a + 1 }

        let f = inc(1);
        let g = inc(10);
        g.await * 10 + f.await
    };

    assert_eq!(out, 112);
}

/// An async closure is reached through a function pointer, which builds its
/// machine on the way in rather than at the call site.
#[test]
fn async_closures_await() {
    let out: i64 = rune! {
        let f = async |a| { a * 2 };
        f(21).await
    };

    assert_eq!(out, 42);
}

/// An async block is a future which was never a call, so there is nothing to
/// splice and it is polled the way it always was.
#[test]
fn async_blocks_await() {
    let out: i64 = rune! {
        let a = 7;
        let f = async { a };
        f.await
    };

    assert_eq!(out, 7);
}

/// `select` polls its branches itself, so a branch is started before anything
/// awaits it and cannot be taken over.
#[test]
fn select_drives_its_own_branches() {
    let out: i64 = rune! {
        async fn inc(a) { a + 1 }

        let a = inc(1);
        let b = inc(2);

        let v = select {
            v = a => v,
            v = b => v,
        };

        // Either branch may win, both of which are correct.
        if v == 2 || v == 3 { 1 } else { 0 }
    };

    assert_eq!(out, 1);
}

/// A future which is dropped without ever being awaited still holds the values
/// its call was given, and they are taken apart rather than left in place.
#[test]
fn futures_dropped_without_being_awaited() {
    let out: i64 = rune! {
        async fn inc(a) { a + 1 }

        let f = inc(1);
        5
    };

    assert_eq!(out, 5);
}

/// A value produced inside a spliced frame reaches the caller the same way a
/// value returned from an ordinary call does, `?` included.
#[test]
fn values_return_out_of_a_spliced_frame() {
    let out: i64 = rune! {
        async fn inner() { Ok(41) }

        async fn outer() {
            let v = inner().await?;
            Ok(v + 1)
        }

        match outer().await {
            Ok(v) => v,
            Err(..) => -1,
        }
    };

    assert_eq!(out, 42);
}

/// An error raised inside a spliced frame unwinds to whoever awaited it rather
/// than being trapped in a machine of its own.
#[test]
fn errors_unwind_out_of_a_spliced_frame() {
    assert_vm_error!(
        r#"
        async fn inner(a) { a / 0 }
        async fn outer(a) { inner(a).await }
        let v = outer(1).await;
        v
        "#,
        VmErrorKind::DivideByZero => {}
    );
}

/// A spliced frame which suspends on something the machine cannot take over
/// leaves the caller suspended too, and resuming runs both back out.
#[test]
fn a_spliced_frame_can_still_suspend() {
    let out: i64 = rune! {
        async fn leaf(a) {
            // An async block is not a call, so awaiting it is a real suspend
            // inside a frame which was spliced in.
            let f = async { a + 1 };
            f.await
        }

        async fn middle(a) { leaf(a).await * 2 }

        middle(20).await
    };

    assert_eq!(out, 42);
}
