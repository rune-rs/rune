prelude!();

#[test]
fn test_assign_assign_exprs() {
    let out: (i64, (), ()) = eval(
        r#"
        let a = #{b: #{c: #{d: 1}}};
        let b = 2;
        let c = 3;

        c = b = a.b.c = 4;
        (a.b.c, b, c)
        "#,
    );
    assert_eq!(out, (4, (), ()));
}
