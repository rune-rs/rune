prelude!();

#[test]
fn test_closure_in_lit_vec() -> VmResult<()> {
    let ret: VecTuple<(i64, Function, Function, i64)> = eval(r#"let a = 4; [0, || 2, || 4, 3]"#);

    let (start, first, second, end) = ret.0;
    assert_eq!(0, start);
    assert_eq!(2, vm_try!(first.call::<i64>(())));
    assert_eq!(4, vm_try!(second.call::<i64>(())));
    assert_eq!(3, end);
    VmResult::Ok(())
}

#[test]
fn test_closure_in_lit_tuple() -> VmResult<()> {
    let ret: (i64, Function, Function, i64) = eval(r#"let a = 4; (0, || 2, || a, 3)"#);

    let (start, first, second, end) = ret;
    assert_eq!(0, start);
    assert_eq!(2, vm_try!(first.call::<i64>(())));
    assert_eq!(4, vm_try!(second.call::<i64>(())));
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

    let proxy: Proxy = eval("let a = 4; #{a: 0, b: || 2, c: || a, d: 3}");

    assert_eq!(0, proxy.a);
    assert_eq!(2, proxy.b.call::<i64>(())?);
    assert_eq!(4, proxy.c.call::<i64>(())?);
    assert_eq!(3, proxy.d);
    Ok(())
}
