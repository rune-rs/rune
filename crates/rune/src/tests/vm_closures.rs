prelude!();

/// Test that we don't accidentally capture `a` as part of its own declaration.
#[test]
fn test_clobbered_scope() {
    let out: i64 = rune! {
        pub fn main() {
            let a = |b| {
                let a = 11;
                a * b
            };

            a(3)
        }
    };
    assert_eq!(out, 33);
}

/// Tests that delcaring `c` doesn't clobber the declaration and that it is
/// being correctly captured.
#[test]
fn test_self_declaration() {
    let out: i64 = rune! {
        pub fn main() {
            let c = 7;

            let c = |b| {
                let c = c * 11;
                c * b
            };

            c(3)
        }
    };
    assert_eq!(out, 231);
}

#[test]
fn test_nested_closures() {
    let out: i64 = rune! {
        pub fn main() {
            let var = 1;

            let a = |i| {
                let b = |j| {
                    var + j
                };

                b(i + 1)
            };

            a(2)
        }
    };
    assert_eq!(out, 4);
}

#[test]
fn test_closure_in_loop_iter() {
    let out: i64 = rune! {
        pub fn main() {
            let out = 1;

            for var in {
                let a = || [1, 2, 3];
                a()
            } {
                let b = |n| var + n;
                out += b(1);
            }

            out
        }
    };
    assert_eq!(out, 10);
}

#[test]
fn test_capture_match() {
    let out: i64 = rune! {
        pub fn main() {
            let n = 1;

            let a = match { let out = || Some(n + 1); out() } {
                Some(n) => |e| n + e,
                _ => |_| 0,
            };

            a(1)
        }
    };
    assert_eq!(out, 3);
}

#[test]
fn test_capture_fn_arg() {
    let out: i64 = rune! {
        fn foo(n) { |a| n + a }
        pub fn main() { foo(1)(2) }
    };
    assert_eq!(out, 3);

    let out: i64 = rune! {
        fn test(a, b) { b / a + 1 }
        pub fn main() {{let a = || test; a()}({let b = || 2; b()}, {let c = || 6; c()}) }
    };
    assert_eq!(out, 4);

    let out: (i64, i64) = rune! {
        pub fn main() { ({let b = || 2; b()}, {let c = || 6; c()}) }
    };
    assert_eq!(out, (2, 6));

    let out: Vec<i64> = rune! {
        pub fn main() { [{let b = || 2; b()}, {let c = || 6; c()}] }
    };
    assert_eq!(out, vec![2, 6]);
}

#[test]
fn test_capture_and_environ() {
    let out: i64 = rune! {
        async fn foo(cb) {
            cb(1).await
        }

        pub async fn main() {
            let value = 12;
            foo(async |n| n + value).await
        }
    };
    assert_eq!(out, 13);
}

#[test]
fn test_immediate_call() {
    let out: i64 = rune! {
        pub async fn main() {
            let future = (async || {
                11
            })();

            future.await
        }
    };
    assert_eq!(out, 11);
}

#[test]
fn test_nested_async_closure() {
    let out: i64 = rune! {
        async fn send_requests(list) {
            let input = 1;

            let do_request = async |url, n| {
                Ok(input + n)
            };

            for url in list {
                yield do_request(url, 2).await;
            }
        }

        pub async fn main() {
            let requests = send_requests([
                "https://google.com",
                "https://amazon.com",
            ]);

            let output = 0;

            while let Some(input) = requests.next().await {
                output += input?;
            }

            output
        }
    };
    assert_eq!(out, 6);
}

#[test]
fn test_closure_in_lit_vec() -> VmResult<()> {
    let ret: VecTuple<(i64, Function, Function, i64)> = rune_s! {
        r#"pub fn main() { let a = 4; [0, || 2, || 4, 3] }"#
    };

    let (start, first, second, end) = ret.0;
    assert_eq!(0, start);
    assert_eq!(2, vm_try!(first.call::<_, i64>(())));
    assert_eq!(4, vm_try!(second.call::<_, i64>(())));
    assert_eq!(3, end);
    VmResult::Ok(())
}

#[test]
fn test_closure_in_lit_tuple() -> VmResult<()> {
    let ret: (i64, Function, Function, i64) = rune_s! {
        r#"pub fn main() { let a = 4; (0, || 2, || a, 3) }"#
    };

    let (start, first, second, end) = ret;
    assert_eq!(0, start);
    assert_eq!(2, vm_try!(first.call::<_, i64>(())));
    assert_eq!(4, vm_try!(second.call::<_, i64>(())));
    assert_eq!(3, end);
    VmResult::Ok(())
}

#[test]
fn test_closure_in_lit_object() -> Result<()> {
    #[derive(FromValue)]
    struct Proxy {
        a: i64,
        b: Function,
        c: Function,
        d: i64,
    }

    let proxy: Proxy = rune_s! {
        r#"pub fn main() { let a = 4; #{a: 0, b: || 2, c: || a, d: 3} }"#
    };

    assert_eq!(0, proxy.a);
    assert_eq!(2, proxy.b.call::<_, i64>(()).into_result()?);
    assert_eq!(4, proxy.c.call::<_, i64>(()).into_result()?);
    assert_eq!(3, proxy.d);
    Ok(())
}
