prelude!();

#[test]
fn test_result() {
    let out: i64 = rune!(match Err("err") {
        Err("err") => 1,
        _ => 2,
    });
    assert_eq!(out, 1);

    let out: i64 = rune!(match Err("err") {
        Ok("ok") => 1,
        _ => 2,
    });
    assert_eq!(out, 2);

    let out: i64 = rune!(match Ok("ok") {
        Ok("ok") => 1,
        _ => 2,
    });
    assert_eq!(out, 1);
}
